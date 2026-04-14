/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `CrdtDocument` trait — the abstraction boundary between the entity/field
//! system and the CRDT backend.
//!
//! Implementors wrap a specific CRDT library (currently automerge) and expose
//! a uniform interface for creating, mutating, serialising, and merging schedule
//! documents.  The entity and edit layers interact with this trait only —
//! they never import automerge types directly.
//!
//! ## Document structure
//!
//! The document root contains one map per entity kind plus an `actors/` map:
//!
//! ```text
//! document root
//! ├── panels/       { uuid_str → { field → value, ... } }
//! ├── presenters/   { uuid_str → { field → value, ... } }
//! ├── event_rooms/  { uuid_str → { field → value, ... } }
//! ├── hotel_rooms/  { uuid_str → { field → value, ... } }
//! ├── panel_types/  { uuid_str → { field → value, ... } }
//! └── actors/       { actor_id_str → { display_name } }
//! ```
//!
//! Entity maps grow monotonically — there are no hard deletes.  Soft-delete is
//! expressed by setting the entity's identifying fields to `CrdtScalar::Null`.

use crate::entity::EntityKind;
use super::{ActorId, CrdtOp, CrdtScalar};

/// Abstraction over a CRDT-backed schedule document.
///
/// Each method maps to one or more operations on the underlying CRDT library.
/// The trait is generic so the entity and edit layers can be unit-tested with
/// a stub backend and the production code can use `AutomergeDocument`.
///
/// ## Swapping backends
///
/// To use a different CRDT library, implement `CrdtDocument` for a new type.
/// No other code changes are required as long as the field-type → CRDT-type
/// mapping is preserved (see [`CrdtOp`] for the mapping).
pub trait CrdtDocument: Sized + Send + Sync {
    /// Error type returned by backend operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new empty document, attributing subsequent operations to `actor`.
    ///
    /// The actor ID is embedded in every automerge operation so that causal
    /// ordering and LWW tiebreaking work correctly when merging with peers.
    fn new(actor: &ActorId) -> Result<Self, Self::Error>;

    /// Deserialise a document from bytes produced by [`save`][Self::save].
    ///
    /// The loaded document retains whatever actor ID was last set by the
    /// device that wrote the saved bytes.  Call [`set_actor`][Self::set_actor]
    /// before making further writes if you need to change the signing actor.
    fn load(bytes: &[u8]) -> Result<Self, Self::Error>;

    /// Change the actor ID used for subsequent write operations.
    ///
    /// Useful after loading a foreign device's file for a read/merge-only
    /// operation — set your own actor before writing so ops are attributed
    /// to this device.
    fn set_actor(&mut self, actor: &ActorId);

    /// Serialise the document to bytes for storage or transmission.
    ///
    /// The bytes can be passed to [`load`][Self::load] or
    /// [`merge_from`][Self::merge_from] on another replica.
    fn save(&mut self) -> Vec<u8>;

    /// Merge another document (given as raw saved bytes) into `self`.
    ///
    /// Automerge merge is commutative and idempotent — re-merging a file
    /// already seen is safe.  After merging all peer files in a shared folder,
    /// all replicas converge to the same state.
    fn merge_from(&mut self, other_bytes: &[u8]) -> Result<(), Self::Error>;

    /// Apply a single field-level operation to this document.
    ///
    /// The operation is immediately committed.  If the entity referenced by
    /// `entity_uuid` does not yet exist, field ops create the entity map
    /// implicitly (equivalent to `EnsureEntity` + the field op).
    fn apply(&mut self, op: &CrdtOp) -> Result<(), Self::Error>;

    // -----------------------------------------------------------------------
    // Read operations
    // -----------------------------------------------------------------------

    /// Read a scalar field value from the document.
    ///
    /// Returns `None` if the entity or field does not exist in the document.
    fn read_scalar(
        &self,
        kind: EntityKind,
        uuid: uuid::Uuid,
        field: &str,
    ) -> Option<CrdtScalar>;

    /// Read the current text content of a prose field.
    ///
    /// Returns `None` if the entity or field does not exist, or if the field
    /// is not a `Text` object (use [`read_scalar`][Self::read_scalar] for
    /// `Str`-typed fields).
    fn read_text(&self, kind: EntityKind, uuid: uuid::Uuid, field: &str) -> Option<String>;

    /// Read the UUID list of a relationship field.
    ///
    /// Returns an empty `Vec` if the entity or field does not exist.
    /// Deduplicates on read — concurrent adds from multiple actors may create
    /// duplicate entries in the underlying list, and this method normalises
    /// them away.
    fn read_list(&self, kind: EntityKind, uuid: uuid::Uuid, field: &str) -> Vec<uuid::Uuid>;

    /// Check whether an entity map has been created in the document.
    fn entity_exists(&self, kind: EntityKind, uuid: uuid::Uuid) -> bool;
}
