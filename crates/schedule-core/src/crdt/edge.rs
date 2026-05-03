/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CRDT-backed edge storage (FEATURE-023).
//!
//! `RawEdgeMap` remains the fast in-memory bidirectional index used by all
//! `Schedule::edges_*` queries.  The authoritative state, however, lives in
//! the automerge document as an `ObjType::List` of EntityIdentifier scalars
//! on a single **canonical owner** entity per relation.
//!
//! Canonical owners follow the panels-outward rule from FEATURE-023:
//!
//! | Relation                        | Owner     | Field                   | Homogenous? |
//! |---------------------------------|-----------|-------------------------|-------------|
//! | Panel ↔ Presenter (credited)   | Panel     | `credited_presenters`   | no          |
//! | Panel ↔ Presenter (uncredited) | Panel     | `uncredited_presenters` | no          |
//! | Panel ↔ EventRoom              | Panel     | `event_rooms`           | no          |
//! | Panel ↔ Session               | Panel     | `sessions`              | yes         |
//! | Session ↔ Presenter           | Session   | `presenters`            | yes         |
//!
//! The CRDT storage is a single source of truth that can be saved to disk
//! and merged across replicas.  The in-memory `RawEdgeMap` is derived from
//! it and used for fast queries.

use crate::crdt;
use crate::crdt::CrdtFieldType;
use crate::entity::{EntityUuid, RuntimeEntityId};
use crate::field::NamedField;
use crate::value::{FieldTypeItem, FieldValue, FieldValueItem};
use automerge::transaction::Transactable;
use automerge::{AutoCommit, ObjType, ReadDoc, Value};
use uuid::NonNilUuid;

/// Resolved CRDT ownership for an edge, looked up from the field inventory.
#[derive(Clone, Copy)]
pub struct CanonicalOwner {
    /// `true` when the near (queried) field is the edge_kind owner side.
    pub near_is_owner: bool,
    /// The owner-side field (carries `Owner { target_field: … }`).
    pub owner_field: &'static crate::edge::HalfEdgeDescriptor,
    /// The target-side field (the inverse/lookup field).
    pub target_field: &'static crate::edge::HalfEdgeDescriptor,
}

impl CanonicalOwner {
    /// Owner entity type name.
    #[must_use]
    pub fn owner_type(&self) -> &'static str {
        self.owner_field.entity_type_name()
    }

    /// Target entity type name.
    #[must_use]
    pub fn target_type(&self) -> &'static str {
        self.target_field.entity_type_name()
    }

    /// Owner-side field name (the CRDT list key).
    #[must_use]
    pub fn field_name(&self) -> &'static str {
        self.owner_field.name()
    }
}

impl std::fmt::Debug for CanonicalOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanonicalOwner")
            .field("near_is_owner", &self.near_is_owner)
            .field("owner_type", &self.owner_type())
            .field("target_type", &self.target_type())
            .field("field_name", &self.field_name())
            .finish()
    }
}

/// Resolve CRDT ownership for an edge given both field descriptors.
///
/// Each field knows its own [`crate::edge::EdgeKind`], so resolution
/// is a constant-time check on the two supplied fields:
///
/// - If `near_field` is `Owner { target_field }` and `target_field`
///   identifies `far_field`, `near` is the owner.
/// - Else if `far_field` is `Owner { target_field }` and `target_field`
///   identifies `near_field`, `far` is the owner.
/// - Otherwise the pair is not a recognized edge.
///
/// Taking both fields makes the lookup unambiguous even when multiple edge
/// types exist between the same pair of entity types (e.g. FEATURE-065:
/// `credited_presenters` and `uncredited_presenters` both target
/// `HALF_EDGE_PANELS`).
#[must_use]
pub fn canonical_owner(
    near_field: &'static crate::edge::HalfEdgeDescriptor,
    far_field: &'static crate::edge::HalfEdgeDescriptor,
) -> Option<CanonicalOwner> {
    fn same(
        a: &'static crate::edge::HalfEdgeDescriptor,
        b: &'static crate::edge::HalfEdgeDescriptor,
    ) -> bool {
        a.name() == b.name() && a.entity_type_name() == b.entity_type_name()
    }
    if let crate::edge::EdgeKind::Owner { target_field, .. } = near_field.edge_kind {
        if same(target_field, far_field) {
            return Some(CanonicalOwner {
                near_is_owner: true,
                owner_field: near_field,
                target_field: far_field,
            });
        }
    }
    if let crate::edge::EdgeKind::Owner { target_field, .. } = far_field.edge_kind {
        if same(target_field, near_field) {
            return Some(CanonicalOwner {
                near_is_owner: false,
                owner_field: far_field,
                target_field: near_field,
            });
        }
    }
    None
}

/// Ensure that the empty list object exists at `owner.field_name` so that
/// concurrent replicas both inherit the same `ObjId` when they later add
/// entries.  No-op when the list is already present.
///
/// This is called by `Schedule::insert` for every canonical owner field on
/// the inserted entity type.  Without this step, two replicas that both
/// create the first edge would each `put_object(List)` on divergent
/// branches, and automerge's merge would silently discard one side's
/// inserts (LWW on object identity).
///
/// # Errors
/// Propagates [`crdt::CrdtError`] from the underlying automerge operations.
pub fn ensure_owner_list(
    doc: &mut AutoCommit,
    owner_type: &str,
    owner_uuid: NonNilUuid,
    field_name: &str,
) -> Result<(), crdt::CrdtError> {
    let parent = crdt::ensure_entity_map(doc, owner_type, owner_uuid)?;
    match doc.get(&parent, field_name)? {
        Some((Value::Object(ObjType::List), _)) => Ok(()),
        Some(_) | None => {
            doc.put_object(&parent, field_name, ObjType::List)?;
            Ok(())
        }
    }
}

/// Incrementally append `target_uuid` to `owner.field_name` if not already
/// present.  Used by the `Schedule::edge_add` mirror path so concurrent
/// adds from two replicas converge to the union rather than LWW.
///
/// # Errors
/// Propagates [`crdt::CrdtError`] from the underlying automerge operations.
pub fn list_append_unique(
    doc: &mut AutoCommit,
    owner_type: &str,
    owner_uuid: NonNilUuid,
    target_type: &'static str,
    field_name: &str,
    target_uuid: NonNilUuid,
) -> Result<(), crdt::CrdtError> {
    let parent = crdt::ensure_entity_map(doc, owner_type, owner_uuid)?;
    // Reuse existing list or create one.
    let list_id = match doc.get(&parent, field_name)? {
        Some((Value::Object(ObjType::List), id)) => id,
        Some(_) | None => doc.put_object(&parent, field_name, ObjType::List)?,
    };
    // Check for duplicate entries under this actor's history; automerge's
    // merge will still admit parallel inserts of the same uuid by
    // concurrent actors, which we dedup on read.
    let len = doc.length(&list_id);
    let target_rid_str = format!("{}:{}", target_type, target_uuid);
    for i in 0..len {
        if let Some((Value::Scalar(scalar), _)) = doc.get(&list_id, i)? {
            if let automerge::ScalarValue::Str(s) = scalar.as_ref() {
                if s.as_ref() == target_rid_str.as_str() {
                    return Ok(());
                }
            }
        }
    }
    // SAFETY: target_type/target_uuid are carried together throughout the
    // edge API; this tags the scalar string consistently with how
    // `write_owner_list` does.
    let rid = unsafe { RuntimeEntityId::new_unchecked(target_uuid, target_type) };
    let scalar = crdt::item_to_scalar(&FieldValueItem::EntityIdentifier(rid))?;
    doc.insert(&list_id, len, scalar)?;
    Ok(())
}

/// Incrementally remove `target_uuid` from `owner.field_name` (every
/// occurrence).  Used by `Schedule::edge_remove` so concurrent
/// add-vs-unobserved-remove resolves add-wins: a remove operation deletes
/// only indices the actor has already observed; a concurrent add on
/// another replica inserts at a position the remover never saw, so it
/// survives the merge.
///
/// # Errors
/// Propagates [`crdt::CrdtError`] from the underlying automerge operations.
pub fn list_remove_uuid(
    doc: &mut AutoCommit,
    owner_type: &str,
    owner_uuid: NonNilUuid,
    target_type: &'static str,
    field_name: &str,
    target_uuid: NonNilUuid,
) -> Result<(), crdt::CrdtError> {
    let parent = crdt::ensure_entity_map(doc, owner_type, owner_uuid)?;
    let Some((Value::Object(ObjType::List), list_id)) = doc.get(&parent, field_name)? else {
        return Ok(());
    };
    let target_rid_str = format!("{}:{}", target_type, target_uuid);
    // Walk back-to-front so deletions don't shift remaining indices we
    // still need to inspect.
    let len = doc.length(&list_id);
    for i in (0..len).rev() {
        if let Some((Value::Scalar(scalar), _)) = doc.get(&list_id, i)? {
            if let automerge::ScalarValue::Str(s) = scalar.as_ref() {
                if s.as_ref() == target_rid_str.as_str() {
                    doc.delete(&list_id, i)?;
                }
            }
        }
    }
    Ok(())
}

/// Replace-style full-list rewrite for `owner.field_name`.  Used only
/// internally when the caller explicitly wants LWW-on-the-whole-list
/// semantics (reasonable for user-driven bulk "replace" actions,
/// documented as such in `docs/crdt-design.md`).
///
/// Reuses the existing list object when present so that follow-up
/// incremental operations from a divergent replica can still merge.
///
/// # Errors
/// Propagates [`crdt::CrdtError`] from the underlying automerge operations.
pub fn write_owner_list(
    doc: &mut AutoCommit,
    owner_type: &'static str,
    owner_uuid: NonNilUuid,
    target_type: &'static str,
    field_name: &'static str,
    target_uuids: &[NonNilUuid],
) -> Result<(), crdt::CrdtError> {
    let items: Vec<FieldValueItem> = target_uuids
        .iter()
        .map(|u| {
            // SAFETY: `u` came from the in-memory edge index which already
            // tracks the entity's type; we are merely tagging it for the
            // CRDT write.
            let rid = unsafe { RuntimeEntityId::new_unchecked(*u, target_type) };
            FieldValueItem::EntityIdentifier(rid)
        })
        .collect();
    let value = FieldValue::List(items);
    crdt::write_field(
        doc,
        owner_type,
        owner_uuid,
        field_name,
        CrdtFieldType::List,
        &value,
    )
}

// ── Per-edge metadata ──────────────────────────────────────────────────────────

/// Derive the CRDT map key for per-edge metadata from the membership list field
/// name (e.g. `"presenters"` → `"presenters_meta"`).
#[must_use]
pub fn meta_field_name(field_name: &str) -> String {
    format!("{field_name}_meta")
}

/// Read a boolean per-edge property from the `{field_name}_meta` map.
///
/// Path: `entities/{owner_type}/{owner_uuid}/{meta_field}/{target_uuid}/{prop_name}`
///
/// Returns `default` when any level of the path is absent (no explicit value written).
#[must_use]
pub fn read_edge_meta_bool(
    doc: &AutoCommit,
    owner_type: &str,
    owner_uuid: NonNilUuid,
    field_name: &str,
    target_uuid: NonNilUuid,
    prop_name: &str,
    default: bool,
) -> bool {
    let meta_key = meta_field_name(field_name);
    let target_key = target_uuid.to_string();
    // Walk the path read-only; return default at any missing level.
    let Some(entity_map) = crdt::get_entity_map(doc, owner_type, owner_uuid) else {
        return default;
    };
    let Some((Value::Object(ObjType::Map), meta_map_id)) =
        doc.get(&entity_map, meta_key.as_str()).ok().flatten()
    else {
        return default;
    };
    let Some((Value::Object(ObjType::Map), target_map_id)) =
        doc.get(&meta_map_id, target_key.as_str()).ok().flatten()
    else {
        return default;
    };
    match doc.get(&target_map_id, prop_name).ok().flatten() {
        Some((Value::Scalar(s), _)) => match s.as_ref() {
            automerge::ScalarValue::Boolean(b) => *b,
            _ => default,
        },
        _ => default,
    }
}

/// Write a boolean per-edge property into the `{field_name}_meta` map (LWW).
///
/// Path: `entities/{owner_type}/{owner_uuid}/{meta_field}/{target_uuid}/{prop_name}`
///
/// Intermediate maps are created if absent.
///
/// # Errors
/// Propagates [`crdt::CrdtError`] from the underlying automerge operations.
pub fn write_edge_meta_bool(
    doc: &mut AutoCommit,
    owner_type: &str,
    owner_uuid: NonNilUuid,
    field_name: &str,
    target_uuid: NonNilUuid,
    prop_name: &str,
    value: bool,
) -> Result<(), crdt::CrdtError> {
    let meta_key = meta_field_name(field_name);
    let target_key = target_uuid.to_string();
    let entity_map = crdt::ensure_entity_map(doc, owner_type, owner_uuid)?;
    let meta_map_id = crdt::ensure_map(doc, &entity_map, meta_key.as_str())?;
    let target_map_id = crdt::ensure_map(doc, &meta_map_id, target_key.as_str())?;
    doc.put(
        &target_map_id,
        prop_name,
        automerge::ScalarValue::Boolean(value),
    )?;
    Ok(())
}

/// Read `owner`'s `field_name` list from the CRDT document, returning the
/// UUIDs of the target entities.  Missing or empty lists yield `Vec::new()`.
#[must_use]
pub fn read_owner_list(
    doc: &AutoCommit,
    owner_type: &'static str,
    owner_uuid: NonNilUuid,
    field_name: &'static str,
    target_type: FieldTypeItem,
) -> Vec<NonNilUuid> {
    let Ok(Some(FieldValue::List(items))) = crdt::read_field(
        doc,
        owner_type,
        owner_uuid,
        field_name,
        target_type,
        CrdtFieldType::List,
    ) else {
        return Vec::new();
    };
    items
        .into_iter()
        .filter_map(|it| match it {
            FieldValueItem::EntityIdentifier(rid) => Some(rid.entity_uuid()),
            _ => None,
        })
        .collect()
}
