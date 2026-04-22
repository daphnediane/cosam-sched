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
//! | Relation                     | Owner     | Field          | Homo? |
//! |------------------------------|-----------|----------------|-------|
//! | Panel ↔ Presenter            | Panel     | `presenters`   | no    |
//! | Panel ↔ EventRoom            | Panel     | `event_rooms`  | no    |
//! | Panel → PanelType            | Panel     | `panel_type`   | no    |
//! | EventRoom ↔ HotelRoom        | EventRoom | `hotel_rooms`  | no    |
//! | Presenter → Group            | Presenter | `groups`       | yes   |
//!
//! Every `Schedule::edge_add` / `edge_remove` / `edge_set` / `edge_set_to`
//! call resolves the canonical owner for its `(L, R)` pair and writes the
//! post-mutation list to the doc so that concurrent replicas converge under
//! automerge's list semantics (add-wins for concurrent add/remove,
//! union-of-inserts for concurrent adds — see `docs/crdt-design.md`).

use crate::crdt;
use crate::entity::RuntimeEntityId;
use crate::value::{CrdtFieldType, FieldTypeItem, FieldValue, FieldValueItem};
use automerge::transaction::Transactable;
use automerge::{AutoCommit, ObjType, ReadDoc, Value};
use uuid::NonNilUuid;

/// Which side of the `(L, R)` edge owns the CRDT list, plus the field name.
#[derive(Debug, Clone, Copy)]
pub struct CanonicalOwner {
    /// `true` when the left entity owns the field; `false` when the right
    /// does (lookup from the reverse-side call site).
    pub owner_is_left: bool,
    /// The entity type that owns the field — always equal to `L::TYPE_NAME`
    /// if `owner_is_left`, otherwise `R::TYPE_NAME`.
    pub owner_type: &'static str,
    /// The entity type stored in the list — i.e. the non-owner side.
    pub target_type: &'static str,
    /// Name of the list field on the owner.
    pub field_name: &'static str,
}

/// Resolve the canonical owner for edges between `l_type` and `r_type`.
///
/// Searches [`crate::edge_descriptor::ALL_EDGE_DESCRIPTORS`] for a descriptor
/// whose `(owner_type, target_type)` or `(target_type, owner_type)` pair matches
/// `(l_type, r_type)`.
///
/// Returns `None` if the pair is not a recognised relationship.
#[must_use]
pub fn canonical_owner(l_type: &str, r_type: &str) -> Option<CanonicalOwner> {
    use crate::edge_descriptor::ALL_EDGE_DESCRIPTORS;
    for desc in ALL_EDGE_DESCRIPTORS {
        if desc.owner_type == l_type && desc.target_type == r_type {
            // L is the owner side.
            return Some(CanonicalOwner {
                owner_is_left: true,
                owner_type: desc.owner_type,
                target_type: desc.target_type,
                field_name: desc.field_name,
            });
        }
        if !desc.is_homogeneous && desc.target_type == l_type && desc.owner_type == r_type {
            // R is the owner side (heterogeneous only — homo edges don't have a
            // separate reverse direction; the left side always owns).
            return Some(CanonicalOwner {
                owner_is_left: false,
                owner_type: desc.owner_type,
                target_type: desc.target_type,
                field_name: desc.field_name,
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

/// Ensure every canonical owner-list field on the given entity type exists
/// on `owner_uuid`'s entity map.
///
/// Called from `Schedule::insert` (via `mirror_entity_fields`) so that edge
/// mutations made on forks of this doc converge via `insert` / `delete`
/// against a shared list object.
///
/// # Errors
/// Propagates [`crdt::CrdtError`] from the first failing `ensure_owner_list`.
pub fn ensure_all_owner_lists_for_type(
    doc: &mut AutoCommit,
    owner_type: &str,
    owner_uuid: NonNilUuid,
) -> Result<(), crdt::CrdtError> {
    use crate::edge_descriptor::ALL_EDGE_DESCRIPTORS;
    for desc in ALL_EDGE_DESCRIPTORS {
        if desc.owner_type == owner_type {
            ensure_owner_list(doc, owner_type, owner_uuid, desc.field_name)?;
        }
    }
    Ok(())
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
    // concurrent actors, which we dedupe on read.
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
    let rid = unsafe { RuntimeEntityId::from_uuid(target_uuid, target_type) };
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

/// Replace-style full-list rewrite for `owner.field_name`.  Used by
/// `Schedule::edge_set` / `edge_set_to` when the caller explicitly wants
/// LWW-on-the-whole-list semantics (reasonable for user-driven bulk
/// "replace" actions, documented as such in `docs/crdt-design.md`).
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
            let rid = unsafe { RuntimeEntityId::from_uuid(*u, target_type) };
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
            FieldValueItem::EntityIdentifier(rid) => Some(rid.uuid()),
            _ => None,
        })
        .collect()
}
