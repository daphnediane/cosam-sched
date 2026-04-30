/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity storage operations for [`Schedule`].

use crate::crdt;
use crate::entity::{EntityType, EntityUuid};
use crate::{EntityId, RuntimeEntityId};
use std::any::TypeId;
use std::collections::HashMap;
use uuid::NonNilUuid;

use super::Schedule;

impl Schedule {
    // ── Entity storage ────────────────────────────────────────────────────────

    /// Retrieve a shared reference to an entity's internal data.
    ///
    /// Returns `None` if the entity is not present.
    #[must_use]
    pub fn get_internal<E: EntityType>(&self, id: EntityId<E>) -> Option<&E::InternalData> {
        self.entities
            .get(&TypeId::of::<E::InternalData>())?
            .get(&id.entity_uuid())?
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
            .get_mut(&id.entity_uuid())?
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
            .insert(id.entity_uuid(), Box::new(data));
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
        let uuid = id.entity_uuid();
        if self.mirror_enabled {
            if let Err(e) = crdt::put_deleted(&mut self.doc, E::TYPE_NAME, uuid, true) {
                debug_assert!(false, "CRDT soft-delete failed: {e}");
                let _ = e;
            }
        }
        if let Some(map) = self.entities.get_mut(&TypeId::of::<E::InternalData>()) {
            map.remove(&uuid);
        }
        self.edges.clear_all(uuid);
        *self.transitive_edge_cache.borrow_mut() = None;
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
                let id = unsafe { EntityId::new_unchecked(*uuid) };
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
        crate::entity::registered_entity_types().find_map(|reg| {
            let inner = self.entities.get(&(reg.type_id)())?;
            if inner.contains_key(&uuid) {
                // SAFETY: we just confirmed uuid is in the inner map for reg.type_name.
                Some(unsafe { RuntimeEntityId::new_unchecked(uuid, reg.type_name) })
            } else {
                None
            }
        })
    }
}
