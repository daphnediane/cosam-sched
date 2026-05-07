/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity storage operations for [`Schedule`].

use crate::crdt;
use crate::entity::{EntityType, EntityUuid};
use crate::sidecar::ChangeState;
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

    /// Check whether an entity with the given ID already exists.
    ///
    /// Returns `true` if an entity of type `E` with the specified UUID is present
    /// in the schedule.
    #[must_use]
    pub fn contains_entity<E: EntityType>(&self, id: EntityId<E>) -> bool {
        self.entities
            .get(&TypeId::of::<E::InternalData>())
            .is_some_and(|map| map.contains_key(&id.entity_uuid()))
    }

    /// Check whether an entity with the given ID is tombstoned (soft-deleted).
    ///
    /// Returns `true` if the entity exists in the CRDT document but is marked
    /// as deleted. Tombstoned entities can be recreated with `Exact` or
    /// `ExactFromV5` UUID preferences.
    #[must_use]
    pub fn is_entity_deleted<E: EntityType>(&self, id: EntityId<E>) -> bool {
        crate::crdt::is_deleted(&self.doc, E::TYPE_NAME, id.entity_uuid())
    }

    /// Resolve a UUID preference to an entity ID with conflict checking.
    ///
    /// Returns `None` for `Exact` and `ExactFromV5` preferences if the UUID
    /// conflicts with an existing non-tombstoned entity. For `Prefer` and
    /// `PreferFromV5`, falls back to `GenerateNew` on conflict. Returns `Some`
    /// for `GenerateNew` unconditionally.
    ///
    /// This is the safe alternative to `EntityId::from_preference_unchecked`.
    #[must_use]
    pub fn try_resolve_entity_id<E: EntityType>(
        &self,
        preference: crate::entity::UuidPreference,
    ) -> Option<EntityId<E>> {
        match &preference {
            crate::entity::UuidPreference::Exact(_)
            | crate::entity::UuidPreference::ExactFromV5 { .. } => {
                // SAFETY: We check for conflicts before using the resolved UUID
                let id = unsafe { EntityId::<E>::from_preference_unchecked(preference) };
                if self.contains_entity(id) && !self.is_entity_deleted(id) {
                    None
                } else {
                    Some(id)
                }
            }
            crate::entity::UuidPreference::Prefer(_)
            | crate::entity::UuidPreference::PreferFromV5 { .. } => {
                // SAFETY: We check for conflicts before using the resolved UUID
                let id = unsafe { EntityId::<E>::from_preference_unchecked(preference) };
                if self.contains_entity(id) && !self.is_entity_deleted(id) {
                    // Fallback to GenerateNew if the preferred UUID conflicts
                    Some(unsafe {
                        EntityId::<E>::from_preference_unchecked(
                            crate::entity::UuidPreference::GenerateNew,
                        )
                    })
                } else {
                    Some(id)
                }
            }
            crate::entity::UuidPreference::GenerateNew => {
                // SAFETY: GenerateNew is always conflict-free
                Some(unsafe { EntityId::<E>::from_preference_unchecked(preference) })
            }
        }
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
        let uuid = id.entity_uuid();
        self.entities
            .entry(TypeId::of::<E::InternalData>())
            .or_default()
            .insert(uuid, Box::new(data));
        // Only track during normal operation — not during rehydration from the doc.
        if self.mirror_enabled {
            self.mark_entity_changed(uuid, ChangeState::Added);
        }
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
        self.mark_entity_changed(uuid, ChangeState::Deleted);
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
