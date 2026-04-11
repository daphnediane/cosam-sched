/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity and edge storage implementation

use std::collections::HashMap;

use crate::entity::{
    DirectedEdge, EntityKind, EntityType, EventRoomData, EventRoomEntityType,
    EventRoomToHotelRoomData, EventRoomToHotelRoomEntityType, HotelRoomData, HotelRoomEntityType,
    InternalData, PanelData, PanelEntityType, PanelToEventRoomData, PanelToEventRoomEntityType,
    PanelToPanelTypeData, PanelToPanelTypeEntityType, PanelToPresenterData,
    PanelToPresenterEntityType, PanelTypeData, PanelTypeEntityType, PresenterData,
    PresenterEntityType, PresenterToGroupData, PresenterToGroupEntityType, TypedId,
};
use uuid::NonNilUuid;

use super::edge_index::EdgeIndex;

/// Error type for entity insertion conflicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertError {
    /// UUID already exists in storage.
    UuidCollision { uuid: NonNilUuid },
    /// An edge between the same endpoint pair already exists and the
    /// applicable [`EdgePolicy`] is [`EdgePolicy::Reject`].
    DuplicateEdge { left: NonNilUuid, right: NonNilUuid },
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
            InsertError::DuplicateEdge { left, right } => {
                write!(
                    f,
                    "duplicate edge: an edge from {left} to {right} already exists"
                )
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

    // Edge indexes — kept in sync by add_edge / remove_edge
    panel_to_presenter_index: EdgeIndex,
    panel_to_event_room_index: EdgeIndex,
    event_room_to_hotel_room_index: EdgeIndex,
    panel_to_panel_type_index: EdgeIndex,
    presenter_to_group_index: EdgeIndex,

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

/// Policy applied when `Schedule::add_edge` encounters an existing edge
/// between the **same endpoint pair** (same `left_uuid` and `right_uuid`).
///
/// UUID collisions (same edge UUID, different endpoints) are always an error
/// regardless of policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgePolicy {
    /// Reject the duplicate — return `Err(InsertError::DuplicateEdge)`.
    #[default]
    Reject,
    /// Silently drop the new edge; keep the existing one unchanged.
    Ignore,
    /// Remove the existing edge (by endpoint match) and insert the new one.
    Replace,
}

/// Error type for a duplicate endpoint pair when `EdgePolicy::Reject` is active.
/// Included as a variant of [`InsertError`].
impl InsertError {
    /// Convenience constructor for a duplicate-edge error.
    pub fn duplicate_edge(left: NonNilUuid, right: NonNilUuid) -> Self {
        InsertError::DuplicateEdge { left, right }
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

    /// Policy applied when a new edge has the same endpoint pair as an
    /// existing edge of this type.  Defaults to [`EdgePolicy::Reject`].
    fn default_edge_policy() -> EdgePolicy {
        EdgePolicy::Reject
    }
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

    /// Add entity to storage with pre-allocated UUID, registering it in the UUID registry.
    pub fn add_with_uuid<T: TypedStorage>(
        &mut self,
        uuid: NonNilUuid,
        entity: T::Data,
    ) -> Result<(), InsertError> {
        EntityStore::<T>::insert_entity(self, uuid, entity)
    }

    /// Insert a new entity from its data struct, registering it in the UUID registry.
    ///
    /// The UUID is taken from `data.uuid()`. Returns the typed ID on success.
    pub fn add_entity<T: TypedStorage>(&mut self, data: T::Data) -> Result<T::Id, InsertError> {
        let uuid = data.uuid();
        EntityStore::<T>::insert_entity(self, uuid, data)?;
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
    pub fn remove<T: TypedStorage>(&mut self, uuid: NonNilUuid) -> Option<T::Data> {
        EntityStore::<T>::remove_entity(self, uuid)
    }

    // -----------------------------------------------------------------------
    // Edge entity CRUD (maintains EdgeIndex alongside entity storage)
    // -----------------------------------------------------------------------

    /// Add an edge entity and update the edge index.
    ///
    /// Applies the edge type's [`TypedEdgeStorage::default_edge_policy`] when
    /// the same endpoint pair already has an edge.  UUID collisions are always
    /// an error regardless of policy.
    pub fn add_edge<T>(&mut self, data: T::Data) -> Result<T::Id, InsertError>
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        self.add_edge_with_policy::<T>(data, T::default_edge_policy())
    }

    /// Add an edge entity using the specified [`EdgePolicy`] for duplicate
    /// endpoint handling, overriding the type's default.
    pub fn add_edge_with_policy<T>(
        &mut self,
        data: T::Data,
        policy: EdgePolicy,
    ) -> Result<T::Id, InsertError>
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        let uuid = data.uuid();
        let left_uuid = data.left_uuid();
        let right_uuid = data.right_uuid();

        // Early UUID collision check (before any mutations)
        let existing_kind = self.uuid_registry.get(&uuid).copied();
        if let Some(kind) = existing_kind {
            if kind != T::KIND {
                return Err(InsertError::UuidCollision { uuid });
            }
            if T::typed_map(self).contains_key(&uuid) {
                return Err(InsertError::UuidCollision { uuid });
            }
        }

        // Find existing edge with same endpoints
        let outgoing_uuids: Vec<NonNilUuid> = T::edge_index(self).outgoing(left_uuid).to_vec();
        let existing_edge_uuid: Option<NonNilUuid> = {
            let map = T::typed_map(self);
            outgoing_uuids.iter().copied().find(|&edge_uuid| {
                map.get(&edge_uuid)
                    .is_some_and(|d| d.right_uuid() == right_uuid)
            })
        };

        if let Some(existing_uuid) = existing_edge_uuid {
            match policy {
                EdgePolicy::Reject => {
                    return Err(InsertError::DuplicateEdge {
                        left: left_uuid,
                        right: right_uuid,
                    });
                }
                EdgePolicy::Ignore => {
                    return Ok(T::Id::from_uuid(existing_uuid));
                }
                EdgePolicy::Replace => {
                    T::edge_index_mut(self).remove(left_uuid, right_uuid, existing_uuid);
                    EntityStore::<T>::remove_entity(self, existing_uuid);
                }
            }
        }

        EntityStore::<T>::insert_entity(self, uuid, data)?;
        T::edge_index_mut(self).add(left_uuid, right_uuid, uuid);
        Ok(T::Id::from_uuid(uuid))
    }

    /// Remove an edge entity and update the edge index.
    ///
    /// Returns the edge data if it existed. Both the UUID registry and the
    /// [`EdgeIndex`] are cleaned up.
    pub fn remove_edge<T>(&mut self, id: T::Id) -> Option<T::Data>
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        let uuid = id.non_nil_uuid();
        let data = EntityStore::<T>::remove_entity(self, uuid)?;
        T::edge_index_mut(self).remove(data.left_uuid(), data.right_uuid(), uuid);
        Some(data)
    }

    // -----------------------------------------------------------------------
    // Edge queries
    // -----------------------------------------------------------------------

    /// Edge entity UUIDs leaving `from` for edge type `T`.
    pub fn edge_uuids_from<T>(&self, from: NonNilUuid) -> &[NonNilUuid]
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(self).outgoing(from)
    }

    /// Edge entity UUIDs arriving at `to` for edge type `T`.
    pub fn edge_uuids_to<T>(&self, to: NonNilUuid) -> &[NonNilUuid]
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(self).incoming(to)
    }

    /// Resolved edge data for all edges leaving `from`.
    pub fn edges_from<T>(&self, from: NonNilUuid) -> Vec<&T::Data>
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(self)
            .outgoing(from)
            .iter()
            .filter_map(|&edge_uuid| self.get::<T>(edge_uuid))
            .collect()
    }

    /// Resolved edge data for all edges arriving at `to`.
    pub fn edges_to<T>(&self, to: NonNilUuid) -> Vec<&T::Data>
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(self)
            .incoming(to)
            .iter()
            .filter_map(|&edge_uuid| self.get::<T>(edge_uuid))
            .collect()
    }

    /// Check whether an edge of type `T` exists between `from` and `to`.
    pub fn edge_exists<T>(&self, from: NonNilUuid, to: NonNilUuid) -> bool
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        let map = T::typed_map(self);
        T::edge_index(self).outgoing(from).iter().any(|&edge_uuid| {
            map.get(&edge_uuid)
                .is_some_and(|data| data.right_uuid() == to)
        })
    }

    /// Number of edges of type `T` currently stored.
    pub fn edge_count<T>(&self) -> usize
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(self).len()
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

/// Trait for edge entity types that manage their own edges within [`EntityStorage`].
///
/// Provides default `add_edge`, `add_edge_with_policy`, and `remove_edge` methods
/// that route through [`EntityStorage`] and maintain both the entity map and the
/// [`EdgeIndex`]. Implement this as an empty marker impl on each edge entity type.
pub trait EdgeEntityType: TypedEdgeStorage
where
    Self::Data: DirectedEdge,
{
    fn add_edge(storage: &mut EntityStorage, data: Self::Data) -> Result<Self::Id, InsertError>
    where
        Self: Sized,
    {
        storage.add_edge::<Self>(data)
    }

    fn add_edge_with_policy(
        storage: &mut EntityStorage,
        data: Self::Data,
        policy: EdgePolicy,
    ) -> Result<Self::Id, InsertError>
    where
        Self: Sized,
    {
        storage.add_edge_with_policy::<Self>(data, policy)
    }

    fn remove_edge(storage: &mut EntityStorage, id: Self::Id) -> Option<Self::Data>
    where
        Self: Sized,
    {
        storage.remove_edge::<Self>(id)
    }
}

impl EdgeEntityType for PanelToPresenterEntityType {}
impl EdgeEntityType for PanelToEventRoomEntityType {}
impl EdgeEntityType for EventRoomToHotelRoomEntityType {}
impl EdgeEntityType for PanelToPanelTypeEntityType {}
impl EdgeEntityType for PresenterToGroupEntityType {}
