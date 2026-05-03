/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CRDT save/load/merge operations for [`Schedule`].

use crate::crdt::CrdtFieldType;
use crate::crdt::{self, CrdtError};
use crate::edge::{EdgeKind, FullEdge, HalfEdge, RawEdgeMap};
use crate::entity::{registered_entity_types, EntityType, EntityUuid};
use crate::field::{NamedField, ReadableField};
use crate::value::{FieldError, FieldValue};
use crate::EntityId;
use automerge::AutoCommit;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::NonNilUuid;

use super::Schedule;

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
pub const FILE_MAGIC: &[u8; 6] = b"COSAM\x00";

/// Current native file format version.
pub const FILE_FORMAT_VERSION: u16 = 1;

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

// ── CRDT save/load implementation ─────────────────────────────────────────────

impl Schedule {
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
        if !matches!(desc.crdt_type(), CrdtFieldType::Scalar) {
            return Vec::new();
        }
        let Some(entity_map) = crdt::get_entity_map(&self.doc, E::TYPE_NAME, id.entity_uuid())
        else {
            return Vec::new();
        };
        let Ok(values) = self.doc.get_all(&entity_map, desc.name()) else {
            return Vec::new();
        };
        let item_type = desc.field_type().item_type();
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
            rehydrate_fn: fn(
                &mut Schedule,
                NonNilUuid,
            ) -> Result<NonNilUuid, crate::edit::builder::BuildError>,
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
    /// Iterates every field in the inventory whose `edge_kind` is
    /// [`EdgeKind::Owner`], iterates every live owner uuid in the
    /// doc, reads the list, and `add_edge`s each endpoint pair into the cache.
    /// The caller is responsible for running this under
    /// [`Self::with_mirror_disabled`] — otherwise each replayed edge would
    /// re-write the same list back into the doc.
    fn rebuild_edges_from_doc(&mut self) {
        use crate::value::FieldTypeItem;

        // Snapshot the `(owner_uuid, target_uuids)` pairs while borrowing
        // `&self.doc`, then apply them under `&mut self`.
        struct EdgeBatch {
            owner_field: &'static dyn crate::edge::HalfEdge,
            target_field: &'static dyn crate::edge::HalfEdge,
            pairs: Vec<(NonNilUuid, Vec<NonNilUuid>)>,
        }
        let mut batches: Vec<EdgeBatch> = Vec::new();
        for collected in crate::field::all_named_fields() {
            let Some(owner_nf) = collected.0.try_as_half_edge() else {
                continue;
            };
            let EdgeKind::Owner {
                target_field: target_nf,
                ..
            } = owner_nf.edge_kind()
            else {
                continue;
            };
            let owner_type = owner_nf.entity_type_name();
            let field_name = owner_nf.name();
            let target_type = target_nf.entity_type_name();
            let owner_field = owner_nf.edge_id();
            let target_field = target_nf.edge_id();
            let owner_uuids = crdt::list_all_uuids(&self.doc, owner_type);
            let mut pairs: Vec<(NonNilUuid, Vec<NonNilUuid>)> = Vec::new();
            for owner_uuid in owner_uuids {
                if crdt::is_deleted(&self.doc, owner_type, owner_uuid) {
                    continue;
                }
                let targets = crate::crdt::edge::read_owner_list(
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
                    owner_field,
                    target_field,
                    pairs,
                });
            }
        }
        for batch in batches {
            for (owner_uuid, targets) in batch.pairs {
                // SAFETY: owner_uuid and target_uuid come from edge descriptors which
                // guarantee type compatibility with their respective fields.
                let owner_type = batch.owner_field.entity_type_name();
                let target_type = batch.target_field.entity_type_name();
                let from = unsafe {
                    crate::entity::RuntimeEntityId::new_unchecked(owner_uuid, owner_type)
                };
                let edge = FullEdge {
                    near: batch.owner_field,
                    far: batch.target_field,
                };
                let targets: Vec<crate::entity::RuntimeEntityId> = targets
                    .into_iter()
                    .map(|target_uuid| unsafe {
                        crate::entity::RuntimeEntityId::new_unchecked(target_uuid, target_type)
                    })
                    .collect();
                self.edges
                    .add_edge(from, edge, targets)
                    .expect("edge type validation failed");
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
        let uuid = id.entity_uuid();
        let type_name = E::TYPE_NAME;
        crdt::touch_entity(&mut self.doc, type_name, uuid)?;
        // Collect (name, crdt_type, value) while holding `&self`, then apply
        // writes while holding `&mut self.doc`.
        let mut pending: Vec<(&'static str, CrdtFieldType, FieldValue)> = Vec::new();
        for desc in E::field_set().fields() {
            if !matches!(
                desc.crdt_type(),
                CrdtFieldType::Scalar | CrdtFieldType::Text | CrdtFieldType::List
            ) {
                continue;
            }
            if let Ok(Some(v)) = desc.read(id, self) {
                pending.push((desc.name(), desc.crdt_type(), v));
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
        //
        // We derive which fields own edges directly from the `Owner`
        // variant in each field descriptor, avoiding a global descriptor scan.
        for desc in E::field_set().fields() {
            if matches!(desc.edge_kind(), EdgeKind::Owner { .. }) {
                crate::crdt::edge::ensure_owner_list(&mut self.doc, type_name, uuid, desc.name())?;
            }
        }
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
        let uuid = id.entity_uuid();
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
}
