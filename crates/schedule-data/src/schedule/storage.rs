/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity storage implementation

use std::collections::HashMap;

use crate::entity::{
    EntityKind, EntityType, EventRoomData, EventRoomEntityType, HotelRoomData, HotelRoomEntityType,
    InternalData, PanelData, PanelEntityType, PanelTypeData, PanelTypeEntityType, PresenterData,
    PresenterEntityType, TypedId,
};
use uuid::NonNilUuid;

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

/// Concrete typed entity storage — one `HashMap` per entity type.
/// This avoids type erasure and allows direct `&T::Data` references.
#[derive(Debug, Clone, Default)]
pub struct EntityStorage {
    // Node entities
    pub panels: HashMap<NonNilUuid, PanelData>,
    pub presenters: HashMap<NonNilUuid, PresenterData>,
    pub event_rooms: HashMap<NonNilUuid, EventRoomData>,
    pub hotel_rooms: HashMap<NonNilUuid, HotelRoomData>,
    pub panel_types: HashMap<NonNilUuid, PanelTypeData>,

    // Reverse lookup indexes — maintained by entity type hooks
    pub panels_by_panel_type: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    pub panels_by_event_room: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    pub panels_by_presenter: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    pub event_rooms_by_hotel_room: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    pub presenters_by_group: HashMap<NonNilUuid, Vec<NonNilUuid>>,

    /// UUID registry mapping every known UUID to its entity kind.
    pub uuid_registry: HashMap<NonNilUuid, EntityKind>,
}

/// Provides access to the concrete `HashMap` for an entity type.
/// Implemented on `EntityType` marker structs.
pub trait TypedStorage: EntityType {
    fn typed_map(storage: &EntityStorage) -> &HashMap<NonNilUuid, Self::Data>;
    fn typed_map_mut(storage: &mut EntityStorage) -> &mut HashMap<NonNilUuid, Self::Data>;
}

impl TypedStorage for PanelEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelData> {
        &s.panels
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelData> {
        &mut s.panels
    }
}

impl TypedStorage for PresenterEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PresenterData> {
        &s.presenters
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PresenterData> {
        &mut s.presenters
    }
}

impl TypedStorage for EventRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, EventRoomData> {
        &s.event_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, EventRoomData> {
        &mut s.event_rooms
    }
}

impl TypedStorage for HotelRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, HotelRoomData> {
        &s.hotel_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, HotelRoomData> {
        &mut s.hotel_rooms
    }
}

impl TypedStorage for PanelTypeEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelTypeData> {
        &s.panel_types
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelTypeData> {
        &mut s.panel_types
    }
}

impl EntityStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get entity by type and UUID.
    pub fn get<T: TypedStorage>(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        T::typed_map(self).get(&uuid)
    }

    /// Get entity by internal UUID (alias for `get`).
    pub fn get_by_uuid<T: TypedStorage>(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        T::typed_map(self).get(&uuid)
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
    /// The UUID is taken from `data.uuid()`. Returns the typed ID on success.
    pub fn add_entity<T: TypedStorage>(&mut self, data: T::Data) -> Result<T::Id, InsertError> {
        let uuid = data.uuid();
        let data_for_hook = data.clone();
        EntityStore::<T>::insert_entity(self, uuid, data)?;
        T::on_insert(self, &data_for_hook);
        Ok(T::Id::from_uuid(uuid))
    }

    /// Check if entity with given UUID exists.
    pub fn contains_uuid<T: TypedStorage>(&self, uuid: NonNilUuid) -> bool {
        T::typed_map(self).contains_key(&uuid)
    }

    /// Get a mutable reference to an entity by type and UUID.
    pub fn get_mut<T: TypedStorage>(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data> {
        T::typed_map_mut(self).get_mut(&uuid)
    }

    /// Remove an entity by type and UUID, unregistering its UUID.
    /// Calls the entity type's `on_remove` hook before removal.
    pub fn remove<T: TypedStorage>(&mut self, uuid: NonNilUuid) -> Option<T::Data> {
        let data = T::typed_map(self).get(&uuid).cloned()?;
        T::on_remove(self, &data);
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
        T::typed_map(self).get(&uuid)
    }

    fn get_entity_mut(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data> {
        T::typed_map_mut(self).get_mut(&uuid)
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
        if T::typed_map(self).contains_key(&uuid) {
            return Err(InsertError::UuidCollision { uuid });
        }
        self.uuid_registry.insert(uuid, T::KIND);
        T::typed_map_mut(self).insert(uuid, data);
        Ok(())
    }

    fn remove_entity(&mut self, uuid: NonNilUuid) -> Option<T::Data> {
        let result = T::typed_map_mut(self).remove(&uuid);
        if result.is_some() {
            self.uuid_registry.remove(&uuid);
        }
        result
    }

    fn contains_entity(&self, uuid: NonNilUuid) -> bool {
        T::typed_map(self).contains_key(&uuid)
    }
}
