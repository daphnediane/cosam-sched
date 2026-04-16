/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity type system — [`EntityType`] trait, [`EntityId`], [`EntityKind`].
//!
//! Full implementation (`EntityKind`, `RuntimeEntityId`, `UuidPreference`,
//! `EntityId` methods, `NonNilUuid`) is in FEATURE-012. This module exposes
//! the minimal types required by `field.rs` so the full trait hierarchy
//! compiles now.

use crate::value::ValidationError;
use std::fmt;
use std::marker::PhantomData;
use uuid::Uuid;

/// Opaque handle to the per-entity-type field registry.
///
/// Full implementation (name lookup, required/indexable iterators, etc.)
/// is in FEATURE-013. Forward-declared here so [`EntityType`] can reference it.
pub struct FieldSet<E: EntityType> {
    _marker: PhantomData<E>,
}

/// Core trait implemented by every entity type singleton struct.
///
/// Full implementation scaffolding (`EntityKind`, `RuntimeEntityId`,
/// `UuidPreference`) is in FEATURE-012.
pub trait EntityType: 'static + Sized {
    /// Runtime storage struct; the field system operates on this.
    type InternalData: Clone + Send + Sync + fmt::Debug + 'static;

    /// Export/API view produced by [`EntityType::export`].
    type Data: Clone;

    /// Short, stable name for this entity type (e.g. `"panel_type"`).
    const TYPE_NAME: &'static str;

    /// Return the static field registry for this entity type.
    fn field_set() -> &'static FieldSet<Self>;

    /// Produce the public export view from internal storage data.
    fn export(internal: &Self::InternalData) -> Self::Data;

    /// Validate internal data and return any constraint violations.
    fn validate(data: &Self::InternalData) -> Vec<ValidationError>;
}

/// Compile-time type-safe entity identifier.
///
/// Wraps a [`Uuid`] with a `PhantomData<E>` so the type system prevents
/// mixing IDs from different entity types. Full methods (`new()`, `from_uuid()`,
/// serde, `NonNilUuid` backing) are implemented in FEATURE-012.
#[derive(PartialEq, Eq, Hash)]
pub struct EntityId<E: EntityType> {
    uuid: Uuid,
    _marker: PhantomData<fn() -> E>,
}

// Manual Clone/Copy impls — the derive macros would add `E: Clone`/`E: Copy`
// as unnecessary bounds. `Uuid` and `PhantomData<fn() -> E>` are always both.
impl<E: EntityType> Clone for EntityId<E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E: EntityType> Copy for EntityId<E> {}

impl<E: EntityType> EntityId<E> {
    /// Construct a typed entity ID, returning `None` if `uuid` is nil.
    ///
    /// This is the only way to construct an `EntityId<E>` — the nil check
    /// upholds the non-nil invariant that `non_nil_uuid()` relies on.
    pub fn new(uuid: Uuid) -> Option<Self> {
        if uuid.is_nil() {
            None
        } else {
            Some(Self {
                uuid,
                _marker: PhantomData,
            })
        }
    }

    /// Return the underlying [`Uuid`].
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

impl<E: EntityType> fmt::Debug for EntityId<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EntityId<{}>({:?})", E::TYPE_NAME, self.uuid)
    }
}

impl<E: EntityType> fmt::Display for EntityId<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.uuid)
    }
}
