/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity type system — [`EntityType`] trait, [`UuidPreference`], and related types.
//!
//! Entity identifiers are defined in [`crate::entity::id`]:
//! - [`crate::entity::id::EntityId`] — compile-time type-safe entity identifier
//! - [`crate::entity::id::RuntimeEntityId`] — dynamic (untyped) entity identifier
//! - [`crate::entity::id::DynamicEntityId`] — dynamic entity identifier trait
//! - [`crate::entity::id::EntityUuid`] — entity UUID wrapper
//! - [`crate::entity::id::EntityTyped`] — entity UUID with type marker
//! - [`crate::entity::id::TypedEntityId`] — typed entity ID alias
//!
//! Field node identifiers are defined in [`crate::edge`]:
//! - [`crate::edge::FieldNodeId`] — compile-time type-safe field node identifier
//! - [`crate::edge::RuntimeFieldNodeId`] — dynamic (untyped) field node identifier
//!
//! Non-nil UUID identity uses [`uuid::NonNilUuid`] from the `uuid` crate
//! directly.

pub mod id;

use crate::value::ValidationError;
use std::fmt;
use uuid::{NonNilUuid, Uuid};

// ── Re-exports from id ─────────────────────────────────────────────────────────

pub use id::{DynamicEntityId, EntityId, EntityTyped, EntityUuid, RuntimeEntityId, TypedEntityId};

// ── UuidPreference ────────────────────────────────────────────────────────────

/// Controls UUID assignment when creating a new entity via a builder.
///
/// Most business logic should not name this type directly — it is a builder
/// concern.
#[derive(Debug, Clone)]
pub enum UuidPreference {
    /// Generate a fresh v7 (time-ordered) UUID.
    ///
    /// This is the default for new entities with no external natural key.
    GenerateNew,

    /// Derive a deterministic v5 UUID from the entity-type namespace and a
    /// natural-key string (e.g. `"GP001"`, a presenter name, a room name).
    ///
    /// Re-importing the same spreadsheet produces the same UUIDs.
    FromV5 { name: String },

    /// Use an exact, caller-supplied UUID.
    ///
    /// Use this when round-tripping a previously serialized entity so its
    /// identity is preserved unchanged.
    Exact(NonNilUuid),
}

// ── FieldSet ─────────────────────────────────────────────────────────────────

/// Re-export so callers can use `entity::FieldSet<E>` without importing `field_set`.
pub use crate::field::set::FieldSet;

// ── EntityType trait ──────────────────────────────────────────────────────────

/// Core trait implemented by every entity type singleton struct.
pub trait EntityType: 'static + Sized {
    /// Runtime storage struct; the field system operates on this.
    type InternalData: Clone + Send + Sync + fmt::Debug + 'static;

    /// Export/API view produced by [`EntityType::export`].
    type Data: Clone;

    /// Short, stable name for this entity type (e.g. `"panel_type"`).
    const TYPE_NAME: &'static str;

    /// The v5 UUID namespace for this entity type.
    ///
    /// This namespace is used for deterministic v5 UUID generation from
    /// natural keys (e.g., `"GP001"`). Each entity type has a unique,
    /// fixed namespace to ensure IDs derived from the same name are
    /// unique across types.
    ///
    /// Implementations should use a `static LazyLock<Uuid>` internally
    /// to compute the namespace once and return a reference.
    fn uuid_namespace() -> &'static Uuid;

    /// Return the static field registry for this entity type.
    fn field_set() -> &'static FieldSet<Self>;

    /// Produce the public export view from internal storage data.
    fn export(internal: &Self::InternalData) -> Self::Data;

    /// Validate internal data and return any constraint violations.
    fn validate(data: &Self::InternalData) -> Vec<ValidationError>;
}

// ── Inventory registration types ──────────────────────────────────────────────

/// Type alias for the entity build function used in edit commands.
///
/// Builds an entity with the given exact UUID and field name+value pairs.
/// Used by the edit command system to replay `AddEntity` / undo `RemoveEntity`.
pub type EntityBuildFn = fn(
    &mut crate::schedule::Schedule,
    NonNilUuid,
    &[(&'static str, crate::value::FieldValue)],
) -> Result<NonNilUuid, crate::edit::builder::BuildError>;

/// Type-erased entity type descriptor, registered globally via `inventory`.
///
/// Each concrete entity type impl block submits one of these. Use
/// [`registered_entity_types`] to iterate all registered types at runtime.
pub struct RegisteredEntityType {
    /// Stable snake_case type name (e.g. `"panel"`, `"presenter"`).
    pub type_name: &'static str,
    /// Returns the UUID namespace used for deterministic v5 ID generation.
    pub uuid_namespace: fn() -> &'static Uuid,
    /// Returns the `TypeId` of this entity type's `InternalData` associated type.
    /// Used by `Schedule::identify` to map a bare UUID to its entity type.
    pub type_id: fn() -> std::any::TypeId,
    /// Build an entity with the given exact UUID and field name+value pairs.
    ///
    /// Used by the edit command system to replay `AddEntity` / undo `RemoveEntity`.
    /// The UUID is always used as-is (`UuidPreference::Exact`), guaranteeing
    /// that redo recreates the same identity.  Field names are canonical names
    /// or aliases registered in the entity's [`FieldSet`].
    ///
    /// Returns the resulting [`NonNilUuid`] on success, or a [`BuildError`] if
    /// any write or validation step fails.
    ///
    /// [`FieldSet`]: crate::field::set::FieldSet
    /// [`BuildError`]: crate::edit::builder::BuildError
    pub build_fn: EntityBuildFn,

    /// Read a single field value from an existing entity by field name.
    ///
    /// Used by the edit command system to capture `old_value` before applying
    /// an `UpdateField` command.  Returns `None` if the field returns no value
    /// (unset optional), or `Err` if the field is write-only or the entity is
    /// absent.
    pub read_field_fn: fn(
        &crate::schedule::Schedule,
        NonNilUuid,
        &'static str,
    )
        -> Result<Option<crate::value::FieldValue>, crate::value::FieldError>,

    /// Write a single field value into an existing entity by field name.
    ///
    /// Used by the edit command system to apply and undo `UpdateField` commands.
    /// Returns `Err` if the field is read-only, the entity is absent, or the
    /// value conversion fails.
    pub write_field_fn: fn(
        &mut crate::schedule::Schedule,
        NonNilUuid,
        &'static str,
        crate::value::FieldValue,
    ) -> Result<(), crate::value::FieldError>,

    /// Snapshot all read+write fields of an existing entity into a
    /// `Vec<(&'static str, FieldValue)>`.
    ///
    /// Used by the edit command system to capture state before `RemoveEntity`,
    /// enabling undo to restore the entity via [`Self::build_fn`].
    /// Only fields that have both `read_fn` and `write_fn` are included;
    /// read-only computed fields and write-only modifier fields are skipped.
    /// Fields whose read returns `None` (unset optional fields) are also skipped.
    pub snapshot_fn:
        fn(&crate::schedule::Schedule, NonNilUuid) -> Vec<(&'static str, crate::value::FieldValue)>,

    /// Remove the entity with the given UUID from the schedule, clearing all edges.
    ///
    /// Used by the edit command system to apply `RemoveEntity` and undo `AddEntity`.
    pub remove_fn: fn(&mut crate::schedule::Schedule, NonNilUuid),

    /// Rehydrate an entity from the authoritative CRDT document into the
    /// in-memory cache.
    ///
    /// Reads every non-derived writable field for this entity type out of
    /// `schedule.doc()` via [`crate::crdt::read_field`], collects them into
    /// a `(field_name, FieldValue)` batch, and invokes
    /// [`crate::edit::builder::build_entity`] with `UuidPreference::Exact(uuid)`.
    ///
    /// The caller is responsible for disabling the CRDT mirror
    /// ([`crate::schedule::Schedule::with_mirror_disabled`]) before calling
    /// this so rehydrated writes don't re-emit change records against the
    /// doc they were just read from.
    pub rehydrate_fn: fn(
        &mut crate::schedule::Schedule,
        NonNilUuid,
    ) -> Result<NonNilUuid, crate::edit::builder::BuildError>,
}
inventory::collect!(RegisteredEntityType);

/// Iterate over all entity types registered via `inventory::submit!`.
pub fn registered_entity_types() -> impl Iterator<Item = &'static RegisteredEntityType> {
    inventory::iter::<RegisteredEntityType>()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registered_entity_types_contains_all_five() {
        let names: Vec<&'static str> = registered_entity_types().map(|r| r.type_name).collect();
        for expected in &[
            "panel",
            "presenter",
            "event_room",
            "hotel_room",
            "panel_type",
        ] {
            assert!(
                names.contains(expected),
                "registered_entity_types() missing \"{expected}\"; got {names:?}"
            );
        }
        assert_eq!(names.len(), 5, "expected exactly 5 registered entity types");
    }
}
