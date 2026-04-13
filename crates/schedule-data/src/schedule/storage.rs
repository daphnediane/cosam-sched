/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity storage implementation

use std::collections::HashMap;

use crate::entity::{
    EntityKind, EntityType, EventRoomEntityType, EventRoomId, HotelRoomEntityType, HotelRoomId,
    InternalData, PanelEntityType, PanelId, PanelTypeEntityType, PanelTypeId, PresenterEntityType,
    PresenterId, TypedId,
};
use uuid::NonNilUuid;

use super::{EdgeMap, EntityMap};

/// Error type for entity insertion conflicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertError {
    /// UUID already exists in storage.
    UuidCollision { uuid: NonNilUuid },
}

/// Error type returned by `Builder::build()` — covers both field validation
/// failures and UUID collision on insert.
#[derive(Debug, Clone)]
pub enum BuildError {
    /// One or more required fields failed validation.
    Validation(crate::field::validation::ValidationError),
    /// The entity's UUID collided with an existing entry in the schedule.
    Insert(InsertError),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::Validation(e) => write!(f, "validation error: {e}"),
            BuildError::Insert(e) => write!(f, "insert error: {e}"),
        }
    }
}

impl std::error::Error for BuildError {}

impl From<crate::field::validation::ValidationError> for BuildError {
    fn from(e: crate::field::validation::ValidationError) -> Self {
        BuildError::Validation(e)
    }
}

impl From<InsertError> for BuildError {
    fn from(e: InsertError) -> Self {
        BuildError::Insert(e)
    }
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::UuidCollision { uuid } => {
                write!(f, "UUID collision: {uuid} already exists")
            }
        }
    }
}

impl std::error::Error for InsertError {}

/// Concrete typed entity storage using generic type-safe wrappers.
///
/// Uses [`EntityMap`] for entity storage and [`EdgeMap`] for bidirectional
/// reverse lookup indexes, providing type-safe operations with minimal overhead.
#[derive(Debug, Clone, Default)]
/// Central storage for all entities and their relationships.
///
/// This struct contains all entity data and bidirectional relationship indexes.
/// It serves as the single source of truth for the schedule data model.
///
/// ## Structure
///
/// - **Entity maps**: Direct storage for each entity type (`panels`, `presenters`, etc.)
/// - **EdgeMaps**: Bidirectional indexes for efficient relationship lookup in both directions
/// - **UUID registry**: Maps all known UUIDs to their entity kind for type-agnostic lookup
///
/// ## Relationship Management
///
/// All EdgeMaps are automatically maintained by the `EntityType` lifecycle hooks:
/// - `on_insert()` - adds edges when entities are created
/// - `on_soft_delete()` - removes edges when entities are soft deleted
/// - `on_update()` - updates edges when relationships change
/// - `on_soft_delete_cleanup_edges()` - removes edges pointing to soft-deleted entities
///
/// ## Thread Safety
///
/// This struct is not thread-safe and should be accessed through a single thread
/// or protected by appropriate synchronization mechanisms.
pub struct EntityStorage {
    /// All panel entities in the schedule
    pub panels: EntityMap<PanelEntityType>,
    /// All presenter entities in the schedule
    pub presenters: EntityMap<PresenterEntityType>,
    /// All event room entities in the schedule
    pub event_rooms: EntityMap<EventRoomEntityType>,
    /// All hotel room entities in the schedule
    pub hotel_rooms: EntityMap<HotelRoomEntityType>,
    /// All panel type entities in the schedule
    pub panel_types: EntityMap<PanelTypeEntityType>,

    // Bidirectional edge indexes — maintained by entity type hooks
    /// Maps panel types to their assigned panels (PanelTypeId -> [PanelId])
    pub panels_by_panel_type: EdgeMap<PanelTypeId, PanelId>,
    /// Maps event rooms to their assigned panels (EventRoomId -> [PanelId])
    pub panels_by_event_room: EdgeMap<EventRoomId, PanelId>,
    /// Maps presenters to their assigned panels (PresenterId -> [PanelId])
    pub panels_by_presenter: EdgeMap<PresenterId, PanelId>,
    /// Maps hotel rooms to their assigned event rooms (HotelRoomId -> [EventRoomId])
    pub event_rooms_by_hotel_room: EdgeMap<HotelRoomId, EventRoomId>,
    /// Group-member edges: left = group `PresenterId`, right = member `PresenterId`
    pub presenter_group_members: EdgeMap<PresenterId, PresenterId>,

    /// UUID registry mapping every known UUID to its entity kind.
    /// Used for type-agnostic UUID lookup and validation.
    pub uuid_registry: HashMap<NonNilUuid, EntityKind>,
}

/// Provides access to the concrete [`EntityMap`] for an entity type.
/// Implemented on `EntityType` marker structs.
pub trait TypedStorage: EntityType + Sized {
    fn typed_map(storage: &EntityStorage) -> &EntityMap<Self>;
    fn typed_map_mut(storage: &mut EntityStorage) -> &mut EntityMap<Self>;
}

impl TypedStorage for PanelEntityType {
    fn typed_map(s: &EntityStorage) -> &EntityMap<PanelEntityType> {
        &s.panels
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut EntityMap<PanelEntityType> {
        &mut s.panels
    }
}

impl TypedStorage for PresenterEntityType {
    fn typed_map(s: &EntityStorage) -> &EntityMap<PresenterEntityType> {
        &s.presenters
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut EntityMap<PresenterEntityType> {
        &mut s.presenters
    }
}

impl TypedStorage for EventRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &EntityMap<EventRoomEntityType> {
        &s.event_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut EntityMap<EventRoomEntityType> {
        &mut s.event_rooms
    }
}

impl TypedStorage for HotelRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &EntityMap<HotelRoomEntityType> {
        &s.hotel_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut EntityMap<HotelRoomEntityType> {
        &mut s.hotel_rooms
    }
}

impl TypedStorage for PanelTypeEntityType {
    fn typed_map(s: &EntityStorage) -> &EntityMap<PanelTypeEntityType> {
        &s.panel_types
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut EntityMap<PanelTypeEntityType> {
        &mut s.panel_types
    }
}

impl EntityStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get entity by type and UUID.
    pub fn get<T: TypedStorage>(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        T::typed_map(self).get(T::Id::from_uuid(uuid))
    }

    /// Get entity by internal UUID (alias for `get`).
    pub fn get_by_uuid<T: TypedStorage>(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        T::typed_map(self).get(T::Id::from_uuid(uuid))
    }

    /// Get entities by index query, returning all that tie at the best match strength.
    ///
    /// Returns typed IDs of all entities that match at the best priority level.
    pub fn get_by_index<T>(&self, query: &str) -> Vec<T::Id>
    where
        T: EntityType + TypedStorage,
    {
        let field_set = T::field_set();
        let map = T::typed_map(self);

        let mut best_priority = crate::field::traits::match_priority::MIN_MATCH;
        let mut matched_uuids: Vec<NonNilUuid> = Vec::new();

        for entity in map.values() {
            if let Some(match_result) = field_set.match_index(query, entity) {
                if match_result.priority > best_priority {
                    best_priority = match_result.priority;
                    matched_uuids.clear();
                    matched_uuids.push(match_result.entity_uuid);
                } else if match_result.priority == best_priority {
                    matched_uuids.push(match_result.entity_uuid);
                }
            }
        }

        matched_uuids.into_iter().map(T::Id::from_uuid).collect()
    }

    /// Look up an entity by an indexable field value.
    ///
    /// Searches all entities of type T by their indexable fields and returns
    /// the entity if exactly one matches at the best priority level.
    pub fn lookup_by_indexable<T>(&self, query: &str) -> Option<&T::Data>
    where
        T: EntityType + TypedStorage,
    {
        let ids = self.get_by_index::<T>(query);
        if ids.len() == 1 {
            EntityStore::<T>::get_entity(self, ids[0].non_nil_uuid())
        } else {
            None
        }
    }

    /// Add entity to storage with pre-allocated UUID, registering it in the UUID registry.
    pub fn add_with_uuid<T: TypedStorage>(
        &mut self,
        uuid: NonNilUuid,
        entity: T::Data,
    ) -> Result<(), InsertError> {
        let entity_for_hook = entity.clone();
        EntityStore::<T>::insert_entity(self, uuid, entity)?;
        T::on_insert(self, &entity_for_hook);
        Ok(())
    }

    /// Insert a new entity from its data struct, registering it in the UUID registry.
    ///
    /// The UUID is taken from `data.id().non_nil_uuid()`. Returns the typed ID on success.
    pub fn add_entity<T: TypedStorage>(&mut self, data: T::Data) -> Result<T::Id, InsertError> {
        let uuid = data.id().non_nil_uuid();
        let id = T::Id::from_uuid(uuid);
        let data_for_hook = data.clone();
        EntityStore::<T>::insert_entity(self, uuid, data)?;
        T::on_insert(self, &data_for_hook);
        Ok(id)
    }

    /// Check if entity with given UUID exists.
    pub fn contains_uuid<T: TypedStorage>(&self, uuid: NonNilUuid) -> bool {
        T::typed_map(self).contains_key(T::Id::from_uuid(uuid))
    }

    /// Get a mutable reference to an entity by type and UUID.
    pub fn get_mut<T: TypedStorage>(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data> {
        T::typed_map_mut(self).get_mut(T::Id::from_uuid(uuid))
    }

    /// Remove an entity by type and UUID, unregistering its UUID.
    /// Calls the entity type's `on_soft_delete` hook before removal.
    pub fn remove<T: TypedStorage>(&mut self, uuid: NonNilUuid) -> Option<T::Data> {
        let data = T::typed_map(self).get(T::Id::from_uuid(uuid)).cloned()?;
        T::on_soft_delete(self, &data);
        EntityStore::<T>::remove_entity(self, uuid)
    }
}

/// Trait for any storage backend that can hold entities of type `T`.
///
/// Implemented by [`super::Schedule`] via a blanket impl over [`TypedStorage`].
/// Alternative backends (e.g. a string-field cache for `cosam-modify`) can
/// implement this independently.
pub trait EntityStore<T: EntityType> {
    /// Retrieve an entity by UUID.
    fn get_entity(&self, uuid: NonNilUuid) -> Option<&T::Data>;

    /// Retrieve a mutable reference to an entity by UUID.
    fn get_entity_mut(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data>;

    /// Insert an entity with a pre-allocated UUID.
    /// Returns `Err` if the UUID already exists.
    fn insert_entity(&mut self, uuid: NonNilUuid, data: T::Data) -> Result<(), InsertError>;

    /// Remove an entity by UUID, returning the data if it existed.
    fn remove_entity(&mut self, uuid: NonNilUuid) -> Option<T::Data>;

    /// Check if an entity with the given UUID exists.
    fn contains_entity(&self, uuid: NonNilUuid) -> bool;
}

/// Blanket implementation: `EntityStorage` is an `EntityStore<T>` for any
/// entity type that has a [`TypedStorage`] mapping.
///
/// UUID registration is handled here: `insert_entity` registers the UUID in
/// the `uuid_registry`, and `remove_entity` unregisters it.
impl<T: TypedStorage> EntityStore<T> for EntityStorage {
    fn get_entity(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        T::typed_map(self).get(T::Id::from_uuid(uuid))
    }

    fn get_entity_mut(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data> {
        T::typed_map_mut(self).get_mut(T::Id::from_uuid(uuid))
    }

    fn insert_entity(&mut self, uuid: NonNilUuid, data: T::Data) -> Result<(), InsertError> {
        // Check for cross-type UUID collision
        let existing_kind = self.uuid_registry.get(&uuid).copied();
        if let Some(kind) = existing_kind {
            if kind != T::KIND {
                return Err(InsertError::UuidCollision { uuid });
            }
        }
        // Check for same-type duplicate
        let id = T::Id::from_uuid(uuid);
        if T::typed_map(self).contains_key(id) {
            return Err(InsertError::UuidCollision { uuid });
        }
        self.uuid_registry.insert(uuid, T::KIND);
        T::typed_map_mut(self).insert(id, data);
        Ok(())
    }

    fn remove_entity(&mut self, uuid: NonNilUuid) -> Option<T::Data> {
        let id = T::Id::from_uuid(uuid);
        let result = T::typed_map_mut(self).remove(id);
        if result.is_some() {
            self.uuid_registry.remove(&uuid);
        }
        result
    }

    fn contains_entity(&self, uuid: NonNilUuid) -> bool {
        T::typed_map(self).contains_key(T::Id::from_uuid(uuid))
    }
}
