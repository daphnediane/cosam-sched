/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`Schedule`] — top-level coordination container.
//!
//! Full implementation (EntityStorage, EdgeMap indexes, UUID registry,
//! ScheduleMetadata) is in FEATURE-019. This stub provides the minimal
//! type-erased storage needed so the `field.rs` blanket impls compile and
//! the associated tests pass.

use crate::entity::{EntityId, EntityType};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use uuid::Uuid;

/// Top-level schedule container.
///
/// Holds all entity storage, edge indexes, and the UUID registry.
/// Full implementation in FEATURE-019.
#[derive(Debug, Default)]
pub struct Schedule {
    /// Type-erased entity storage: `(TypeId of E, Uuid) → Box<dyn Any + Send + Sync>`.
    ///
    /// Each value is a `Box<E::InternalData>`. Full typed storage per entity kind
    /// is implemented in FEATURE-019.
    storage: HashMap<(TypeId, Uuid), Box<dyn Any + Send + Sync>>,
}

impl Schedule {
    /// Retrieve a shared reference to an entity's internal data.
    ///
    /// Returns `None` if the entity is not present or the stored type does
    /// not match `E::InternalData`.
    pub fn get_internal<E: EntityType>(&self, id: EntityId<E>) -> Option<&E::InternalData> {
        let key = (TypeId::of::<E::InternalData>(), id.uuid());
        self.storage
            .get(&key)
            .and_then(|b| b.downcast_ref::<E::InternalData>())
    }

    /// Retrieve a mutable reference to an entity's internal data.
    ///
    /// Returns `None` if the entity is not present or the stored type does
    /// not match `E::InternalData`.
    pub fn get_internal_mut<E: EntityType>(
        &mut self,
        id: EntityId<E>,
    ) -> Option<&mut E::InternalData> {
        let key = (TypeId::of::<E::InternalData>(), id.uuid());
        self.storage
            .get_mut(&key)
            .and_then(|b| b.downcast_mut::<E::InternalData>())
    }

    /// Insert or replace an entity's internal data.
    ///
    /// Used by tests and builders. Full entity lifecycle (insert, remove,
    /// UUID registry) is implemented in FEATURE-019.
    pub fn insert<E: EntityType>(&mut self, id: EntityId<E>, data: E::InternalData) {
        let key = (TypeId::of::<E::InternalData>(), id.uuid());
        self.storage.insert(key, Box::new(data));
    }
}

#[cfg(test)]
impl Schedule {
    /// Test-only convenience alias for [`Schedule::insert`].
    pub fn insert_mock<E: EntityType>(&mut self, id: EntityId<E>, data: E::InternalData) {
        self.insert(id, data);
    }
}
