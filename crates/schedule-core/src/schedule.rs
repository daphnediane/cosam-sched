/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`Schedule`] — top-level coordination container.
//!
//! Holds all entity storage, relationship edges, and schedule metadata.
//! Fully generic: no entity-type imports here; all typed wiring lives in
//! entity modules.

use crate::crdt::{self, CrdtError};
use crate::edge_cache::HomoEdgeCache;
use crate::edge_map::RawEdgeMap;
use crate::entity::{registered_entity_types, EntityId, EntityType, RuntimeEntityId};
use crate::field::ReadableField;
use crate::value::{CrdtFieldType, FieldError, FieldValue};
use automerge::AutoCommit;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use thiserror::Error;
use uuid::NonNilUuid;

/// Errors returned by [`Schedule::load`] and [`Schedule::load_from_file`].
#[derive(Debug, Error)]
pub enum LoadError {
    /// The file header is missing, corrupted, or an unsupported format version.
    ///
    /// Only returned by [`Schedule::load_from_file`].
    #[error("invalid file format: {0}")]
    Format(String),

    /// The automerge byte blob could not be decoded.
    #[error("failed to decode automerge document: {0}")]
    Codec(String),

    /// Rebuilding a specific entity from the document failed — most commonly
    /// because a required field is missing after a schema migration.
    #[error("failed to rehydrate {type_name}:{uuid}: {detail}")]
    Rehydrate {
        type_name: &'static str,
        uuid: NonNilUuid,
        detail: String,
    },
}

// ── File format constants ─────────────────────────────────────────────────────

/// Magic bytes at the start of every native schedule file.
const FILE_MAGIC: &[u8; 6] = b"COSAM\x00";

/// Current native file format version.
const FILE_FORMAT_VERSION: u16 = 1;

// ── ScheduleMetadata ──────────────────────────────────────────────────────────

/// Top-level schedule identity and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleMetadata {
    /// Globally unique schedule identity (v7, generated at [`Schedule::new`]).
    pub schedule_id: NonNilUuid,
    /// When this schedule was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Human-readable generator identifier (e.g. `"cosam-convert 0.1"`).
    pub generator: String,
    /// Monotonically increasing edit version counter.
    pub version: u32,
}

// ── Schedule ──────────────────────────────────────────────────────────────────

/// Top-level schedule container.
///
/// - **Entity storage**: `HashMap<TypeId, HashMap<NonNilUuid, Box<dyn Any + Send + Sync>>>` —
///   one inner map per entity type; indexed by `TypeId::of::<E::InternalData>()`.
/// - **Edge storage**: a single [`RawEdgeMap`] for all relationships.
/// - **Metadata**: schedule UUID, timestamps, generator info.
///
/// There is no separate `EntityStorage` struct; storage lives directly here.
/// Generic `get_internal` / `insert` dispatch via `TypeId`.
///
/// ## CRDT source of truth
///
/// The authoritative state of every entity lives in the [`AutoCommit`]
/// document `doc`. The `entities` HashMap is a cache that mirrors the
/// document: every successful field write routes through
/// [`crdt::write_field`] before returning, and [`Self::remove_entity`]
/// soft-deletes via the `__deleted` flag. On `load` / `apply_changes` /
/// `merge` the cache is rebuilt in full from the document (FEATURE-022
/// part 2).
///
/// During load the mirror is disabled via [`Self::mirror_enabled`] so that
/// rehydrating entities does not generate redundant writes against the doc.
pub struct Schedule {
    /// Two-level type-erased entity store (cache mirroring the CRDT doc).
    ///
    /// Outer key: `TypeId::of::<E::InternalData>()`.
    /// Inner key: entity UUID.
    /// Value: `Box<E::InternalData>`.
    entities: HashMap<TypeId, HashMap<NonNilUuid, Box<dyn Any + Send + Sync>>>,

    /// Single unified edge store for all entity relationships.
    edges: RawEdgeMap,

    /// Cache for transitive homogeneous-edge relationships (inclusive groups, members, etc.).
    /// Set to `None` whenever a homogeneous edge is modified; rebuilt lazily per-entry on query.
    /// Heterogeneous-edge mutations do not touch this field since the cache contains no heterogeneous data.
    homo_edge_cache: RefCell<Option<HomoEdgeCache>>,

    /// Schedule identity and provenance.
    pub metadata: ScheduleMetadata,

    /// Authoritative CRDT document. All non-derived field values flow here
    /// first; `entities` then mirrors the post-write state.
    doc: AutoCommit,

    /// When `false`, field writes skip the CRDT mirror. Used during
    /// bulk rehydration from the document (FEATURE-022 part 2) to avoid
    /// re-generating change records for values already in the doc.
    mirror_enabled: bool,
}

impl std::fmt::Debug for Schedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schedule")
            .field("entities", &self.entities)
            .field("edges", &self.edges)
            .field("homo_edge_cache", &self.homo_edge_cache)
            .field("metadata", &self.metadata)
            .field("mirror_enabled", &self.mirror_enabled)
            .finish()
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    /// Create a new, empty schedule with a fresh v7 UUID.
    #[must_use]
    pub fn new() -> Self {
        let raw = uuid::Uuid::now_v7();
        // SAFETY: Uuid::now_v7() is never nil.
        let schedule_id = unsafe { NonNilUuid::new_unchecked(raw) };
        Self {
            entities: HashMap::new(),
            edges: RawEdgeMap::default(),
            homo_edge_cache: RefCell::new(None),
            metadata: ScheduleMetadata {
                schedule_id,
                created_at: chrono::Utc::now(),
                generator: String::new(),
                version: 0,
            },
            doc: AutoCommit::new(),
            mirror_enabled: true,
        }
    }

    // ── CRDT save/load ────────────────────────────────────────────────────────

    /// Serialize the entire authoritative CRDT document to a compact byte
    /// blob suitable for on-disk persistence or transport.
    ///
    /// This is a pure pass-through to [`AutoCommit::save`]; the in-memory
    /// cache contributes nothing — it can be fully rebuilt from the bytes
    /// via [`Self::load`].
    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    /// Decode an automerge document from `bytes` and rebuild a `Schedule`
    /// from it: the HashMap cache is rehydrated by replaying every
    /// non-deleted entity through its registered
    /// [`crate::entity::RegisteredEntityType::rehydrate_fn`].
    ///
    /// Metadata (`schedule_id`, `created_at`, etc.) is re-initialized for
    /// the loading process; only entity field data is round-tripped.
    /// Edge state (`RawEdgeMap`) is *not* rehydrated from the doc in this
    /// phase — see FEATURE-023 for owner-list edge storage.
    ///
    /// # Errors
    /// Returns [`LoadError::Codec`] if the bytes do not parse, or
    /// [`LoadError::Rehydrate`] if any entity fails to rebuild (typically
    /// a missing required field after a migration).
    pub fn load(bytes: &[u8]) -> Result<Self, LoadError> {
        let doc = AutoCommit::load(bytes).map_err(|e| LoadError::Codec(e.to_string()))?;
        let mut sched = Self::new();
        sched.doc = doc;
        sched.rebuild_cache_from_doc()?;
        Ok(sched)
    }

    /// Serialize this schedule to the versioned native file format.
    ///
    /// The format is a binary envelope:
    ///
    /// | Offset   | Width  | Description                              |
    /// |----------|--------|------------------------------------------|
    /// | 0        | 6      | Magic: `COSAM\x00`                       |
    /// | 6        | 2      | Format version: `u16` LE (currently `1`) |
    /// | 8        | 4      | Metadata JSON length: `u32` LE           |
    /// | 12       | N      | Metadata: JSON-encoded [`ScheduleMetadata`] |
    /// | 12+N     | …      | Automerge binary document                |
    ///
    /// Metadata (schedule UUID, creation timestamp, generator, edit version)
    /// is embedded in the envelope so that [`Self::load_from_file`] can
    /// restore it exactly; this is the primary difference from the raw
    /// [`Self::save`] / [`Self::load`] pair used for CRDT sync.
    ///
    /// # Panics
    /// Panics if `ScheduleMetadata` cannot be serialized to JSON (this cannot
    /// happen in practice — all field types are always serializable).
    pub fn save_to_file(&mut self) -> Vec<u8> {
        let meta_json = serde_json::to_vec(&self.metadata)
            .expect("ScheduleMetadata serialization is infallible");
        let automerge_bytes = self.doc.save();

        let meta_len = u32::try_from(meta_json.len()).expect("metadata JSON exceeds 4 GiB");
        let mut out =
            Vec::with_capacity(FILE_MAGIC.len() + 2 + 4 + meta_json.len() + automerge_bytes.len());
        out.extend_from_slice(FILE_MAGIC);
        out.extend_from_slice(&FILE_FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&meta_len.to_le_bytes());
        out.extend_from_slice(&meta_json);
        out.extend_from_slice(&automerge_bytes);
        out
    }

    /// Decode a schedule from the native file format, restoring both entity
    /// data (including CRDT history) and schedule metadata.
    ///
    /// This is the counterpart to [`Self::save_to_file`].  Use
    /// [`Self::load`] instead when you have raw automerge bytes from a
    /// sync operation (no metadata envelope).
    ///
    /// # Errors
    /// - [`LoadError::Format`] — bad magic, unsupported version, or
    ///   truncated / invalid metadata JSON.
    /// - [`LoadError::Codec`] — the embedded automerge document cannot be
    ///   decoded.
    /// - [`LoadError::Rehydrate`] — an entity failed to rebuild from the
    ///   document (typically a missing required field after migration).
    pub fn load_from_file(bytes: &[u8]) -> Result<Self, LoadError> {
        const HEADER_SIZE: usize = 6 + 2 + 4; // magic + version + meta_len

        if bytes.len() < HEADER_SIZE {
            return Err(LoadError::Format(
                "file too short to contain a valid header".into(),
            ));
        }
        if &bytes[..6] != FILE_MAGIC {
            return Err(LoadError::Format(
                "unrecognized file magic — not a cosam schedule file".into(),
            ));
        }
        let version = u16::from_le_bytes([bytes[6], bytes[7]]);
        if version != FILE_FORMAT_VERSION {
            return Err(LoadError::Format(format!(
                "unsupported format version {version} (this build supports version {FILE_FORMAT_VERSION})"
            )));
        }
        let meta_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let meta_end = HEADER_SIZE + meta_len;
        if bytes.len() < meta_end {
            return Err(LoadError::Format("metadata section is truncated".into()));
        }
        let metadata: ScheduleMetadata = serde_json::from_slice(&bytes[HEADER_SIZE..meta_end])
            .map_err(|e| LoadError::Format(format!("invalid metadata JSON: {e}")))?;

        let doc =
            AutoCommit::load(&bytes[meta_end..]).map_err(|e| LoadError::Codec(e.to_string()))?;

        let mut sched = Self::new();
        sched.doc = doc;
        sched.metadata = metadata;
        sched.rebuild_cache_from_doc()?;
        Ok(sched)
    }

    /// Apply a batch of encoded automerge changes, then rebuild the
    /// in-memory cache in full so it reflects the post-merge state.
    ///
    /// Typical usage is receiving sync bytes from a peer's
    /// [`Self::get_changes_since`] and calling `apply_changes` to integrate
    /// them into the local replica.
    ///
    /// # Errors
    /// Returns [`LoadError::Codec`] if any byte slice fails to decode as an
    /// automerge `Change`, or [`LoadError::Rehydrate`] if the post-merge
    /// cache rebuild cannot recover an entity (e.g. a required field went
    /// missing under concurrent deletes).
    pub fn apply_changes(&mut self, changes: &[Vec<u8>]) -> Result<(), LoadError> {
        let mut decoded: Vec<automerge::Change> = Vec::with_capacity(changes.len());
        for bytes in changes {
            decoded.push(
                automerge::Change::try_from(bytes.as_slice())
                    .map_err(|e| LoadError::Codec(e.to_string()))?,
            );
        }
        self.doc
            .apply_changes(decoded)
            .map_err(|e| LoadError::Codec(e.to_string()))?;
        self.rebuild_cache_from_doc()
    }

    /// The set of change hashes identifying the current head(s) of the CRDT
    /// document.
    ///
    /// Takes `&mut self` because [`AutoCommit`] flushes pending ops before
    /// reporting heads.  Callers pass the returned slice back later to
    /// [`Self::get_changes_since`] to ask "what have you observed since
    /// this snapshot?" for delta sync.
    pub fn get_heads(&mut self) -> Vec<automerge::ChangeHash> {
        self.doc.get_heads()
    }

    /// Encode every change in the doc's history as bytes.  Useful for a
    /// fresh replica bootstrap (equivalent to `save()`, but without the
    /// compressed document-level framing).
    pub fn get_changes(&mut self) -> Vec<Vec<u8>> {
        self.doc
            .get_changes(&[])
            .into_iter()
            .map(|c| c.raw_bytes().to_vec())
            .collect()
    }

    /// Encode every change the doc has observed that is *not* reachable
    /// from `have_deps`.  Used by sync-pull: the requester sends its
    /// [`Self::get_heads`] and the responder returns the delta.
    pub fn get_changes_since(&mut self, have_deps: &[automerge::ChangeHash]) -> Vec<Vec<u8>> {
        self.doc
            .get_changes(have_deps)
            .into_iter()
            .map(|c| c.raw_bytes().to_vec())
            .collect()
    }

    /// Surface every concurrent value for a scalar field on `id`.
    ///
    /// - Returns an empty vec when the field is unset.
    /// - Returns a single-element vec when there is no conflict — the same
    ///   value as `read_field_value` would observe.
    /// - Returns **all** concurrent writers' values when two or more
    ///   replicas wrote different scalars without either observing the
    ///   other; the primary read (via `field_set`) continues to return
    ///   automerge's deterministically-selected LWW winner.
    ///
    /// Only scalar fields are supported; derived, text, and list fields
    /// yield an empty vec (they have their own per-character or
    /// per-item conflict semantics).
    #[must_use]
    pub fn conflicts_for<E: EntityType>(
        &self,
        id: EntityId<E>,
        field_name: &'static str,
    ) -> Vec<FieldValue> {
        use automerge::{ReadDoc, Value};

        let Some(desc) = E::field_set().get_by_name(field_name) else {
            return Vec::new();
        };
        if !matches!(desc.crdt_type, CrdtFieldType::Scalar) {
            return Vec::new();
        }
        let Some(entity_map) = crdt::get_entity_map(&self.doc, E::TYPE_NAME, id.non_nil_uuid())
        else {
            return Vec::new();
        };
        let Ok(values) = self.doc.get_all(&entity_map, desc.name) else {
            return Vec::new();
        };
        let item_type = desc.field_type.item_type();
        values
            .into_iter()
            .filter_map(|(value, _obj_id)| match value {
                Value::Scalar(sv) => crdt::scalar_to_item(&sv, item_type)
                    .ok()
                    .map(FieldValue::Single),
                _ => None,
            })
            .collect()
    }

    /// Merge `other`'s automerge document into this one and rebuild the
    /// cache to the unified state.  Both replicas remain usable — this is
    /// a symmetric join, not a move.
    ///
    /// # Errors
    /// Returns [`LoadError::Codec`] from automerge merge, or
    /// [`LoadError::Rehydrate`] from the post-merge cache rebuild.
    pub fn merge(&mut self, other: &mut Self) -> Result<(), LoadError> {
        self.doc
            .merge(&mut other.doc)
            .map_err(|e| LoadError::Codec(e.to_string()))?;
        self.rebuild_cache_from_doc()
    }

    /// Discard the in-memory cache and fully reconstitute it from the
    /// current CRDT document.  Used by `load` / `apply_changes` / `merge`.
    ///
    /// Runs under [`Self::with_mirror_disabled`] so replayed entity and
    /// edge writes don't emit redundant changes against the doc we just
    /// read from.
    fn rebuild_cache_from_doc(&mut self) -> Result<(), LoadError> {
        // Wipe the cache — merge can resurrect soft-deleted uuids (add-wins
        // against a delete), retarget edges, and generally change which
        // entities exist.  Rebuilding from scratch is simpler and cheaper
        // than reconciling entry-by-entry.
        self.entities.clear();
        self.edges = RawEdgeMap::default();

        // Snapshot (type_name, rehydrate_fn, uuids) under an immutable
        // borrow of the doc, then apply each rehydrate with the mirror
        // disabled.  Collecting up-front avoids reborrowing the inventory
        // iterator while we mutate `self`.
        struct RehydrateWork {
            type_name: &'static str,
            rehydrate_fn:
                fn(&mut Schedule, NonNilUuid) -> Result<NonNilUuid, crate::builder::BuildError>,
            uuids: Vec<NonNilUuid>,
        }
        let mut work: Vec<RehydrateWork> = Vec::new();
        for reg in registered_entity_types() {
            let uuids: Vec<NonNilUuid> = crdt::list_all_uuids(&self.doc, reg.type_name)
                .into_iter()
                .filter(|u| !crdt::is_deleted(&self.doc, reg.type_name, *u))
                .collect();
            if !uuids.is_empty() {
                work.push(RehydrateWork {
                    type_name: reg.type_name,
                    rehydrate_fn: reg.rehydrate_fn,
                    uuids,
                });
            }
        }

        self.with_mirror_disabled(|s| {
            for item in work {
                for uuid in item.uuids {
                    (item.rehydrate_fn)(s, uuid).map_err(|e| LoadError::Rehydrate {
                        type_name: item.type_name,
                        uuid,
                        detail: e.to_string(),
                    })?;
                }
            }
            s.rebuild_edges_from_doc();
            Ok(())
        })
    }

    /// Replay every canonical owner's relationship-list field into the
    /// in-memory [`RawEdgeMap`].
    ///
    /// For every [`crate::edge_descriptor::EdgeDescriptor`] in
    /// [`crate::edge_descriptor::ALL_EDGE_DESCRIPTORS`], iterate every live owner
    /// uuid in the doc, read the list, and `add_het`/`add_homo` each
    /// endpoint into the cache.  The caller is responsible for running this
    /// under [`Self::with_mirror_disabled`] — otherwise each replayed edge
    /// would re-write the same list back into the doc.
    fn rebuild_edges_from_doc(&mut self) {
        use crate::value::FieldTypeItem;

        // Snapshot the `(owner_uuid, target_uuids)` pairs while borrowing
        // `&self.doc`, then apply them under `&mut self`.
        struct EdgeBatch {
            owner_type: &'static str,
            target_type: &'static str,
            is_homogeneous: bool,
            pairs: Vec<(NonNilUuid, Vec<NonNilUuid>)>,
        }
        let mut batches: Vec<EdgeBatch> = Vec::new();
        for desc in crate::edge_descriptor::ALL_EDGE_DESCRIPTORS {
            let (owner_type, field_name, target_type, is_homogeneous) = (
                desc.owner_type,
                desc.field_name,
                desc.target_type,
                desc.is_homogeneous,
            );
            let owner_uuids = crdt::list_all_uuids(&self.doc, owner_type);
            let mut pairs: Vec<(NonNilUuid, Vec<NonNilUuid>)> = Vec::new();
            for owner_uuid in owner_uuids {
                if crdt::is_deleted(&self.doc, owner_type, owner_uuid) {
                    continue;
                }
                let targets = crate::edge_crdt::read_owner_list(
                    &self.doc,
                    owner_type,
                    owner_uuid,
                    field_name,
                    FieldTypeItem::EntityIdentifier(target_type),
                );
                if !targets.is_empty() {
                    pairs.push((owner_uuid, targets));
                }
            }
            if !pairs.is_empty() {
                batches.push(EdgeBatch {
                    owner_type,
                    target_type,
                    is_homogeneous,
                    pairs,
                });
            }
        }
        for batch in batches {
            for (owner_uuid, targets) in batch.pairs {
                // SAFETY: the uuid/type pair was just read out of the
                // authoritative doc under the same owner entity map.
                let owner_rid = unsafe { RuntimeEntityId::from_uuid(owner_uuid, batch.owner_type) };
                for target_uuid in targets {
                    // SAFETY: same justification — the list was stored as
                    // EntityIdentifier scalars tagged with `batch.target_type`.
                    let target_rid =
                        unsafe { RuntimeEntityId::from_uuid(target_uuid, batch.target_type) };
                    if batch.is_homogeneous {
                        self.edges.add_homo(owner_rid, target_rid);
                    } else {
                        self.edges.add_het(owner_rid, target_rid);
                    }
                }
            }
        }
    }

    // ── CRDT access ───────────────────────────────────────────────────────────

    /// Borrow the underlying CRDT document (for change-tracking / save).
    #[must_use]
    pub fn doc(&self) -> &AutoCommit {
        &self.doc
    }

    /// Mutable access to the CRDT document — restricted to crate-internal
    /// helpers (edit commands, edge mirroring, load path).
    #[allow(dead_code)] // wired in FEATURE-022 part 2 (load/save) and FEATURE-023
    pub(crate) fn doc_mut(&mut self) -> &mut AutoCommit {
        &mut self.doc
    }

    /// Whether field writes currently mirror to the CRDT document.
    #[must_use]
    pub fn mirror_enabled(&self) -> bool {
        self.mirror_enabled
    }

    /// Run `f` with the CRDT mirror temporarily disabled. Used by the load
    /// path to rehydrate the cache without re-emitting CRDT operations.
    pub(crate) fn with_mirror_disabled<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = self.mirror_enabled;
        self.mirror_enabled = false;
        let out = f(self);
        self.mirror_enabled = prev;
        out
    }

    /// Mirror every non-derived field of entity `id` into the CRDT document.
    ///
    /// Called by [`Self::insert`] immediately after the cache is populated.
    /// No-op when [`Self::mirror_enabled`] is false.
    pub(crate) fn mirror_entity_fields<E: EntityType>(
        &mut self,
        id: EntityId<E>,
    ) -> Result<(), CrdtError> {
        if !self.mirror_enabled {
            return Ok(());
        }
        let uuid = id.non_nil_uuid();
        let type_name = E::TYPE_NAME;
        crdt::touch_entity(&mut self.doc, type_name, uuid)?;
        // Collect (name, crdt_type, value) while holding `&self`, then apply
        // writes while holding `&mut self.doc`.
        let mut pending: Vec<(&'static str, CrdtFieldType, FieldValue)> = Vec::new();
        for desc in E::field_set().fields() {
            if matches!(desc.crdt_type, CrdtFieldType::Derived) {
                continue;
            }
            if let Ok(Some(v)) = desc.read(id, self) {
                pending.push((desc.name, desc.crdt_type, v));
            }
        }
        for (name, crdt_type, v) in pending {
            crdt::write_field(&mut self.doc, type_name, uuid, name, crdt_type, &v)?;
        }
        // Ensure the entity is not marked deleted (idempotent on re-insert).
        crdt::put_deleted(&mut self.doc, type_name, uuid, false)?;
        // Pre-create every canonical owner-list field on this entity so
        // concurrent replicas share the same list `ObjId` and
        // `edge_add`/`edge_remove` can converge via incremental
        // `insert`/`delete` (FEATURE-023).
        crate::edge_crdt::ensure_all_owner_lists_for_type(&mut self.doc, type_name, uuid)?;
        Ok(())
    }

    /// Push a single post-write field value into the CRDT document.
    ///
    /// Invoked from [`crate::field::FieldDescriptor::write`] after the inner
    /// write succeeds.  `None` clears the field from the doc (corresponds to
    /// an unset optional field).
    ///
    /// # Errors
    /// Returns [`FieldError::Crdt`] if the mirror write fails.
    pub(crate) fn mirror_field_value<E: EntityType>(
        &mut self,
        id: EntityId<E>,
        name: &'static str,
        crdt_type: CrdtFieldType,
        value: Option<&FieldValue>,
    ) -> Result<(), FieldError> {
        if !self.mirror_enabled || matches!(crdt_type, CrdtFieldType::Derived) {
            return Ok(());
        }
        let uuid = id.non_nil_uuid();
        let type_name = E::TYPE_NAME;
        let res = match value {
            Some(v) => crdt::write_field(&mut self.doc, type_name, uuid, name, crdt_type, v),
            None => crdt::clear_field(&mut self.doc, type_name, uuid, name),
        };
        res.map_err(|e| FieldError::Crdt {
            name,
            detail: e.to_string(),
        })
    }

    // ── Entity storage ────────────────────────────────────────────────────────

    /// Retrieve a shared reference to an entity's internal data.
    ///
    /// Returns `None` if the entity is not present.
    #[must_use]
    pub fn get_internal<E: EntityType>(&self, id: EntityId<E>) -> Option<&E::InternalData> {
        self.entities
            .get(&TypeId::of::<E::InternalData>())?
            .get(&id.non_nil_uuid())?
            .downcast_ref::<E::InternalData>()
    }

    /// Retrieve a mutable reference to an entity's internal data.
    ///
    /// Returns `None` if the entity is not present.
    pub fn get_internal_mut<E: EntityType>(
        &mut self,
        id: EntityId<E>,
    ) -> Option<&mut E::InternalData> {
        self.entities
            .get_mut(&TypeId::of::<E::InternalData>())?
            .get_mut(&id.non_nil_uuid())?
            .downcast_mut::<E::InternalData>()
    }

    /// Insert or replace an entity's internal data.
    ///
    /// Populates the in-memory cache and then mirrors every non-derived field
    /// into the authoritative CRDT document.  Any CRDT mirror error is logged
    /// and otherwise silently tolerated — the cache state is kept as primary
    /// for the current call; a subsequent field write will retry the mirror.
    /// (Mirror failures are only possible on malformed field values today,
    /// and those would have failed validation at build time.)
    pub fn insert<E: EntityType>(&mut self, id: EntityId<E>, data: E::InternalData) {
        self.entities
            .entry(TypeId::of::<E::InternalData>())
            .or_default()
            .insert(id.non_nil_uuid(), Box::new(data));
        if let Err(e) = self.mirror_entity_fields(id) {
            // Mirror should only fail on genuinely malformed data; surface
            // loudly in debug to catch regressions without panicking in
            // release builds.
            debug_assert!(false, "CRDT mirror failed on insert: {e}");
            let _ = e;
        }
    }

    /// Remove an entity and clear all of its edge relationships.
    ///
    /// The CRDT document retains the entity's field history and marks it
    /// `__deleted = true`; the in-memory cache is evicted so queries no
    /// longer see it.  Concurrent replicas that still have the pre-delete
    /// version can merge their edits back in, which is the point of the
    /// soft-delete scheme.
    pub fn remove_entity<E: EntityType>(&mut self, id: EntityId<E>) {
        let uuid = id.non_nil_uuid();
        if self.mirror_enabled {
            if let Err(e) = crdt::put_deleted(&mut self.doc, E::TYPE_NAME, uuid, true) {
                debug_assert!(false, "CRDT soft-delete failed: {e}");
                let _ = e;
            }
        }
        if let Some(map) = self.entities.get_mut(&TypeId::of::<E::InternalData>()) {
            map.remove(&uuid);
        }
        self.edges.clear_all(uuid, E::TYPE_NAME);
        *self.homo_edge_cache.borrow_mut() = None;
    }

    /// Iterate all entities of type `E`, yielding `(EntityId<E>, &E::InternalData)` pairs.
    pub fn iter_entities<E: EntityType>(
        &self,
    ) -> impl Iterator<Item = (EntityId<E>, &E::InternalData)> {
        self.entities
            .get(&TypeId::of::<E::InternalData>())
            .into_iter()
            .flat_map(|map| map.iter())
            .filter_map(|(uuid, boxed)| {
                let data = boxed.downcast_ref::<E::InternalData>()?;
                // SAFETY: uuid came from inserting an EntityId<E>, so it belongs to E.
                let id = unsafe { EntityId::from_uuid(*uuid) };
                Some((id, data))
            })
    }

    /// Count entities of type `E` currently in the schedule.
    #[must_use]
    pub fn entity_count<E: EntityType>(&self) -> usize {
        self.entities
            .get(&TypeId::of::<E::InternalData>())
            .map_or(0, HashMap::len)
    }

    /// Identify which entity type a bare UUID belongs to.
    ///
    /// Queries all inventory-registered entity types (O(5) inner-map lookups).
    /// Returns `None` if the UUID is not found in any type's storage.
    #[must_use]
    pub fn identify(&self, uuid: NonNilUuid) -> Option<RuntimeEntityId> {
        registered_entity_types().find_map(|reg| {
            let inner = self.entities.get(&(reg.type_id)())?;
            if inner.contains_key(&uuid) {
                // SAFETY: we just confirmed uuid is in the inner map for reg.type_name.
                Some(unsafe { RuntimeEntityId::from_uuid(uuid, reg.type_name) })
            } else {
                None
            }
        })
    }

    // ── Edge API ──────────────────────────────────────────────────────────────

    /// All `R` entities reachable from `id` following the L→R direction.
    ///
    /// For heterogeneous edges: reads `edges[id]` filtered by `R::TYPE_NAME`.
    /// For homogeneous edges (L==R): same — forward edges are stored in `edges`.
    #[must_use]
    pub fn edges_from<L: EntityType, R: EntityType>(&self, id: EntityId<L>) -> Vec<EntityId<R>> {
        self.edges
            .neighbors(id.non_nil_uuid())
            .iter()
            .filter_map(|rid| rid.try_as_typed::<R>())
            .collect()
    }

    /// All `L` entities that have an edge pointing to `id`.
    ///
    /// For heterogeneous edges: reads `edges[id]` filtered by `L::TYPE_NAME`.
    /// For homogeneous edges (L==R): reads `homogeneous_reverse[id]` filtered by `L::TYPE_NAME`.
    #[must_use]
    pub fn edges_to<L: EntityType, R: EntityType>(&self, id: EntityId<R>) -> Vec<EntityId<L>> {
        let is_homogeneous = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        let source = if is_homogeneous {
            self.edges.homo_reverse(id.non_nil_uuid())
        } else {
            self.edges.neighbors(id.non_nil_uuid())
        };
        source
            .iter()
            .filter_map(|rid| rid.try_as_typed::<L>())
            .collect()
    }

    /// All `R` entities transitively reachable from `id` via homogeneous forward edges.
    ///
    /// For homogeneous edges (L==R): follows forward edges transitively
    /// (e.g. `inclusive_edges_from<Presenter, Presenter>(alice)` returns all groups
    /// alice belongs to, transitively — not alice herself).
    /// For heterogeneous edges (L≠R): falls back to direct `edges_from` (single hop only).
    ///
    /// Takes `&self`; the edge cache is updated through interior mutability.
    #[must_use]
    pub fn inclusive_edges_from<L: EntityType, R: EntityType>(
        &self,
        id: EntityId<L>,
    ) -> Vec<EntityId<R>> {
        let is_homogeneous = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        if is_homogeneous {
            let uuids = {
                let mut cache_opt = self.homo_edge_cache.borrow_mut();
                let cache = cache_opt.get_or_insert_with(HomoEdgeCache::default);
                cache.get_or_compute_forward(&self.edges, id.non_nil_uuid(), R::TYPE_NAME)
            };
            uuids
                .into_iter()
                // SAFETY: uuid came from the edge map which only stores valid entity IDs of type R.
                .map(|uuid| unsafe { EntityId::from_uuid(uuid) })
                .collect()
        } else {
            self.edges_from::<L, R>(id)
        }
    }

    /// All `L` entities that transitively point to `id` via homogeneous reverse edges.
    ///
    /// For homogeneous edges (L==R): follows reverse edges transitively
    /// (e.g. `inclusive_edges_to<Presenter, Presenter>(team_a)` returns all members
    /// of team_a transitively — not team_a itself).
    /// For heterogeneous edges (L≠R): falls back to direct `edges_to` (single hop only).
    ///
    /// Takes `&self`; the edge cache is updated through interior mutability.
    #[must_use]
    pub fn inclusive_edges_to<L: EntityType, R: EntityType>(
        &self,
        id: EntityId<R>,
    ) -> Vec<EntityId<L>> {
        let is_homogeneous = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        if is_homogeneous {
            let uuids = {
                let mut cache_opt = self.homo_edge_cache.borrow_mut();
                let cache = cache_opt.get_or_insert_with(HomoEdgeCache::default);
                cache.get_or_compute_reverse(&self.edges, id.non_nil_uuid(), L::TYPE_NAME)
            };
            uuids
                .into_iter()
                // SAFETY: uuid came from the edge map which only stores valid entity IDs of type L.
                .map(|uuid| unsafe { EntityId::from_uuid(uuid) })
                .collect()
        } else {
            self.edges_to::<L, R>(id)
        }
    }

    /// Add an edge from `l` to `r`, using the correct heterogeneous/homogeneous storage strategy.
    ///
    /// After updating the cache, if the mirror is enabled the new endpoint
    /// is incrementally `insert`ed into the canonical owner's list field
    /// (via [`crate::edge_crdt::list_append_unique`]) — **not** rewritten in
    /// full — so concurrent add/add from two replicas converges to the
    /// union rather than LWW on the list object.
    pub fn edge_add<L: EntityType, R: EntityType>(&mut self, l: EntityId<L>, r: EntityId<R>) {
        let is_homogeneous = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        let l_rid = RuntimeEntityId::from_typed(l);
        let r_rid = RuntimeEntityId::from_typed(r);
        if is_homogeneous {
            self.edges.add_homo(l_rid, r_rid);
            *self.homo_edge_cache.borrow_mut() = None;
        } else {
            self.edges.add_het(l_rid, r_rid);
        }
        self.mirror_edge_add::<L, R>(l.non_nil_uuid(), r.non_nil_uuid());
    }

    /// Remove the edge from `l` to `r`.
    ///
    /// The CRDT mirror uses an incremental delete on observed indices so
    /// concurrent add-vs-unobserved-remove resolves add-wins.
    pub fn edge_remove<L: EntityType, R: EntityType>(&mut self, l: EntityId<L>, r: EntityId<R>) {
        let is_homogeneous = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        if is_homogeneous {
            self.edges.remove_homo(l.non_nil_uuid(), r.non_nil_uuid());
            *self.homo_edge_cache.borrow_mut() = None;
        } else {
            self.edges.remove_het(l.non_nil_uuid(), r.non_nil_uuid());
        }
        self.mirror_edge_remove::<L, R>(l.non_nil_uuid(), r.non_nil_uuid());
    }

    /// Replace all R-type neighbors of `l` with `rights`.
    ///
    /// Removes any existing edges from `l` to entities of type `R`, then
    /// adds edges to each entity in `rights`.
    pub fn edge_set<L: EntityType, R: EntityType>(
        &mut self,
        l: EntityId<L>,
        rights: Vec<EntityId<R>>,
    ) {
        let is_homogeneous = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        let l_rid = RuntimeEntityId::from_typed(l);
        let new_targets: Vec<RuntimeEntityId> = rights
            .iter()
            .map(|r| RuntimeEntityId::from_typed(*r))
            .collect();
        self.edges
            .set_neighbors(l_rid, &new_targets, R::TYPE_NAME, is_homogeneous);
        if is_homogeneous {
            *self.homo_edge_cache.borrow_mut() = None;
        }
        // Mirror only the owner side — if `l` is the canonical owner for
        // (L,R), we rewrite l's list; otherwise we rewrite each r's list in
        // turn to reflect the flipped set of l-endpoints.
        self.mirror_edge_set::<L, R>(l.non_nil_uuid(), &rights);
    }

    /// Replace all L-type sources pointing to `r` with `lefts`.
    ///
    /// Used for the reverse (members) direction of homogeneous edges.
    /// Removes each old source's forward edge to `r`, then adds forward edges
    /// from each entity in `lefts` to `r`.
    pub fn edge_set_to<L: EntityType, R: EntityType>(
        &mut self,
        r: EntityId<R>,
        lefts: Vec<EntityId<L>>,
    ) {
        let is_homogeneous = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        let old_lefts = self.edges_to::<L, R>(r);
        let r_rid = RuntimeEntityId::from_typed(r);
        for l in old_lefts.iter().copied() {
            let l_rid = RuntimeEntityId::from_typed(l);
            if is_homogeneous {
                self.edges.remove_homo(l_rid.uuid(), r_rid.uuid());
            } else {
                self.edges.remove_het(l_rid.uuid(), r_rid.uuid());
            }
        }
        for l in lefts.iter().copied() {
            let l_rid = RuntimeEntityId::from_typed(l);
            if is_homogeneous {
                self.edges.add_homo(l_rid, r_rid);
            } else {
                self.edges.add_het(l_rid, r_rid);
            }
        }
        if is_homogeneous {
            *self.homo_edge_cache.borrow_mut() = None;
        }
        // Mirror: treat as a set-difference against `lefts`. Each previously
        // connected `l` not in the new set is `edge_remove`d from the owner
        // list; each new `l` not previously connected is `edge_add`ed.
        let new_uuids: Vec<NonNilUuid> = lefts.iter().map(|l| l.non_nil_uuid()).collect();
        let old_uuids: Vec<NonNilUuid> = old_lefts.iter().map(|l| l.non_nil_uuid()).collect();
        for l_uuid in &old_uuids {
            if !new_uuids.contains(l_uuid) {
                self.mirror_edge_remove::<L, R>(*l_uuid, r.non_nil_uuid());
            }
        }
        for l_uuid in &new_uuids {
            if !old_uuids.contains(l_uuid) {
                self.mirror_edge_add::<L, R>(*l_uuid, r.non_nil_uuid());
            }
        }
    }

    /// Read a boolean per-edge property for the `(l, r)` edge.
    ///
    /// Resolves the canonical owner for `(L, R)`, looks up the `EdgeDescriptor`
    /// to find the named field's declared default, then reads the value from the
    /// `{field_name}_meta` CRDT map.  Returns the declared default when no
    /// explicit value has been written.
    ///
    /// # Panics
    /// Panics in debug builds if `prop` is not declared in the descriptor's
    /// `fields` slice.
    #[must_use]
    pub fn edge_get_bool<L: EntityType, R: EntityType>(
        &self,
        l_id: EntityId<L>,
        r_id: EntityId<R>,
        prop: &str,
    ) -> bool {
        use crate::edge_descriptor::EdgeFieldDefault;
        use crate::edge_descriptor::ALL_EDGE_DESCRIPTORS;
        let Some(canon) = crate::edge_crdt::canonical_owner(L::TYPE_NAME, R::TYPE_NAME) else {
            return true;
        };
        let (owner_uuid, target_uuid) = if canon.owner_is_left {
            (l_id.non_nil_uuid(), r_id.non_nil_uuid())
        } else {
            (r_id.non_nil_uuid(), l_id.non_nil_uuid())
        };
        let default = ALL_EDGE_DESCRIPTORS
            .iter()
            .find(|d| d.owner_type == canon.owner_type && d.field_name == canon.field_name)
            .and_then(|d| d.fields.iter().find(|f| f.name == prop))
            .map(|spec| match spec.default {
                EdgeFieldDefault::Boolean(b) => b,
            });
        debug_assert!(
            default.is_some(),
            "edge_get_bool: prop {prop:?} not declared in EdgeDescriptor for {}",
            canon.field_name
        );
        let default = default.unwrap_or(true);
        crate::edge_crdt::read_edge_meta_bool(
            &self.doc,
            canon.owner_type,
            owner_uuid,
            canon.field_name,
            target_uuid,
            prop,
            default,
        )
    }

    /// Write a boolean per-edge property for the `(l, r)` edge (LWW).
    ///
    /// Resolves the canonical owner for `(L, R)` and writes the value into
    /// the `{field_name}_meta` CRDT map.  Silently no-ops if the pair is not
    /// a recognized relationship.
    pub fn edge_set_bool<L: EntityType, R: EntityType>(
        &mut self,
        l_id: EntityId<L>,
        r_id: EntityId<R>,
        prop: &str,
        value: bool,
    ) {
        let Some(canon) = crate::edge_crdt::canonical_owner(L::TYPE_NAME, R::TYPE_NAME) else {
            return;
        };
        let (owner_uuid, target_uuid) = if canon.owner_is_left {
            (l_id.non_nil_uuid(), r_id.non_nil_uuid())
        } else {
            (r_id.non_nil_uuid(), l_id.non_nil_uuid())
        };
        if let Err(e) = crate::edge_crdt::write_edge_meta_bool(
            &mut self.doc,
            canon.owner_type,
            owner_uuid,
            canon.field_name,
            target_uuid,
            prop,
            value,
        ) {
            debug_assert!(false, "CRDT edge_set_bool failed: {e}");
            let _ = e;
        }
    }

    /// After `edge_add`, incrementally append the new endpoint into the
    /// canonical owner's list field. Concurrent add/add converges to the
    /// union because both replicas insert into the same shared list
    /// [`ObjId`](automerge::ObjId) created up-front by
    /// [`crate::edge_crdt::ensure_all_owner_lists_for_type`].
    fn mirror_edge_add<L: EntityType, R: EntityType>(
        &mut self,
        l_uuid: NonNilUuid,
        r_uuid: NonNilUuid,
    ) {
        if !self.mirror_enabled {
            return;
        }
        let Some(canon) = crate::edge_crdt::canonical_owner(L::TYPE_NAME, R::TYPE_NAME) else {
            return;
        };
        let (owner_uuid, target_uuid) = if canon.owner_is_left {
            (l_uuid, r_uuid)
        } else {
            (r_uuid, l_uuid)
        };
        if let Err(e) = crate::edge_crdt::list_append_unique(
            &mut self.doc,
            canon.owner_type,
            owner_uuid,
            canon.target_type,
            canon.field_name,
            target_uuid,
        ) {
            debug_assert!(false, "CRDT edge_add mirror failed: {e}");
            let _ = e;
        }
    }

    /// After `edge_remove`, incrementally delete every occurrence of the
    /// endpoint from the canonical owner's list.  Concurrent add-vs-
    /// unobserved-remove resolves add-wins: the remove only targets
    /// indices this actor observed, so an insert recorded on a parallel
    /// branch survives the merge.
    fn mirror_edge_remove<L: EntityType, R: EntityType>(
        &mut self,
        l_uuid: NonNilUuid,
        r_uuid: NonNilUuid,
    ) {
        if !self.mirror_enabled {
            return;
        }
        let Some(canon) = crate::edge_crdt::canonical_owner(L::TYPE_NAME, R::TYPE_NAME) else {
            return;
        };
        let (owner_uuid, target_uuid) = if canon.owner_is_left {
            (l_uuid, r_uuid)
        } else {
            (r_uuid, l_uuid)
        };
        if let Err(e) = crate::edge_crdt::list_remove_uuid(
            &mut self.doc,
            canon.owner_type,
            owner_uuid,
            canon.target_type,
            canon.field_name,
            target_uuid,
        ) {
            debug_assert!(false, "CRDT edge_remove mirror failed: {e}");
            let _ = e;
        }
    }

    /// Edge-set variant of [`Self::mirror_edge_change`] — bulk version.
    ///
    /// When `L` is the canonical owner, a single list write on `l` suffices.
    /// When `R` owns, every `r` in `rights` and every previous r that just
    /// lost `l` needs its list re-synced; the simplest correct strategy is
    /// to re-derive from the cache for every currently-in-range `r`.
    fn mirror_edge_set<L: EntityType, R: EntityType>(
        &mut self,
        l_uuid: NonNilUuid,
        rights: &[EntityId<R>],
    ) {
        if !self.mirror_enabled {
            return;
        }
        let Some(canon) = crate::edge_crdt::canonical_owner(L::TYPE_NAME, R::TYPE_NAME) else {
            return;
        };
        if canon.owner_is_left {
            // Single write on l's list.
            let targets: Vec<NonNilUuid> = self
                .edges
                .neighbors(l_uuid)
                .iter()
                .filter(|e| e.type_name() == R::TYPE_NAME)
                .map(|e| e.uuid())
                .collect();
            if let Err(e) = crate::edge_crdt::write_owner_list(
                &mut self.doc,
                canon.owner_type,
                l_uuid,
                canon.target_type,
                canon.field_name,
                &targets,
            ) {
                debug_assert!(false, "CRDT edge mirror failed: {e}");
                let _ = e;
            }
            return;
        }
        // R is owner — rewrite every currently-connected r's list, plus each
        // r in `rights` that may have just gained l.  Walk every r whose
        // cache list presently contains l OR that is in `rights`.
        let mut owners: Vec<NonNilUuid> = rights.iter().map(|r| r.non_nil_uuid()).collect();
        // Find previously-connected r's by scanning the cache's l→r adjacency.
        // After `set_neighbors`, l's neighbor list no longer includes removed
        // r's, so we can't learn removed-r uuids from l alone; a reverse scan
        // of all owner entities is the cheapest correct option.
        let doc_uuids = crdt::list_all_uuids(&self.doc, canon.owner_type);
        for r_uuid in doc_uuids {
            if owners.contains(&r_uuid) {
                continue;
            }
            // Was r previously linked to l?  If its doc list contains l_uuid,
            // we must rewrite it now that the cache no longer does.
            let prev = crate::edge_crdt::read_owner_list(
                &self.doc,
                canon.owner_type,
                r_uuid,
                canon.field_name,
                crate::value::FieldTypeItem::EntityIdentifier(L::TYPE_NAME),
            );
            if prev.contains(&l_uuid) {
                owners.push(r_uuid);
            }
        }
        for owner_uuid in owners {
            let targets: Vec<NonNilUuid> = self
                .edges
                .neighbors(owner_uuid)
                .iter()
                .filter(|e| e.type_name() == L::TYPE_NAME)
                .map(|e| e.uuid())
                .collect();
            if let Err(e) = crate::edge_crdt::write_owner_list(
                &mut self.doc,
                canon.owner_type,
                owner_uuid,
                canon.target_type,
                canon.field_name,
                &targets,
            ) {
                debug_assert!(false, "CRDT edge mirror failed: {e}");
                let _ = e;
            }
        }
    }

    // ── Query ─────────────────────────────────────────────────────────────────
}

// ── Helper: convert Vec<EntityId<E>> to FieldValue ───────────────────────────

/// Convert a `Vec<EntityId<E>>` to a `FieldValue::List` of `EntityIdentifier` items.
///
/// Used by `ReadFn::Schedule` closures in edge-backed field descriptors.
pub fn entity_ids_to_field_value<E: EntityType>(ids: Vec<EntityId<E>>) -> FieldValue {
    use crate::value::FieldValueItem;
    FieldValue::List(
        ids.into_iter()
            .map(|id| FieldValueItem::EntityIdentifier(RuntimeEntityId::from_typed(id)))
            .collect(),
    )
}

/// Parse a `FieldValue` into a `Vec<EntityId<E>>`.
///
/// Accepts `FieldValue::List(...)` of `EntityIdentifier` items; returns
/// `Err(FieldError::Conversion(...))` for any non-matching items or variants.
///
/// Used by `WriteFn::Schedule` closures in edge-backed field descriptors.
pub fn field_value_to_entity_ids<E: EntityType>(
    val: FieldValue,
) -> Result<Vec<EntityId<E>>, crate::value::FieldError> {
    use crate::value::{ConversionError, FieldValueItem};
    match val {
        FieldValue::List(items) => items
            .into_iter()
            .map(|item| match item {
                FieldValueItem::EntityIdentifier(rid) => {
                    rid.try_as_typed::<E>()
                        .ok_or(crate::value::FieldError::Conversion(
                            ConversionError::WrongVariant {
                                expected: E::TYPE_NAME,
                                got: "other entity type",
                            },
                        ))
                }
                _ => Err(crate::value::FieldError::Conversion(
                    ConversionError::WrongVariant {
                        expected: "EntityIdentifier",
                        got: "other",
                    },
                )),
            })
            .collect(),
        _ => Err(crate::value::FieldError::Conversion(
            ConversionError::WrongVariant {
                expected: "List",
                got: "other",
            },
        )),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityId, UuidPreference};
    use crate::event_room::{EventRoomCommonData, EventRoomEntityType, EventRoomInternalData};
    use crate::hotel_room::{HotelRoomCommonData, HotelRoomEntityType, HotelRoomInternalData};
    use crate::panel::{PanelCommonData, PanelEntityType, PanelId, PanelInternalData};
    use crate::panel_type::{PanelTypeCommonData, PanelTypeEntityType, PanelTypeInternalData};
    use crate::panel_uniq_id::PanelUniqId;
    use crate::presenter::{
        PresenterCommonData, PresenterEntityType, PresenterId, PresenterInternalData,
    };
    use crate::time::TimeRange;

    fn make_panel_type() -> (EntityId<PanelTypeEntityType>, PanelTypeInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = PanelTypeInternalData {
            id,
            data: PanelTypeCommonData {
                prefix: "GP".into(),
                panel_kind: "Guest Panel".into(),
                ..Default::default()
            },
        };
        (id, data)
    }

    fn make_panel() -> (PanelId, PanelInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = PanelInternalData {
            id,
            data: PanelCommonData {
                name: "Test Panel".into(),
                ..Default::default()
            },
            code: PanelUniqId::parse("GP001").unwrap(),
            time_slot: TimeRange::Unspecified,
        };
        (id, data)
    }

    fn make_presenter(name: &str) -> (EntityId<PresenterEntityType>, PresenterInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = PresenterInternalData {
            id,
            data: PresenterCommonData {
                name: name.into(),
                ..Default::default()
            },
        };
        (id, data)
    }

    fn make_event_room(name: &str) -> (EntityId<EventRoomEntityType>, EventRoomInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = EventRoomInternalData {
            id,
            data: EventRoomCommonData {
                room_name: name.into(),
                ..Default::default()
            },
        };
        (id, data)
    }

    fn make_hotel_room(name: &str) -> (EntityId<HotelRoomEntityType>, HotelRoomInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = HotelRoomInternalData {
            id,
            data: HotelRoomCommonData {
                hotel_room_name: name.into(),
            },
        };
        (id, data)
    }

    // ── Entity storage ────────────────────────────────────────────────────────

    #[test]
    fn insert_and_get_internal() {
        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data.clone());
        let got = sched.get_internal(id).unwrap();
        assert_eq!(got.data.prefix, "GP");
    }

    #[test]
    fn get_internal_missing_returns_none() {
        let sched = Schedule::new();
        let (id, _) = make_panel_type();
        assert!(sched.get_internal(id).is_none());
    }

    #[test]
    fn insert_replaces_existing() {
        let mut sched = Schedule::new();
        let (id, mut data) = make_panel_type();
        sched.insert(id, data.clone());
        data.data.prefix = "SP".into();
        sched.insert(id, data);
        assert_eq!(sched.get_internal(id).unwrap().data.prefix, "SP");
    }

    #[test]
    fn entity_count() {
        let mut sched = Schedule::new();
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
        let (id1, d1) = make_panel_type();
        let (id2, d2) = make_panel_type();
        sched.insert(id1, d1);
        sched.insert(id2, d2);
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 2);
    }

    #[test]
    fn iter_entities() {
        let mut sched = Schedule::new();
        let (id1, d1) = make_panel_type();
        let (id2, d2) = make_panel_type();
        sched.insert(id1, d1);
        sched.insert(id2, d2);
        let ids: std::collections::HashSet<_> = sched
            .iter_entities::<PanelTypeEntityType>()
            .map(|(id, _)| id)
            .collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn remove_entity_removes_from_storage() {
        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data);
        assert!(sched.get_internal(id).is_some());
        sched.remove_entity::<PanelTypeEntityType>(id);
        assert!(sched.get_internal(id).is_none());
    }

    // ── Identify ──────────────────────────────────────────────────────────────

    #[test]
    fn identify_returns_correct_type() {
        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data);
        let rid = sched.identify(id.non_nil_uuid()).unwrap();
        assert_eq!(rid.type_name(), "panel_type");
        assert_eq!(rid.uuid(), id.non_nil_uuid());
    }

    #[test]
    fn identify_missing_uuid_returns_none() {
        let sched = Schedule::new();
        let (id, _) = make_panel_type();
        assert!(sched.identify(id.non_nil_uuid()).is_none());
    }

    #[test]
    fn identify_distinguishes_types() {
        let mut sched = Schedule::new();
        let (pt_id, pt_data) = make_panel_type();
        let (p_id, p_data) = make_presenter("Alice");
        sched.insert(pt_id, pt_data);
        sched.insert(p_id, p_data);
        let pt_rid = sched.identify(pt_id.non_nil_uuid()).unwrap();
        let p_rid = sched.identify(p_id.non_nil_uuid()).unwrap();
        assert_eq!(pt_rid.type_name(), "panel_type");
        assert_eq!(p_rid.type_name(), "presenter");
    }

    // ── Het edges ─────────────────────────────────────────────────────────────

    #[test]
    fn het_edge_add_and_query_both_directions() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);

        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        let presenters = sched.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        assert_eq!(presenters, vec![pres_id]);

        let panels = sched.edges_from::<PresenterEntityType, PanelEntityType>(pres_id);
        assert_eq!(panels, vec![panel_id]);
    }

    #[test]
    fn het_edge_remove() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);

        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);
        sched.edge_remove::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        assert!(sched
            .edges_from::<PanelEntityType, PresenterEntityType>(panel_id)
            .is_empty());
        assert!(sched
            .edges_from::<PresenterEntityType, PanelEntityType>(pres_id)
            .is_empty());
    }

    #[test]
    fn het_edge_set_replaces_all() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (p1_id, p1_data) = make_presenter("Alice");
        let (p2_id, p2_data) = make_presenter("Bob");
        let (p3_id, p3_data) = make_presenter("Carol");
        sched.insert(panel_id, panel_data);
        sched.insert(p1_id, p1_data);
        sched.insert(p2_id, p2_data);
        sched.insert(p3_id, p3_data);

        sched.edge_set::<PanelEntityType, PresenterEntityType>(panel_id, vec![p1_id, p2_id]);
        let mut presenters = sched.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        presenters.sort_by_key(|id| id.uuid());
        let mut expected = vec![p1_id, p2_id];
        expected.sort_by_key(|id| id.uuid());
        assert_eq!(presenters, expected);

        sched.edge_set::<PanelEntityType, PresenterEntityType>(panel_id, vec![p3_id]);
        assert_eq!(
            sched.edges_from::<PanelEntityType, PresenterEntityType>(panel_id),
            vec![p3_id]
        );
        // p1 and p2 no longer link back to panel
        assert!(sched
            .edges_from::<PresenterEntityType, PanelEntityType>(p1_id)
            .is_empty());
        assert!(sched
            .edges_from::<PresenterEntityType, PanelEntityType>(p2_id)
            .is_empty());
    }

    #[test]
    fn remove_entity_clears_het_edges() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);
        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        sched.remove_entity::<PanelEntityType>(panel_id);

        // Edge from presenter side should be gone too
        assert!(sched
            .edges_from::<PresenterEntityType, PanelEntityType>(pres_id)
            .is_empty());
    }

    // ── EventRoom / HotelRoom heterogeneous edges ─────────────────────────────

    #[test]
    fn event_room_hotel_room_het_edge() {
        let mut sched = Schedule::new();
        let (room_id, room_data) = make_event_room("Panel 1");
        let (hotel_id, hotel_data) = make_hotel_room("East Hall");
        sched.insert(room_id, room_data);
        sched.insert(hotel_id, hotel_data);

        sched.edge_add::<EventRoomEntityType, HotelRoomEntityType>(room_id, hotel_id);

        let hotels = sched.edges_from::<EventRoomEntityType, HotelRoomEntityType>(room_id);
        assert_eq!(hotels, vec![hotel_id]);

        // Reverse: hotel_room.event_rooms via edges_from::<HotelRoom, EventRoom>
        let rooms = sched.edges_from::<HotelRoomEntityType, EventRoomEntityType>(hotel_id);
        assert_eq!(rooms, vec![room_id]);
    }

    // ── Homo edges (Presenter → Presenter) ───────────────────────────────────

    #[test]
    fn homo_edge_groups_and_members() {
        let mut sched = Schedule::new();
        let (member_id, member_data) = make_presenter("Alice");
        let (group_id, group_data) = make_presenter("The Group");
        sched.insert(member_id, member_data);
        sched.insert(group_id, group_data);

        // member → group (forward homogeneous edge: member is in group)
        sched.edge_add::<PresenterEntityType, PresenterEntityType>(member_id, group_id);

        // groups of member: edges_from(member)
        let groups = sched.edges_from::<PresenterEntityType, PresenterEntityType>(member_id);
        assert_eq!(groups, vec![group_id]);

        // members of group: edges_to(group)
        let members = sched.edges_to::<PresenterEntityType, PresenterEntityType>(group_id);
        assert_eq!(members, vec![member_id]);
    }

    #[test]
    fn homo_edge_remove() {
        let mut sched = Schedule::new();
        let (member_id, member_data) = make_presenter("Alice");
        let (group_id, group_data) = make_presenter("The Group");
        sched.insert(member_id, member_data);
        sched.insert(group_id, group_data);

        sched.edge_add::<PresenterEntityType, PresenterEntityType>(member_id, group_id);
        sched.edge_remove::<PresenterEntityType, PresenterEntityType>(member_id, group_id);

        assert!(sched
            .edges_from::<PresenterEntityType, PresenterEntityType>(member_id)
            .is_empty());
        assert!(sched
            .edges_to::<PresenterEntityType, PresenterEntityType>(group_id)
            .is_empty());
    }

    #[test]
    fn homo_edge_set_replaces() {
        let mut sched = Schedule::new();
        let (member_id, member_data) = make_presenter("Alice");
        let (g1_id, g1_data) = make_presenter("Group A");
        let (g2_id, g2_data) = make_presenter("Group B");
        sched.insert(member_id, member_data);
        sched.insert(g1_id, g1_data);
        sched.insert(g2_id, g2_data);

        sched.edge_set::<PresenterEntityType, PresenterEntityType>(member_id, vec![g1_id]);
        assert_eq!(
            sched.edges_from::<PresenterEntityType, PresenterEntityType>(member_id),
            vec![g1_id]
        );

        sched.edge_set::<PresenterEntityType, PresenterEntityType>(member_id, vec![g2_id]);
        assert_eq!(
            sched.edges_from::<PresenterEntityType, PresenterEntityType>(member_id),
            vec![g2_id]
        );
        assert!(sched
            .edges_to::<PresenterEntityType, PresenterEntityType>(g1_id)
            .is_empty());
    }

    #[test]
    fn edge_set_to_sets_members() {
        let mut sched = Schedule::new();
        let (m1_id, m1_data) = make_presenter("Alice");
        let (m2_id, m2_data) = make_presenter("Bob");
        let (g_id, g_data) = make_presenter("The Group");
        sched.insert(m1_id, m1_data);
        sched.insert(m2_id, m2_data);
        sched.insert(g_id, g_data);

        // Set members of group to [m1, m2]
        sched.edge_set_to::<PresenterEntityType, PresenterEntityType>(g_id, vec![m1_id, m2_id]);

        let mut members = sched.edges_to::<PresenterEntityType, PresenterEntityType>(g_id);
        members.sort_by_key(|id| id.uuid());
        let mut expected = vec![m1_id, m2_id];
        expected.sort_by_key(|id| id.uuid());
        assert_eq!(members, expected);

        // m1 and m2 should have group in their groups list
        assert_eq!(
            sched.edges_from::<PresenterEntityType, PresenterEntityType>(m1_id),
            vec![g_id]
        );
        assert_eq!(
            sched.edges_from::<PresenterEntityType, PresenterEntityType>(m2_id),
            vec![g_id]
        );

        // Replace with just m1
        sched.edge_set_to::<PresenterEntityType, PresenterEntityType>(g_id, vec![m1_id]);
        assert_eq!(
            sched.edges_to::<PresenterEntityType, PresenterEntityType>(g_id),
            vec![m1_id]
        );
        assert!(sched
            .edges_from::<PresenterEntityType, PresenterEntityType>(m2_id)
            .is_empty());
    }

    #[test]
    fn remove_entity_clears_homo_edges() {
        let mut sched = Schedule::new();
        let (member_id, member_data) = make_presenter("Alice");
        let (group_id, group_data) = make_presenter("The Group");
        sched.insert(member_id, member_data);
        sched.insert(group_id, group_data);
        sched.edge_add::<PresenterEntityType, PresenterEntityType>(member_id, group_id);

        sched.remove_entity::<PresenterEntityType>(member_id);

        // group should no longer see member
        assert!(sched
            .edges_to::<PresenterEntityType, PresenterEntityType>(group_id)
            .is_empty());
    }

    // ── entity_ids_to_field_value / field_value_to_entity_ids ─────────────────

    #[test]
    fn entity_ids_roundtrip_through_field_value() {
        let (id1, _) = make_presenter("Alice");
        let (id2, _) = make_presenter("Bob");
        let ids = vec![id1, id2];
        let fv = entity_ids_to_field_value(ids.clone());
        let back = field_value_to_entity_ids::<PresenterEntityType>(fv).unwrap();
        assert_eq!(back, ids);
    }

    #[test]
    fn field_value_to_entity_ids_wrong_type_is_error() {
        let (room_id, _) = make_event_room("Panel 1");
        let fv = entity_ids_to_field_value(vec![room_id]);
        let result = field_value_to_entity_ids::<PresenterEntityType>(fv);
        assert!(result.is_err());
    }

    // ── CRDT mirror ──────────────────────────────────────────────────────────

    #[test]
    fn crdt_mirror_populates_doc_on_insert() {
        use crate::crdt;
        use crate::value::{CrdtFieldType, FieldTypeItem};

        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data);

        // `prefix` was "GP" on the input InternalData; expect it in the doc.
        let prefix = crdt::read_field(
            sched.doc(),
            "panel_type",
            id.non_nil_uuid(),
            "prefix",
            FieldTypeItem::String,
            CrdtFieldType::Scalar,
        )
        .unwrap();
        assert_eq!(prefix.unwrap().to_string(), "GP");
        assert!(!crdt::is_deleted(
            sched.doc(),
            "panel_type",
            id.non_nil_uuid()
        ));
    }

    #[test]
    fn crdt_mirror_tracks_single_field_write() {
        use crate::crdt;
        use crate::entity::EntityType;
        use crate::value::{CrdtFieldType, FieldTypeItem, FieldValue, FieldValueItem};

        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data);

        PanelTypeEntityType::field_set()
            .write_field_value(
                "prefix",
                id,
                &mut sched,
                FieldValue::Single(FieldValueItem::String("SP".into())),
            )
            .unwrap();

        let got = crdt::read_field(
            sched.doc(),
            "panel_type",
            id.non_nil_uuid(),
            "prefix",
            FieldTypeItem::String,
            CrdtFieldType::Scalar,
        )
        .unwrap()
        .unwrap();
        assert_eq!(got.to_string(), "SP");
    }

    #[test]
    fn remove_entity_soft_deletes_in_doc_and_evicts_cache() {
        use crate::crdt;

        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data);
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
        assert!(!crdt::is_deleted(
            sched.doc(),
            "panel_type",
            id.non_nil_uuid()
        ));

        sched.remove_entity::<PanelTypeEntityType>(id);

        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
        assert!(crdt::is_deleted(
            sched.doc(),
            "panel_type",
            id.non_nil_uuid()
        ));
    }

    // ── Save / Load round-trip ────────────────────────────────────────────────

    #[test]
    fn save_load_roundtrips_panel_type() {
        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data);

        let bytes = sched.save();
        let loaded = Schedule::load(&bytes).expect("load");

        assert_eq!(loaded.entity_count::<PanelTypeEntityType>(), 1);
        let got = loaded.get_internal::<PanelTypeEntityType>(id).unwrap();
        assert_eq!(got.data.prefix, "GP");
        assert_eq!(got.data.panel_kind, "Guest Panel");
    }

    #[test]
    fn save_load_roundtrips_multiple_entity_types() {
        let mut sched = Schedule::new();
        let (pt_id, pt_data) = make_panel_type();
        let (pr_id, pr_data) = make_presenter("Alice");
        let (er_id, er_data) = make_event_room("Panel 1");
        let (hr_id, hr_data) = make_hotel_room("Suite A");
        sched.insert(pt_id, pt_data);
        sched.insert(pr_id, pr_data);
        sched.insert(er_id, er_data);
        sched.insert(hr_id, hr_data);

        let bytes = sched.save();
        let loaded = Schedule::load(&bytes).expect("load");

        assert_eq!(loaded.entity_count::<PanelTypeEntityType>(), 1);
        assert_eq!(loaded.entity_count::<PresenterEntityType>(), 1);
        assert_eq!(loaded.entity_count::<EventRoomEntityType>(), 1);
        assert_eq!(loaded.entity_count::<HotelRoomEntityType>(), 1);

        assert_eq!(
            loaded
                .get_internal::<PresenterEntityType>(pr_id)
                .unwrap()
                .data
                .name,
            "Alice"
        );
        assert_eq!(
            loaded
                .get_internal::<EventRoomEntityType>(er_id)
                .unwrap()
                .data
                .room_name,
            "Panel 1"
        );
        assert_eq!(
            loaded
                .get_internal::<HotelRoomEntityType>(hr_id)
                .unwrap()
                .data
                .hotel_room_name,
            "Suite A"
        );
    }

    #[test]
    fn save_load_respects_soft_delete() {
        let mut sched = Schedule::new();
        let (kept_id, kept_data) = make_panel_type();
        let (gone_id, gone_data) = make_panel_type();
        sched.insert(kept_id, kept_data);
        sched.insert(gone_id, gone_data);
        sched.remove_entity::<PanelTypeEntityType>(gone_id);

        let bytes = sched.save();
        let loaded = Schedule::load(&bytes).expect("load");

        assert_eq!(loaded.entity_count::<PanelTypeEntityType>(), 1);
        assert!(loaded
            .get_internal::<PanelTypeEntityType>(kept_id)
            .is_some());
        assert!(loaded
            .get_internal::<PanelTypeEntityType>(gone_id)
            .is_none());
    }

    #[test]
    fn load_rejects_garbage_bytes() {
        let err = Schedule::load(b"this is not an automerge doc").expect_err("must error");
        assert!(matches!(err, LoadError::Codec(_)));
    }

    // ── Native file format (FEATURE-025) ──────────────────────────────────────

    #[test]
    fn save_to_file_load_from_file_roundtrips_entity_data() {
        let mut sched = Schedule::new();
        let (pt_id, pt_data) = make_panel_type();
        let (pr_id, pr_data) = make_presenter("Alice");
        sched.insert(pt_id, pt_data);
        sched.insert(pr_id, pr_data);

        let bytes = sched.save_to_file();
        let loaded = Schedule::load_from_file(&bytes).expect("load_from_file");

        assert_eq!(loaded.entity_count::<PanelTypeEntityType>(), 1);
        assert_eq!(loaded.entity_count::<PresenterEntityType>(), 1);
        assert_eq!(
            loaded
                .get_internal::<PresenterEntityType>(pr_id)
                .unwrap()
                .data
                .name,
            "Alice"
        );
    }

    #[test]
    fn save_to_file_load_from_file_preserves_metadata() {
        let mut sched = Schedule::new();
        sched.metadata.generator = "cosam-convert 0.1".into();
        sched.metadata.version = 42;
        let saved_id = sched.metadata.schedule_id;
        let saved_at = sched.metadata.created_at;

        let bytes = sched.save_to_file();
        let loaded = Schedule::load_from_file(&bytes).expect("load_from_file");

        assert_eq!(loaded.metadata.schedule_id, saved_id);
        assert_eq!(loaded.metadata.created_at, saved_at);
        assert_eq!(loaded.metadata.generator, "cosam-convert 0.1");
        assert_eq!(loaded.metadata.version, 42);
    }

    #[test]
    fn save_to_file_load_from_file_preserves_edges() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);
        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        let bytes = sched.save_to_file();
        let loaded = Schedule::load_from_file(&bytes).expect("load_from_file");

        let forwards: Vec<PresenterId> =
            loaded.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        assert_eq!(forwards, vec![pres_id]);
    }

    #[test]
    fn load_from_file_rejects_too_short() {
        let err = Schedule::load_from_file(b"short").expect_err("must error");
        assert!(matches!(err, LoadError::Format(_)));
    }

    #[test]
    fn load_from_file_rejects_wrong_magic() {
        let mut bad = b"WRONG\x00\x01\x00\x00\x00\x00\x00".to_vec();
        bad.extend_from_slice(&automerge::AutoCommit::new().save());
        let err = Schedule::load_from_file(&bad).expect_err("must error");
        assert!(matches!(err, LoadError::Format(_)));
    }

    #[test]
    fn load_from_file_rejects_unsupported_version() {
        // Write a valid magic + version 99 header.
        let version: u16 = 99;
        let meta_json = b"{}";
        let meta_len = meta_json.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"COSAM\x00");
        buf.extend_from_slice(&version.to_le_bytes());
        buf.extend_from_slice(&meta_len.to_le_bytes());
        buf.extend_from_slice(meta_json);
        buf.extend_from_slice(&automerge::AutoCommit::new().save());
        let err = Schedule::load_from_file(&buf).expect_err("must error");
        assert!(matches!(err, LoadError::Format(_)));
    }

    #[test]
    fn load_from_file_rejects_garbage_bytes() {
        let err = Schedule::load_from_file(b"this is not a cosam file").expect_err("must error");
        assert!(matches!(err, LoadError::Format(_)));
    }

    // ── Edge CRDT round-trip (FEATURE-023) ────────────────────────────────────

    #[test]
    fn save_load_roundtrips_panel_presenter_edge() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);
        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        let bytes = sched.save();
        let loaded = Schedule::load(&bytes).expect("load");

        // Forward edge (panel → presenter)
        let forwards: Vec<PresenterId> =
            loaded.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        assert_eq!(forwards, vec![pres_id]);
        // Reverse edge (presenter → panel) also rebuilt from the single
        // owner list on the panel side.
        let reverses: Vec<PanelId> =
            loaded.edges_from::<PresenterEntityType, PanelEntityType>(pres_id);
        assert_eq!(reverses, vec![panel_id]);
    }

    #[test]
    fn save_load_roundtrips_event_room_hotel_room_edge() {
        let mut sched = Schedule::new();
        let (er_id, er_data) = make_event_room("Panel 1");
        let (hr_id, hr_data) = make_hotel_room("Suite A");
        sched.insert(er_id, er_data);
        sched.insert(hr_id, hr_data);
        sched.edge_add::<EventRoomEntityType, HotelRoomEntityType>(er_id, hr_id);

        let bytes = sched.save();
        let loaded = Schedule::load(&bytes).expect("load");

        let hotel_rooms: Vec<EntityId<HotelRoomEntityType>> =
            loaded.edges_from::<EventRoomEntityType, HotelRoomEntityType>(er_id);
        assert_eq!(hotel_rooms, vec![hr_id]);
        let event_rooms: Vec<EntityId<EventRoomEntityType>> =
            loaded.edges_from::<HotelRoomEntityType, EventRoomEntityType>(hr_id);
        assert_eq!(event_rooms, vec![er_id]);
    }

    #[test]
    fn save_load_roundtrips_presenter_group_edge() {
        let mut sched = Schedule::new();
        let (alice_id, alice) = make_presenter("Alice");
        let (group_id, group) = make_presenter("Speakers");
        sched.insert(alice_id, alice);
        sched.insert(group_id, group);
        // alice is a member of the Speakers group
        sched.edge_add::<PresenterEntityType, PresenterEntityType>(alice_id, group_id);

        let bytes = sched.save();
        let loaded = Schedule::load(&bytes).expect("load");

        let groups: Vec<PresenterId> =
            loaded.edges_from::<PresenterEntityType, PresenterEntityType>(alice_id);
        assert_eq!(groups, vec![group_id]);
        let members: Vec<PresenterId> =
            loaded.edges_to::<PresenterEntityType, PresenterEntityType>(group_id);
        assert_eq!(members, vec![alice_id]);
    }

    #[test]
    fn edge_remove_roundtrips_through_save_load() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);
        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);
        sched.edge_remove::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        let bytes = sched.save();
        let loaded = Schedule::load(&bytes).expect("load");

        let forwards: Vec<PresenterId> =
            loaded.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        assert!(forwards.is_empty());
    }

    #[test]
    fn edge_set_replaces_through_save_load() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (alice_id, alice_data) = make_presenter("Alice");
        let (bob_id, bob_data) = make_presenter("Bob");
        sched.insert(panel_id, panel_data);
        sched.insert(alice_id, alice_data);
        sched.insert(bob_id, bob_data);
        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, alice_id);
        sched.edge_set::<PanelEntityType, PresenterEntityType>(panel_id, vec![bob_id]);

        let bytes = sched.save();
        let loaded = Schedule::load(&bytes).expect("load");

        let forwards: Vec<PresenterId> =
            loaded.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        assert_eq!(forwards, vec![bob_id]);
    }

    /// Concurrent add/add from two replicas converges to the union.
    #[test]
    fn concurrent_edge_adds_merge_to_union() {
        use automerge::AutoCommit;

        // Base replica holds a panel + two presenters, no edges yet.
        let mut base = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (alice_id, alice_data) = make_presenter("Alice");
        let (bob_id, bob_data) = make_presenter("Bob");
        base.insert(panel_id, panel_data);
        base.insert(alice_id, alice_data);
        base.insert(bob_id, bob_data);
        let base_bytes = base.save();

        // Replica A adds Alice.
        let mut replica_a = Schedule::load(&base_bytes).expect("load A");
        replica_a.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, alice_id);

        // Replica B (independent) adds Bob.
        let mut replica_b = Schedule::load(&base_bytes).expect("load B");
        replica_b.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, bob_id);

        // Merge A ← B at the automerge layer, then rebuild via load().
        let mut doc_a = AutoCommit::load(&replica_a.save()).unwrap();
        let mut doc_b = AutoCommit::load(&replica_b.save()).unwrap();
        doc_a.merge(&mut doc_b).unwrap();
        let merged = Schedule::load(&doc_a.save()).expect("load merged");

        let mut forwards: Vec<PresenterId> =
            merged.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        forwards.sort_by_key(|id| id.non_nil_uuid());
        let mut expected = vec![alice_id, bob_id];
        expected.sort_by_key(|id| id.non_nil_uuid());
        assert_eq!(forwards, expected);
    }

    // ── Change tracking / merge / conflicts (FEATURE-024) ────────────────────

    #[test]
    fn merge_two_schedules_combines_entities() {
        let mut a = Schedule::new();
        let (pt_id, pt_data) = make_panel_type();
        a.insert(pt_id, pt_data);

        // B starts from the shared base state and adds a presenter.
        let mut b = Schedule::load(&a.save()).expect("load base");
        let (pr_id, pr_data) = make_presenter("Alice");
        b.insert(pr_id, pr_data);

        a.merge(&mut b).expect("merge");

        assert_eq!(a.entity_count::<PanelTypeEntityType>(), 1);
        assert_eq!(a.entity_count::<PresenterEntityType>(), 1);
        assert!(a.get_internal::<PresenterEntityType>(pr_id).is_some());
    }

    #[test]
    fn merge_preserves_edges_from_both_sides() {
        use crate::entity::EntityType;

        let mut base = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (alice_id, alice_data) = make_presenter("Alice");
        let (bob_id, bob_data) = make_presenter("Bob");
        base.insert(panel_id, panel_data);
        base.insert(alice_id, alice_data);
        base.insert(bob_id, bob_data);

        let mut a = Schedule::load(&base.save()).expect("load A");
        let mut b = Schedule::load(&base.save()).expect("load B");
        a.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, alice_id);
        b.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, bob_id);

        a.merge(&mut b).expect("merge");

        let mut ids: Vec<_> = a
            .edges_from::<PanelEntityType, PresenterEntityType>(panel_id)
            .iter()
            .map(|id| id.non_nil_uuid())
            .collect();
        ids.sort();
        let mut expected = vec![alice_id.non_nil_uuid(), bob_id.non_nil_uuid()];
        expected.sort();
        assert_eq!(ids, expected);
        let _ = PanelEntityType::TYPE_NAME; // suppress unused-trait-import warning
    }

    #[test]
    fn apply_changes_delta_sync_roundtrip() {
        // A creates a panel_type, captures heads.  B diverges: loads A's
        // state, adds a presenter, sends back only the changes A hasn't
        // observed.  A applies them and should see the new presenter.
        let mut a = Schedule::new();
        let (pt_id, pt_data) = make_panel_type();
        a.insert(pt_id, pt_data);
        let heads_a = a.get_heads();

        let mut b = Schedule::load(&a.save()).expect("load");
        let (pr_id, pr_data) = make_presenter("Alice");
        b.insert(pr_id, pr_data);

        let delta = b.get_changes_since(&heads_a);
        assert!(!delta.is_empty(), "expected at least one new change");

        a.apply_changes(&delta).expect("apply");

        assert!(a.get_internal::<PresenterEntityType>(pr_id).is_some());
        assert_eq!(a.entity_count::<PanelTypeEntityType>(), 1);
    }

    #[test]
    fn get_changes_returns_full_history() {
        let mut a = Schedule::new();
        let (pt_id, pt_data) = make_panel_type();
        a.insert(pt_id, pt_data);

        let changes = a.get_changes();
        assert!(!changes.is_empty());

        // Replay the changes into a fresh schedule and verify the entity
        // is reconstructed.
        let mut b = Schedule::new();
        b.apply_changes(&changes).expect("apply");
        assert!(b.get_internal::<PanelTypeEntityType>(pt_id).is_some());
    }

    #[test]
    fn conflicts_for_reports_concurrent_scalar_writes() {
        // Two replicas concurrently write different `prefix` values to the
        // same panel_type; after merge, `conflicts_for` surfaces both.
        use crate::entity::EntityType;
        use crate::value::{FieldValue, FieldValueItem};

        let mut base = Schedule::new();
        let (pt_id, pt_data) = make_panel_type();
        base.insert(pt_id, pt_data);

        let mut a = Schedule::load(&base.save()).expect("load A");
        let mut b = Schedule::load(&base.save()).expect("load B");

        PanelTypeEntityType::field_set()
            .write_field_value(
                "prefix",
                pt_id,
                &mut a,
                FieldValue::Single(FieldValueItem::String("A-PREFIX".into())),
            )
            .unwrap();
        PanelTypeEntityType::field_set()
            .write_field_value(
                "prefix",
                pt_id,
                &mut b,
                FieldValue::Single(FieldValueItem::String("B-PREFIX".into())),
            )
            .unwrap();

        a.merge(&mut b).expect("merge");

        let conflicts = a.conflicts_for::<PanelTypeEntityType>(pt_id, "prefix");
        let strs: Vec<String> = conflicts
            .into_iter()
            .filter_map(|fv| match fv {
                FieldValue::Single(FieldValueItem::String(s)) => Some(s),
                _ => None,
            })
            .collect();
        assert_eq!(strs.len(), 2, "expected both concurrent values: {strs:?}");
        assert!(strs.contains(&"A-PREFIX".to_string()));
        assert!(strs.contains(&"B-PREFIX".to_string()));
    }

    #[test]
    fn conflicts_for_returns_single_when_no_conflict() {
        use crate::entity::EntityType;
        use crate::value::{FieldValue, FieldValueItem};

        let mut sched = Schedule::new();
        let (pt_id, pt_data) = make_panel_type();
        sched.insert(pt_id, pt_data);
        PanelTypeEntityType::field_set()
            .write_field_value(
                "prefix",
                pt_id,
                &mut sched,
                FieldValue::Single(FieldValueItem::String("solo".into())),
            )
            .unwrap();

        let conflicts = sched.conflicts_for::<PanelTypeEntityType>(pt_id, "prefix");
        assert_eq!(conflicts.len(), 1);
        match &conflicts[0] {
            FieldValue::Single(FieldValueItem::String(s)) => assert_eq!(s, "solo"),
            other => panic!("unexpected conflict value: {other:?}"),
        }
    }

    /// Concurrent add vs. unobserved remove resolves add-wins under
    /// automerge's list semantics.
    #[test]
    fn concurrent_add_beats_unobserved_remove() {
        use automerge::AutoCommit;

        let mut base = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (alice_id, alice_data) = make_presenter("Alice");
        base.insert(panel_id, panel_data);
        base.insert(alice_id, alice_data);
        let base_bytes = base.save();

        // A adds Alice without knowing about any remove on B's side.
        let mut replica_a = Schedule::load(&base_bytes).expect("load A");
        replica_a.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, alice_id);

        // B starts from the same base (no edge), removes Alice (no-op on its
        // own state but records a causally-unordered change); this simulates
        // B having never observed A's add.
        let mut replica_b = Schedule::load(&base_bytes).expect("load B");
        replica_b.edge_remove::<PanelEntityType, PresenterEntityType>(panel_id, alice_id);

        let mut doc_a = AutoCommit::load(&replica_a.save()).unwrap();
        let mut doc_b = AutoCommit::load(&replica_b.save()).unwrap();
        doc_a.merge(&mut doc_b).unwrap();
        let merged = Schedule::load(&doc_a.save()).expect("load merged");

        // Add wins: Alice is still in the list.
        let forwards: Vec<PresenterId> =
            merged.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        assert_eq!(forwards, vec![alice_id]);
    }

    // ── Edge cache tests ───────────────────────────────────────────────────────

    #[test]
    fn inclusive_edges_from_transitive_closure() {
        use crate::entity::EntityType;
        use crate::field_set::FieldSet;
        use crate::value::ValidationError;
        use uuid::{NonNilUuid, Uuid};

        // Mock entity type for testing
        #[derive(PartialEq, Eq)]
        struct TestType;
        #[derive(Clone, Debug)]
        struct TestData;
        impl EntityType for TestType {
            type InternalData = TestData;
            type Data = TestData;
            const TYPE_NAME: &'static str = "test_type";
            fn uuid_namespace() -> &'static Uuid {
                static NS: std::sync::LazyLock<Uuid> =
                    std::sync::LazyLock::new(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"test"));
                &NS
            }
            fn field_set() -> &'static FieldSet<Self> {
                unimplemented!()
            }
            fn export(_: &TestData) -> TestData {
                TestData
            }
            fn validate(_: &TestData) -> Vec<ValidationError> {
                vec![]
            }
        }

        fn nnu(n: u128) -> NonNilUuid {
            NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
        }

        fn id(n: u128) -> EntityId<TestType> {
            unsafe { EntityId::from_uuid(nnu(n)) }
        }

        let mut sched = Schedule::new();

        // Create chain: 1 -> 2 -> 3
        sched.edge_add(id(1), id(2));
        sched.edge_add(id(2), id(3));

        // Query from 1 should reach both 2 and 3 transitively
        let result = sched.inclusive_edges_from::<TestType, TestType>(id(1));
        assert_eq!(result.len(), 2);
        assert!(result.contains(&id(2)));
        assert!(result.contains(&id(3)));
    }

    #[test]
    fn inclusive_edges_to_transitive_closure() {
        use crate::entity::EntityType;
        use crate::field_set::FieldSet;
        use crate::value::ValidationError;
        use uuid::{NonNilUuid, Uuid};

        #[derive(PartialEq, Eq)]
        struct TestType;
        #[derive(Clone, Debug)]
        struct TestData;
        impl EntityType for TestType {
            type InternalData = TestData;
            type Data = TestData;
            const TYPE_NAME: &'static str = "test_type";
            fn uuid_namespace() -> &'static Uuid {
                static NS: std::sync::LazyLock<Uuid> =
                    std::sync::LazyLock::new(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"test"));
                &NS
            }
            fn field_set() -> &'static FieldSet<Self> {
                unimplemented!()
            }
            fn export(_: &TestData) -> TestData {
                TestData
            }
            fn validate(_: &TestData) -> Vec<ValidationError> {
                vec![]
            }
        }

        fn nnu(n: u128) -> NonNilUuid {
            NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
        }

        fn id(n: u128) -> EntityId<TestType> {
            unsafe { EntityId::from_uuid(nnu(n)) }
        }

        let mut sched = Schedule::new();

        // Create chain: 1 -> 2 -> 3
        sched.edge_add(id(1), id(2));
        sched.edge_add(id(2), id(3));

        // Query to 3 should find both 1 and 2 transitively
        let result = sched.inclusive_edges_to::<TestType, TestType>(id(3));
        assert_eq!(result.len(), 2);
        assert!(result.contains(&id(1)));
        assert!(result.contains(&id(2)));
    }

    #[test]
    fn inclusive_edges_cycle_handling() {
        use crate::entity::EntityType;
        use crate::field_set::FieldSet;
        use crate::value::ValidationError;
        use uuid::{NonNilUuid, Uuid};

        #[derive(PartialEq, Eq)]
        struct TestType;
        #[derive(Clone, Debug)]
        struct TestData;
        impl EntityType for TestType {
            type InternalData = TestData;
            type Data = TestData;
            const TYPE_NAME: &'static str = "test_type";
            fn uuid_namespace() -> &'static Uuid {
                static NS: std::sync::LazyLock<Uuid> =
                    std::sync::LazyLock::new(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"test"));
                &NS
            }
            fn field_set() -> &'static FieldSet<Self> {
                unimplemented!()
            }
            fn export(_: &TestData) -> TestData {
                TestData
            }
            fn validate(_: &TestData) -> Vec<ValidationError> {
                vec![]
            }
        }

        fn nnu(n: u128) -> NonNilUuid {
            NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
        }

        fn id(n: u128) -> EntityId<TestType> {
            unsafe { EntityId::from_uuid(nnu(n)) }
        }

        let mut sched = Schedule::new();

        // Create cycle: 1 -> 2, 2 -> 1
        sched.edge_add(id(1), id(2));
        sched.edge_add(id(2), id(1));

        // Should not infinite loop
        let result = sched.inclusive_edges_from::<TestType, TestType>(id(1));
        assert!(result.contains(&id(2)));
    }

    #[test]
    fn inclusive_edges_cache_invalidation() {
        use crate::entity::EntityType;
        use crate::field_set::FieldSet;
        use crate::value::ValidationError;
        use uuid::{NonNilUuid, Uuid};

        #[derive(PartialEq, Eq)]
        struct TestType;
        #[derive(Clone, Debug)]
        struct TestData;
        impl EntityType for TestType {
            type InternalData = TestData;
            type Data = TestData;
            const TYPE_NAME: &'static str = "test_type";
            fn uuid_namespace() -> &'static Uuid {
                static NS: std::sync::LazyLock<Uuid> =
                    std::sync::LazyLock::new(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"test"));
                &NS
            }
            fn field_set() -> &'static FieldSet<Self> {
                unimplemented!()
            }
            fn export(_: &TestData) -> TestData {
                TestData
            }
            fn validate(_: &TestData) -> Vec<ValidationError> {
                vec![]
            }
        }

        fn nnu(n: u128) -> NonNilUuid {
            NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
        }

        fn id(n: u128) -> EntityId<TestType> {
            unsafe { EntityId::from_uuid(nnu(n)) }
        }

        let mut sched = Schedule::new();

        // Add initial edge
        sched.edge_add(id(1), id(2));
        let result1 = sched.inclusive_edges_from::<TestType, TestType>(id(1));
        assert_eq!(result1.len(), 1);

        // Add another edge - cache should invalidate
        sched.edge_add(id(2), id(3));
        let result2 = sched.inclusive_edges_from::<TestType, TestType>(id(1));
        assert!(result2.contains(&id(2)));
        assert!(result2.contains(&id(3)));
    }

    // ── Edge bool metadata ────────────────────────────────────────────────

    #[test]
    fn edge_get_bool_returns_default_when_not_set() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);
        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        // Default for "credited" is true; no explicit write yet.
        assert!(
            sched.edge_get_bool::<PanelEntityType, PresenterEntityType>(
                panel_id, pres_id, "credited"
            ),
            "credited should default to true"
        );
    }

    #[test]
    fn edge_set_bool_round_trip() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);
        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
            panel_id, pres_id, "credited", false,
        );
        assert!(
            !sched.edge_get_bool::<PanelEntityType, PresenterEntityType>(
                panel_id, pres_id, "credited"
            ),
            "credited should be false after set"
        );

        sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
            panel_id, pres_id, "credited", true,
        );
        assert!(
            sched.edge_get_bool::<PanelEntityType, PresenterEntityType>(
                panel_id, pres_id, "credited"
            ),
            "credited should be true after re-set"
        );
    }

    #[test]
    fn edge_meta_save_load_round_trip() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);
        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);
        sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
            panel_id, pres_id, "credited", false,
        );

        let bytes = sched.save();
        let loaded = Schedule::load(&bytes).expect("load");
        assert!(
            !loaded.edge_get_bool::<PanelEntityType, PresenterEntityType>(
                panel_id, pres_id, "credited"
            ),
            "credited=false must survive save/load"
        );
    }
}
