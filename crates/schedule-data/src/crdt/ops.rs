/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field-level CRDT operation types.
//!
//! [`CrdtOp`] is the atomic unit passed to [`super::CrdtDocument::apply`].
//! Each variant maps directly to one or more automerge calls:
//!
//! | Variant | automerge operation |
//! |---|---|
//! | `EnsureEntity` | `put_object(kind_map, uuid_str, ObjType::Map)` (idempotent) |
//! | `PutScalar` | `put(entity_map, field, scalar_value)` |
//! | `PutText` | `splice_text(text_obj, 0, current_len, new_text)` |
//! | `ListAdd` | `insert(list_obj, len, uuid_str)` (deduplicated) |
//! | `ListRemove` | `delete(list_obj, idx)` for all matching entries |

use crate::entity::EntityKind;

/// Field-level CRDT operation.
///
/// Corresponds to one user-visible mutation of a schedule entity, expressed
/// at a level the CRDT backend can directly execute and replicate.
///
/// ### Relationship between `EditCommand` and `CrdtOp`
///
/// An [`crate::edit::EditCommand`] captures the high-level intent (e.g.,
/// "update the title of panel X from A to B").  When the edit system is
/// wired to a CRDT backend, each applied command will emit one or more
/// `CrdtOp`s that are applied to the document and broadcast to peers.
#[derive(Debug, Clone, PartialEq)]
pub enum CrdtOp {
    /// Create the entity map entry in the document (idempotent).
    ///
    /// Must be applied before any field ops for a newly-created entity.
    /// Applying to an entity that already exists is safe and has no effect.
    EnsureEntity {
        entity_kind: EntityKind,
        entity_uuid: uuid::Uuid,
    },

    /// Set a scalar field using LWW (Last-Write-Wins) semantics.
    ///
    /// Use for: `String`, `Integer`, `Float`, `Boolean`, `DateTime`,
    /// `Duration`, and UUID-typed reference fields (`event_room_id`,
    /// `panel_type_id`, etc.).
    ///
    /// Concurrent writes to the same field are resolved by automerge's
    /// internal clock + actor ID tiebreaker.
    PutScalar {
        entity_kind: EntityKind,
        entity_uuid: uuid::Uuid,
        field_name: String,
        value: CrdtScalar,
    },

    /// Overwrite a prose field using character-level RGA.
    ///
    /// Use for: `description`, `note`, `notes_non_printing`, `workshop_notes`,
    /// `av_notes`.  The full text content is replaced.  For partial in-place
    /// edits (e.g., from a GUI text editor), the automerge document's Text
    /// object can be manipulated directly via `splice_text`.
    ///
    /// This `PutText` variant is used by the edit command path where we have
    /// the complete new value (e.g., from a field update or import).
    PutText {
        entity_kind: EntityKind,
        entity_uuid: uuid::Uuid,
        field_name: String,
        text: String,
    },

    /// Add a UUID element to a relationship list field.
    ///
    /// Use for: `presenter_ids`, `event_room_ids`, `group_ids`, etc.
    ///
    /// Idempotent: if the element is already present in the list, this is a
    /// no-op.  Concurrent adds from different actors are union-merged (both
    /// survive), giving OR-Set-equivalent add-wins semantics.
    ListAdd {
        entity_kind: EntityKind,
        entity_uuid: uuid::Uuid,
        field_name: String,
        element: uuid::Uuid,
    },

    /// Remove all occurrences of a UUID element from a relationship list field.
    ///
    /// No-op if the element is not present.  An element added concurrently by
    /// another actor (not yet observed by this replica) will survive — the
    /// remove only cancels tokens that were observed at the time of removal.
    ListRemove {
        entity_kind: EntityKind,
        entity_uuid: uuid::Uuid,
        field_name: String,
        element: uuid::Uuid,
    },
}

/// Scalar value for CRDT field operations.
///
/// Mirrors [`crate::field::FieldValue`] for the leaf (non-container) cases
/// and maps directly to automerge `ScalarValue` variants.  Container types
/// (`List`, `Text`) are handled by dedicated [`CrdtOp`] variants instead.
#[derive(Debug, Clone, PartialEq)]
pub enum CrdtScalar {
    /// Null / absent value (soft-delete marker for optional fields).
    Null,
    /// Boolean field (`sewing_machines`, `is_break`, `is_explicit_group`, etc.).
    Bool(bool),
    /// Integer field (rank, sort_key counters, etc.).
    Int(i64),
    /// Floating-point field.
    Float(f64),
    /// Short string field (name, uid, cost, rank label, etc.).
    Str(String),
    /// `NaiveDateTime` encoded as milliseconds since Unix epoch.
    ///
    /// Maps to automerge `ScalarValue::Timestamp`.
    TimestampMs(i64),
    /// `chrono::Duration` encoded as total minutes.
    ///
    /// Maps to automerge `ScalarValue::Int`.
    DurationMins(i64),
    /// UUID reference field (`event_room_id`, `panel_type_id`, etc.).
    ///
    /// Stored as a string in automerge to avoid encoding complexity.
    Uuid(uuid::Uuid),
}
