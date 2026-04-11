/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity and edge storage implementation

use std::collections::HashMap;

use crate::entity::{
    DirectedEdge, EntityType, EventRoomData, EventRoomEntityType, EventRoomToHotelRoomData,
    EventRoomToHotelRoomEntityType, HotelRoomData, HotelRoomEntityType, PanelData, PanelEntityType,
    PanelToEventRoomData, PanelToEventRoomEntityType, PanelToPanelTypeData,
    PanelToPanelTypeEntityType, PanelToPresenterData, PanelToPresenterEntityType, PanelTypeData,
    PanelTypeEntityType, PresenterData, PresenterEntityType, PresenterToGroupData,
    PresenterToGroupEntityType,
};
use uuid::NonNilUuid;

use super::edge_index::EdgeIndex;

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

    // Edge entities
    pub panel_to_presenters: HashMap<NonNilUuid, PanelToPresenterData>,
    pub panel_to_event_rooms: HashMap<NonNilUuid, PanelToEventRoomData>,
    pub event_room_to_hotel_rooms: HashMap<NonNilUuid, EventRoomToHotelRoomData>,
    pub panel_to_panel_types: HashMap<NonNilUuid, PanelToPanelTypeData>,
    pub presenter_to_groups: HashMap<NonNilUuid, PresenterToGroupData>,

    // Edge indexes — kept in sync by Schedule::add_edge / Schedule::remove_edge
    panel_to_presenter_index: EdgeIndex,
    panel_to_event_room_index: EdgeIndex,
    event_room_to_hotel_room_index: EdgeIndex,
    panel_to_panel_type_index: EdgeIndex,
    presenter_to_group_index: EdgeIndex,
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

impl TypedStorage for PanelToPresenterEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelToPresenterData> {
        &s.panel_to_presenters
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelToPresenterData> {
        &mut s.panel_to_presenters
    }
}

impl TypedStorage for PanelToEventRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelToEventRoomData> {
        &s.panel_to_event_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelToEventRoomData> {
        &mut s.panel_to_event_rooms
    }
}

impl TypedStorage for EventRoomToHotelRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, EventRoomToHotelRoomData> {
        &s.event_room_to_hotel_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, EventRoomToHotelRoomData> {
        &mut s.event_room_to_hotel_rooms
    }
}

impl TypedStorage for PanelToPanelTypeEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelToPanelTypeData> {
        &s.panel_to_panel_types
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelToPanelTypeData> {
        &mut s.panel_to_panel_types
    }
}

impl TypedStorage for PresenterToGroupEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PresenterToGroupData> {
        &s.presenter_to_groups
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PresenterToGroupData> {
        &mut s.presenter_to_groups
    }
}

/// Maps an edge entity type to its [`EdgeIndex`] within [`EntityStorage`].
///
/// This is the edge counterpart of [`TypedStorage`]: where `TypedStorage`
/// routes to the correct `HashMap`, `TypedEdgeStorage` routes to the correct
/// `EdgeIndex`.  Requires `Self::Data: DirectedEdge` so edge endpoints can
/// be extracted.
pub trait TypedEdgeStorage: TypedStorage
where
    Self::Data: DirectedEdge,
{
    fn edge_index(storage: &EntityStorage) -> &EdgeIndex;
    fn edge_index_mut(storage: &mut EntityStorage) -> &mut EdgeIndex;
}

impl TypedEdgeStorage for PanelToPresenterEntityType {
    fn edge_index(s: &EntityStorage) -> &EdgeIndex {
        &s.panel_to_presenter_index
    }
    fn edge_index_mut(s: &mut EntityStorage) -> &mut EdgeIndex {
        &mut s.panel_to_presenter_index
    }
}

impl TypedEdgeStorage for PanelToEventRoomEntityType {
    fn edge_index(s: &EntityStorage) -> &EdgeIndex {
        &s.panel_to_event_room_index
    }
    fn edge_index_mut(s: &mut EntityStorage) -> &mut EdgeIndex {
        &mut s.panel_to_event_room_index
    }
}

impl TypedEdgeStorage for EventRoomToHotelRoomEntityType {
    fn edge_index(s: &EntityStorage) -> &EdgeIndex {
        &s.event_room_to_hotel_room_index
    }
    fn edge_index_mut(s: &mut EntityStorage) -> &mut EdgeIndex {
        &mut s.event_room_to_hotel_room_index
    }
}

impl TypedEdgeStorage for PanelToPanelTypeEntityType {
    fn edge_index(s: &EntityStorage) -> &EdgeIndex {
        &s.panel_to_panel_type_index
    }
    fn edge_index_mut(s: &mut EntityStorage) -> &mut EdgeIndex {
        &mut s.panel_to_panel_type_index
    }
}

impl TypedEdgeStorage for PresenterToGroupEntityType {
    fn edge_index(s: &EntityStorage) -> &EdgeIndex {
        &s.presenter_to_group_index
    }
    fn edge_index_mut(s: &mut EntityStorage) -> &mut EdgeIndex {
        &mut s.presenter_to_group_index
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

    /// Add entity to storage with pre-allocated UUID.
    pub fn add_with_uuid<T: TypedStorage>(
        &mut self,
        uuid: NonNilUuid,
        entity: T::Data,
    ) -> Result<(), InsertError> {
        let map = T::typed_map_mut(self);
        if map.contains_key(&uuid) {
            return Err(InsertError::UuidCollision { uuid });
        }
        map.insert(uuid, entity);
        Ok(())
    }

    /// Check if entity with given UUID exists.
    pub fn contains_uuid<T: TypedStorage>(&self, uuid: NonNilUuid) -> bool {
        T::typed_map(self).contains_key(&uuid)
    }

    /// Get a mutable reference to an entity by type and UUID.
    pub fn get_mut<T: TypedStorage>(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data> {
        T::typed_map_mut(self).get_mut(&uuid)
    }

    /// Remove an entity by type and UUID, returning the removed data if it existed.
    pub fn remove<T: TypedStorage>(&mut self, uuid: NonNilUuid) -> Option<T::Data> {
        T::typed_map_mut(self).remove(&uuid)
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
impl<T: TypedStorage> EntityStore<T> for EntityStorage {
    fn get_entity(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        T::typed_map(self).get(&uuid)
    }

    fn get_entity_mut(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data> {
        T::typed_map_mut(self).get_mut(&uuid)
    }

    fn insert_entity(&mut self, uuid: NonNilUuid, data: T::Data) -> Result<(), InsertError> {
        let map = T::typed_map_mut(self);
        if map.contains_key(&uuid) {
            return Err(InsertError::UuidCollision { uuid });
        }
        map.insert(uuid, data);
        Ok(())
    }

    fn remove_entity(&mut self, uuid: NonNilUuid) -> Option<T::Data> {
        T::typed_map_mut(self).remove(&uuid)
    }

    fn contains_entity(&self, uuid: NonNilUuid) -> bool {
        T::typed_map(self).contains_key(&uuid)
    }
}
