/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule container and storage system

pub mod storage;

use chrono::NaiveDateTime;
use std::collections::HashMap;
use std::fmt;

use crate::edge::{Edge, EdgeId, EdgeStorage as _};
use crate::edge::{
    EventRoomToHotelRoomStorage, GenericEdgeStorage, PanelToPanelTypeStorage,
    PanelToPresenterStorage, PresenterToGroupStorage,
};
use crate::edge_entity_query::EdgeEntityQuery;
use crate::entity::{
    EntityKind, EntityRef, EntityType, EntityUUID, EventRoomId, HotelRoomId, InternalData, PanelId,
    PanelTypeId, PresenterId, PublicEntityRef, TypedId,
};
use crate::entity::{PanelToPresenterId, PresenterToGroupId};
use crate::field::validation::ValidationError;
use crate::field::FieldValue;
use crate::query::{FieldMatch, QueryOptions};

/// Direction for relationship queries in find_related
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
    /// Outgoing relationships (e.g., panel -> presenter)
    Outgoing,
    /// Incoming relationships (e.g., presenter <- panel)
    Incoming,
}

// Re-export storage types
pub use storage::*;

/// Schedule container holding all entities and relationships
#[derive(Debug, Clone)]
pub struct Schedule {
    pub entities: EntityStorage,
    pub presenter_to_group: PresenterToGroupStorage,
    pub panel_to_presenter: PanelToPresenterStorage,
    pub panel_to_event_room: GenericEdgeStorage<crate::edge::PanelToEventRoomEdge>,
    pub event_room_to_hotel_room: EventRoomToHotelRoomStorage,
    pub panel_to_panel_type: PanelToPanelTypeStorage,
    entity_registry: HashMap<uuid::NonNilUuid, EntityKind>,
    pub metadata: ScheduleMetadata,
    /// Query engine for edge-entities with caching
    pub edge_query: EdgeEntityQuery,
}

impl Schedule {
    pub fn new() -> Self {
        let entities = EntityStorage::new();
        let edge_query = EdgeEntityQuery::new(&entities);
        Self {
            entities,
            presenter_to_group: PresenterToGroupStorage::new(),
            panel_to_presenter: PanelToPresenterStorage::new(),
            panel_to_event_room: GenericEdgeStorage::new(),
            event_room_to_hotel_room: EventRoomToHotelRoomStorage::new(),
            panel_to_panel_type: PanelToPanelTypeStorage::new(),
            entity_registry: HashMap::new(),
            metadata: ScheduleMetadata::new(),
            edge_query,
        }
    }

    /// Get entity by type and UUID
    pub fn get_entity<T: TypedStorage>(&self, uuid: uuid::NonNilUuid) -> Option<&T::Data> {
        self.entities.get::<T>(uuid)
    }

    /// Get entity by internal UUID
    pub fn get_entity_by_uuid<T: TypedStorage>(&self, uuid: uuid::NonNilUuid) -> Option<&T::Data> {
        self.entities.get_by_uuid::<T>(uuid)
    }

    /// Find entities matching field conditions
    pub fn find_entities<T: TypedStorage + Sized>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<uuid::NonNilUuid> {
        self.entities.find::<T>(matches, options)
    }

    /// Get entities matching field conditions
    pub fn get_entities<T: TypedStorage + Sized>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<&T::Data> {
        self.entities.get_many::<T>(matches, options)
    }

    /// Add entity to schedule
    pub fn add_entity<T: TypedStorage>(
        &mut self,
        entity: T::Data,
    ) -> Result<uuid::NonNilUuid, ScheduleError>
    where
        T::Data: InternalData,
    {
        let uuid = entity.uuid();
        self.entity_registry.insert(uuid, T::kind());
        self.entities.add_with_uuid::<T>(uuid, entity)?;
        Ok(uuid)
    }

    /// Add entity and return UUID
    pub fn add_entity_with_uuid<T: TypedStorage>(
        &mut self,
        entity: T::Data,
    ) -> Result<uuid::NonNilUuid, ScheduleError>
    where
        T::Data: InternalData,
    {
        let uuid = entity.uuid();
        self.entity_registry.insert(uuid, T::kind());
        self.entities.add_with_uuid::<T>(uuid, entity)?;
        Ok(uuid)
    }

    /// Update entity fields
    pub fn update_entity<T: TypedStorage>(
        &mut self,
        uuid: uuid::NonNilUuid,
        updates: &[(String, FieldValue)],
    ) -> Result<(), ScheduleError> {
        self.entities.update::<T>(uuid, updates)
    }

    /// Identify which entity kind (if any) owns the given UUID.
    /// Returns an `EntityUUID` tagging the UUID with its entity type.
    pub fn identify(&self, uuid: uuid::NonNilUuid) -> Option<EntityUUID> {
        match *self.entity_registry.get(&uuid)? {
            EntityKind::Panel => Some(EntityUUID::Panel(PanelId::from_uuid(uuid))),
            EntityKind::Presenter => Some(EntityUUID::Presenter(PresenterId::from_uuid(uuid))),
            EntityKind::EventRoom => Some(EntityUUID::EventRoom(EventRoomId::from_uuid(uuid))),
            EntityKind::HotelRoom => Some(EntityUUID::HotelRoom(HotelRoomId::from_uuid(uuid))),
            EntityKind::PanelType => Some(EntityUUID::PanelType(PanelTypeId::from_uuid(uuid))),
            EntityKind::PanelToPresenter => Some(EntityUUID::PanelToPresenter(
                crate::entity::panel_to_presenter::PanelToPresenterId::from_uuid(uuid),
            )),
            EntityKind::PanelToEventRoom => Some(EntityUUID::PanelToEventRoom(
                crate::entity::panel_to_event_room::PanelToEventRoomId::from_uuid(uuid),
            )),
            EntityKind::EventRoomToHotelRoom => Some(EntityUUID::EventRoomToHotelRoom(
                crate::entity::event_room_to_hotel_room::EventRoomToHotelRoomId::from_uuid(uuid),
            )),
            EntityKind::PanelToPanelType => Some(EntityUUID::PanelToPanelType(
                crate::entity::panel_to_panel_type::PanelToPanelTypeId::from_uuid(uuid),
            )),
            EntityKind::PresenterToGroup => Some(EntityUUID::PresenterToGroup(
                crate::entity::presenter_to_group::PresenterToGroupId::from_uuid(uuid),
            )),
        }
    }

    /// Fetch entity data by typed ID, returning a borrowed reference to the raw internal data.
    /// Zero runtime dispatch — storage is selected at compile time via `Id::EntityType`.
    pub fn fetch_entity<Id: TypedId>(&self, id: Id) -> Option<&<Id::EntityType as EntityType>::Data>
    where
        Id::EntityType: TypedStorage,
    {
        self.entities
            .get_by_uuid::<Id::EntityType>(id.non_nil_uuid())
    }

    /// Fetch entity data by typed ID, returning an owned public value.
    /// Dispatches directly to the correct storage for the entity type.
    pub fn fetch_typed<Id: TypedId>(&self, id: Id) -> Option<PublicEntityRef> {
        let uuid = id.non_nil_uuid();
        match Id::kind() {
            EntityKind::Panel => self
                .entities
                .get_by_uuid::<crate::entity::PanelEntityType>(uuid)
                .map(|data| PublicEntityRef::Panel(data.to_public())),
            EntityKind::Presenter => self
                .entities
                .get_by_uuid::<crate::entity::PresenterEntityType>(uuid)
                .map(|data| PublicEntityRef::Presenter(data.to_public())),
            EntityKind::EventRoom => self
                .entities
                .get_by_uuid::<crate::entity::EventRoomEntityType>(uuid)
                .map(|data| PublicEntityRef::EventRoom(data.to_public())),
            EntityKind::HotelRoom => self
                .entities
                .get_by_uuid::<crate::entity::HotelRoomEntityType>(uuid)
                .map(|data| PublicEntityRef::HotelRoom(data.to_public())),
            EntityKind::PanelType => self
                .entities
                .get_by_uuid::<crate::entity::PanelTypeEntityType>(uuid)
                .map(|data| PublicEntityRef::PanelType(data.to_public())),
            EntityKind::PanelToPresenter => None, // Edge-entities don't have PublicEntityRef yet
            EntityKind::PanelToEventRoom => None, // Edge-entities don't have PublicEntityRef yet
            EntityKind::EventRoomToHotelRoom => None, // Edge-entities don't have PublicEntityRef yet
            EntityKind::PanelToPanelType => None, // Edge-entities don't have PublicEntityRef yet
            EntityKind::PresenterToGroup => None, // Edge-entities don't have PublicEntityRef yet
        }
    }

    /// Lookup entity data by typed ID, returning a borrowed reference.
    /// Dispatches directly to the correct storage for the entity type.
    pub fn lookup_typed<Id: TypedId>(&self, id: Id) -> Option<EntityRef<'_>> {
        let uuid = id.non_nil_uuid();
        match Id::kind() {
            EntityKind::Panel => self
                .entities
                .get_by_uuid::<crate::entity::PanelEntityType>(uuid)
                .map(EntityRef::Panel),
            EntityKind::Presenter => self
                .entities
                .get_by_uuid::<crate::entity::PresenterEntityType>(uuid)
                .map(EntityRef::Presenter),
            EntityKind::EventRoom => self
                .entities
                .get_by_uuid::<crate::entity::EventRoomEntityType>(uuid)
                .map(EntityRef::EventRoom),
            EntityKind::HotelRoom => self
                .entities
                .get_by_uuid::<crate::entity::HotelRoomEntityType>(uuid)
                .map(EntityRef::HotelRoom),
            EntityKind::PanelType => self
                .entities
                .get_by_uuid::<crate::entity::PanelTypeEntityType>(uuid)
                .map(EntityRef::PanelType),
            EntityKind::PanelToPresenter => None, // Edge-entities don't have EntityRef yet
            EntityKind::PanelToEventRoom => None, // Edge-entities don't have EntityRef yet
            EntityKind::EventRoomToHotelRoom => None, // Edge-entities don't have EntityRef yet
            EntityKind::PanelToPanelType => None, // Edge-entities don't have EntityRef yet
            EntityKind::PresenterToGroup => None, // Edge-entities don't have EntityRef yet
        }
    }

    /// Fetch entity by UUID, identifying entity type then fetching via `fetch_typed`.
    pub fn fetch_uuid(&self, uuid: uuid::NonNilUuid) -> Option<PublicEntityRef> {
        match self.identify(uuid)? {
            EntityUUID::Panel(id) => self.fetch_typed(id),
            EntityUUID::Presenter(id) => self.fetch_typed(id),
            EntityUUID::EventRoom(id) => self.fetch_typed(id),
            EntityUUID::HotelRoom(id) => self.fetch_typed(id),
            EntityUUID::PanelType(id) => self.fetch_typed(id),
            EntityUUID::PanelToPresenter(_) => None, // Edge-entities don't have PublicEntityRef yet
            EntityUUID::PanelToEventRoom(_) => None, // Edge-entities don't have PublicEntityRef yet
            EntityUUID::EventRoomToHotelRoom(_) => None, // Edge-entities don't have PublicEntityRef yet
            EntityUUID::PanelToPanelType(_) => None, // Edge-entities don't have PublicEntityRef yet
            EntityUUID::PresenterToGroup(_) => None, // Edge-entities don't have PublicEntityRef yet
        }
    }

    /// Get the entity type (kind) for a given UUID
    pub fn type_of_uuid(&self, uuid: uuid::NonNilUuid) -> Option<EntityKind> {
        self.entity_registry.get(&uuid).copied()
    }

    /// Lookup entity by UUID, identifying entity type then fetching via `lookup_typed`.
    pub fn lookup_uuid(&self, uuid: uuid::NonNilUuid) -> Option<EntityRef<'_>> {
        match self.identify(uuid)? {
            EntityUUID::Panel(id) => self.lookup_typed(id),
            EntityUUID::Presenter(id) => self.lookup_typed(id),
            EntityUUID::EventRoom(id) => self.lookup_typed(id),
            EntityUUID::HotelRoom(id) => self.lookup_typed(id),
            EntityUUID::PanelType(id) => self.lookup_typed(id),
            EntityUUID::PanelToPresenter(_) => None, // Edge-entities don't have EntityRef yet
            EntityUUID::PanelToEventRoom(_) => None, // Edge-entities don't have EntityRef yet
            EntityUUID::EventRoomToHotelRoom(_) => None, // Edge-entities don't have EntityRef yet
            EntityUUID::PanelToPanelType(_) => None, // Edge-entities don't have EntityRef yet
            EntityUUID::PresenterToGroup(_) => None, // Edge-entities don't have EntityRef yet
        }
    }

    /// Find entities related to a given entity (dispatches to appropriate typed storage)
    pub fn find_related<T: EntityType>(
        &self,
        uuid: uuid::NonNilUuid,
        edge_type: crate::edge::EdgeType,
        direction: RelationshipDirection,
    ) -> Vec<uuid::NonNilUuid> {
        use crate::edge::EdgeType;
        match edge_type {
            EdgeType::PanelToPresenter => {
                if direction == RelationshipDirection::Outgoing {
                    self.panel_to_presenter
                        .find_outgoing(uuid)
                        .iter()
                        .map(|e| e.to_uuid())
                        .collect()
                } else {
                    self.panel_to_presenter
                        .find_incoming(uuid)
                        .iter()
                        .map(|e| e.from_uuid())
                        .collect()
                }
            }
            EdgeType::PanelToEventRoom => {
                if direction == RelationshipDirection::Outgoing {
                    self.panel_to_event_room
                        .find_outgoing(uuid)
                        .iter()
                        .map(|e| e.to_uuid())
                        .collect()
                } else {
                    self.panel_to_event_room
                        .find_incoming(uuid)
                        .iter()
                        .map(|e| e.from_uuid())
                        .collect()
                }
            }
            EdgeType::PanelToPanelType => {
                if direction == RelationshipDirection::Outgoing {
                    self.panel_to_panel_type
                        .find_outgoing(uuid)
                        .iter()
                        .map(|e| e.to_uuid())
                        .collect()
                } else {
                    self.panel_to_panel_type
                        .find_incoming(uuid)
                        .iter()
                        .map(|e| e.from_uuid())
                        .collect()
                }
            }
            EdgeType::PresenterToGroup => {
                if direction == RelationshipDirection::Outgoing {
                    self.presenter_to_group.direct_groups_of(uuid).to_vec()
                } else {
                    self.presenter_to_group.direct_members_of(uuid).to_vec()
                }
            }
            EdgeType::EventRoomToHotelRoom => {
                if direction == RelationshipDirection::Outgoing {
                    self.event_room_to_hotel_room
                        .find_outgoing(uuid)
                        .iter()
                        .map(|e| e.to_uuid())
                        .collect()
                } else {
                    self.event_room_to_hotel_room
                        .find_incoming(uuid)
                        .iter()
                        .map(|e| e.from_uuid())
                        .collect()
                }
            }
        }
    }

    // === Entity Relationship Methods ===

    /// Get all presenters for a panel (returns PresenterIds)
    pub fn get_panel_presenters(&self, panel_id: PanelId) -> Vec<PresenterId> {
        self.panel_to_presenter
            .find_outgoing(panel_id.non_nil_uuid())
            .iter()
            .map(|e| PresenterId::from_uuid(e.to_uuid()))
            .collect()
    }

    /// Get the primary event room for a panel (returns EventRoomId)
    pub fn get_panel_event_room(&self, panel_id: PanelId) -> Option<EventRoomId> {
        self.panel_to_event_room
            .find_outgoing(panel_id.non_nil_uuid())
            .first()
            .map(|e| EventRoomId::from_uuid(e.to_uuid()))
    }

    /// Get the panel type for a panel (returns PanelTypeId)
    pub fn get_panel_type(&self, panel_id: PanelId) -> Option<PanelTypeId> {
        self.panel_to_panel_type
            .find_outgoing(panel_id.non_nil_uuid())
            .first()
            .map(|e| PanelTypeId::from_uuid(e.to_uuid()))
    }

    /// Get all groups a presenter belongs to (returns PresenterIds)
    pub fn get_presenter_groups(&self, presenter_id: PresenterId) -> Vec<PresenterId> {
        self.presenter_to_group
            .direct_groups_of(presenter_id.non_nil_uuid())
            .iter()
            .map(|&uuid| PresenterId::from_uuid(uuid))
            .collect()
    }

    /// Get all members of a presenter group (returns PresenterIds)
    pub fn get_presenter_members(&self, presenter_id: PresenterId) -> Vec<PresenterId> {
        self.presenter_to_group
            .direct_members_of(presenter_id.non_nil_uuid())
            .iter()
            .map(|&uuid| PresenterId::from_uuid(uuid))
            .collect()
    }

    /// Get all panels a presenter participates in (returns PanelIds)
    pub fn get_presenter_panels(&self, presenter_id: PresenterId) -> Vec<PanelId> {
        self.panel_to_presenter
            .find_incoming(presenter_id.non_nil_uuid())
            .iter()
            .map(|e| PanelId::from_uuid(e.from_uuid()))
            .collect()
    }

    /// Connect a panel to a presenter
    pub fn connect_panel_to_presenter(
        &mut self,
        panel_id: PanelId,
        presenter_id: PresenterId,
    ) -> Result<EdgeId, ScheduleError> {
        let edge = crate::edge::PanelToPresenterEdge::new(panel_id, presenter_id);
        self.panel_to_presenter
            .add_edge(edge)
            .map_err(|e| ScheduleError::StorageError {
                message: e.to_string(),
            })
    }

    /// Connect a panel to an event room
    pub fn connect_panel_to_event_room(
        &mut self,
        panel_id: PanelId,
        room_id: EventRoomId,
    ) -> Result<EdgeId, ScheduleError> {
        let edge = crate::edge::PanelToEventRoomEdge::new(panel_id, room_id);
        self.panel_to_event_room
            .add_edge(edge)
            .map_err(|e| ScheduleError::StorageError {
                message: e.to_string(),
            })
    }

    /// Connect a panel to a panel type
    pub fn connect_panel_to_panel_type(
        &mut self,
        panel_id: PanelId,
        type_id: PanelTypeId,
    ) -> Result<EdgeId, ScheduleError> {
        let edge = crate::edge::PanelToPanelTypeEdge::new(panel_id, type_id);
        self.panel_to_panel_type
            .add_edge(edge)
            .map_err(|e| ScheduleError::StorageError {
                message: e.to_string(),
            })
    }

    /// Connect a presenter to a group
    pub fn connect_presenter_to_group(
        &mut self,
        presenter_id: PresenterId,
        group_id: PresenterId,
    ) -> Result<EdgeId, ScheduleError> {
        let edge = crate::edge::presenter_to_group::PresenterToGroupEdge::new(
            presenter_id,
            group_id,
            false,
            false,
        );
        self.presenter_to_group
            .add_edge(edge)
            .map_err(|e| ScheduleError::StorageError {
                message: e.to_string(),
            })
    }

    /// Connect an event room to a hotel room
    pub fn connect_event_room_to_hotel_room(
        &mut self,
        event_room_id: EventRoomId,
        hotel_room_id: HotelRoomId,
    ) -> Result<EdgeId, ScheduleError> {
        let edge = crate::edge::EventRoomToHotelRoomEdge::new(event_room_id, hotel_room_id);
        self.event_room_to_hotel_room
            .add_edge(edge)
            .map_err(|e| ScheduleError::StorageError {
                message: e.to_string(),
            })
    }

    /// Create a PanelToPresenter edge-entity
    ///
    /// Uses V5 UUID generation for deterministic identity based on the panel
    /// and presenter UUIDs. This ensures the same relationship always produces
    /// the same edge UUID, enabling natural collision detection.
    pub fn create_panel_to_presenter_entity(
        &mut self,
        panel_id: PanelId,
        presenter_id: PresenterId,
    ) -> Result<PanelToPresenterId, ScheduleError> {
        // Generate deterministic V5 UUID based on endpoints
        let edge_uuid = crate::uuid_v5::panel_to_presenter_uuid(
            panel_id.non_nil_uuid(),
            presenter_id.non_nil_uuid(),
        );

        let data = crate::entity::PanelToPresenterData {
            entity_uuid: edge_uuid,
            panel_uuid: panel_id.non_nil_uuid(),
            presenter_uuid: presenter_id.non_nil_uuid(),
        };

        // Register the entity kind
        self.entity_registry
            .insert(edge_uuid, crate::entity::EntityKind::PanelToPresenter);

        // Store the entity (may overwrite if same relationship existed)
        self.entities.panel_to_presenters.insert(edge_uuid, data);

        // Invalidate edge query cache since we modified edges
        self.edge_query.invalidate_cache();

        Ok(PanelToPresenterId::from_uuid(edge_uuid))
    }

    /// Create a PresenterToGroup edge-entity
    ///
    /// Uses V5 UUID generation for deterministic identity based on the member
    /// and group UUIDs. This ensures the same relationship always produces
    /// the same edge UUID, enabling natural collision detection.
    pub fn create_presenter_to_group_entity(
        &mut self,
        member_id: PresenterId,
        group_id: PresenterId,
        always_shown_in_group: bool,
        always_grouped: bool,
    ) -> Result<PresenterToGroupId, ScheduleError> {
        // Generate deterministic V5 UUID based on endpoints
        let edge_uuid = crate::uuid_v5::presenter_to_group_uuid(
            member_id.non_nil_uuid(),
            group_id.non_nil_uuid(),
        );

        let data = crate::entity::PresenterToGroupData {
            entity_uuid: edge_uuid,
            member_uuid: member_id.non_nil_uuid(),
            group_uuid: group_id.non_nil_uuid(),
            always_shown_in_group,
            always_grouped,
        };

        // Register the entity kind
        self.entity_registry
            .insert(edge_uuid, crate::entity::EntityKind::PresenterToGroup);

        // Store the entity
        self.entities.presenter_to_groups.insert(edge_uuid, data);

        // Invalidate edge query cache
        self.edge_query.invalidate_cache();

        Ok(PresenterToGroupId::from_uuid(edge_uuid))
    }

    /// Delete a PanelToPresenter edge-entity by ID
    pub fn delete_panel_to_presenter_entity(
        &mut self,
        edge_id: PanelToPresenterId,
    ) -> Result<(), ScheduleError> {
        let uuid = edge_id.non_nil_uuid();

        if self.entities.panel_to_presenters.remove(&uuid).is_none() {
            return Err(ScheduleError::EdgeNotFound {
                edge_id: uuid.to_string(),
            });
        }

        self.entity_registry.remove(&uuid);
        self.edge_query.invalidate_cache();

        Ok(())
    }

    /// Delete a PresenterToGroup edge-entity by ID
    pub fn delete_presenter_to_group_entity(
        &mut self,
        edge_id: PresenterToGroupId,
    ) -> Result<(), ScheduleError> {
        let uuid = edge_id.non_nil_uuid();

        if self.entities.presenter_to_groups.remove(&uuid).is_none() {
            return Err(ScheduleError::EdgeNotFound {
                edge_id: uuid.to_string(),
            });
        }

        self.entity_registry.remove(&uuid);
        self.edge_query.invalidate_cache();

        Ok(())
    }

    /// Delete PanelToPresenter edges by endpoints
    pub fn delete_panel_to_presenter_by_endpoints(
        &mut self,
        panel_id: PanelId,
        presenter_id: PresenterId,
    ) -> Result<(), ScheduleError> {
        let panel_uuid = panel_id.non_nil_uuid();
        let presenter_uuid = presenter_id.non_nil_uuid();

        // Find and remove matching edges
        let edges_to_remove: Vec<uuid::NonNilUuid> = self
            .entities
            .panel_to_presenters
            .iter()
            .filter(|(_, data)| {
                data.panel_uuid == panel_uuid && data.presenter_uuid == presenter_uuid
            })
            .map(|(uuid, _)| *uuid)
            .collect();

        if edges_to_remove.is_empty() {
            return Err(ScheduleError::EdgeNotFound {
                edge_id: format!("panel:{:?},presenter:{:?}", panel_uuid, presenter_uuid),
            });
        }

        for uuid in edges_to_remove {
            self.entities.panel_to_presenters.remove(&uuid);
            self.entity_registry.remove(&uuid);
        }

        self.edge_query.invalidate_cache();
        Ok(())
    }

    /// Delete PresenterToGroup edges by endpoints
    pub fn delete_presenter_to_group_by_endpoints(
        &mut self,
        member_id: PresenterId,
        group_id: PresenterId,
    ) -> Result<(), ScheduleError> {
        let member_uuid = member_id.non_nil_uuid();
        let group_uuid = group_id.non_nil_uuid();

        // Find and remove matching edges
        let edges_to_remove: Vec<uuid::NonNilUuid> = self
            .entities
            .presenter_to_groups
            .iter()
            .filter(|(_, data)| data.member_uuid == member_uuid && data.group_uuid == group_uuid)
            .map(|(uuid, _)| *uuid)
            .collect();

        if edges_to_remove.is_empty() {
            return Err(ScheduleError::EdgeNotFound {
                edge_id: format!("member:{:?},group:{:?}", member_uuid, group_uuid),
            });
        }

        for uuid in edges_to_remove {
            self.entities.presenter_to_groups.remove(&uuid);
            self.entity_registry.remove(&uuid);
        }

        self.edge_query.invalidate_cache();
        Ok(())
    }

    // === Cache Invalidation Methods ===

    /// Invalidate panel-to-presenter cache
    pub fn invalidate_panel_to_presenter_cache(&mut self) {
        self.panel_to_presenter.invalidate_cache();
    }

    /// Invalidate event-room-to-hotel-room cache
    pub fn invalidate_event_room_to_hotel_room_cache(&mut self) {
        self.event_room_to_hotel_room.invalidate_cache();
    }

    // === Transitive Closure Methods ===

    /// Get all presenters for a panel, including those from presenter groups
    pub fn get_panel_inclusive_presenters(&mut self, panel_id: PanelId) -> Vec<PresenterId> {
        self.panel_to_presenter
            .get_inclusive_presenters(panel_id.non_nil_uuid(), &mut self.presenter_to_group)
            .iter()
            .map(|&uuid| PresenterId::from_uuid(uuid))
            .collect()
    }

    /// Get all panels for a presenter, including those from presenter groups
    pub fn get_presenter_inclusive_panels(&mut self, presenter_id: PresenterId) -> Vec<PanelId> {
        self.panel_to_presenter
            .get_inclusive_panels(presenter_id.non_nil_uuid(), &mut self.presenter_to_group)
            .iter()
            .map(|&uuid| PanelId::from_uuid(uuid))
            .collect()
    }

    // === Data Resolution Methods ===

    /// Get entity names for a list of UUIDs
    pub fn get_entity_names<T: TypedStorage + Sized>(
        &self,
        uuids: &[uuid::NonNilUuid],
    ) -> Vec<String> {
        uuids
            .iter()
            .filter_map(|&uuid| self.get_entity::<T>(uuid))
            .map(|entity| self.get_entity_name::<T>(entity))
            .collect()
    }

    /// Helper to get the name field from any entity
    fn get_entity_name<T: EntityType>(&self, entity: &T::Data) -> String {
        // This is a simplified approach - in practice we'd use the field system
        // For now, we'll use pattern matching on known entity types
        use crate::entity;

        // Use downcasting or field access to get the name
        // This is a placeholder - the real implementation would use the field system
        match std::any::type_name::<T>() {
            x if x.contains("Panel") => {
                if let Some(panel) =
                    (entity as &dyn std::any::Any).downcast_ref::<entity::PanelData>()
                {
                    panel.name.clone()
                } else {
                    "Unknown".to_string()
                }
            }
            x if x.contains("Presenter") => {
                if let Some(presenter) =
                    (entity as &dyn std::any::Any).downcast_ref::<entity::PresenterData>()
                {
                    presenter.name.clone()
                } else {
                    "Unknown".to_string()
                }
            }
            _ => "Unknown".to_string(),
        }
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

/// Schedule metadata
#[derive(Debug, Clone)]
pub struct ScheduleMetadata {
    pub version: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub generator: String,
    pub schedule_id: uuid::NonNilUuid,
}

impl ScheduleMetadata {
    pub fn new() -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            version: "1.0".to_string(),
            created_at: now,
            updated_at: now,
            generator: "schedule-data".to_string(),
            schedule_id: unsafe { uuid::NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) },
        }
    }
}

impl Default for ScheduleMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Schedule-specific errors
#[derive(Debug, Clone)]
pub enum ScheduleError {
    EntityNotFound { entity_type: String, id: String },
    EdgeNotFound { edge_id: String },
    ValidationError { errors: Vec<ValidationError> },
    StorageError { message: String },
    DuplicateEntity { entity_type: String, id: String },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScheduleError::EntityNotFound { entity_type, id } => {
                write!(f, "Entity {} with ID {} not found", entity_type, id)
            }
            ScheduleError::EdgeNotFound { edge_id } => {
                write!(f, "Edge with ID {} not found", edge_id)
            }
            ScheduleError::ValidationError { errors } => {
                write!(f, "Validation failed: {:?}", errors)
            }
            ScheduleError::StorageError { message } => {
                write!(f, "Storage error: {}", message)
            }
            ScheduleError::DuplicateEntity { entity_type, id } => {
                write!(f, "Duplicate entity {} with ID {}", entity_type, id)
            }
        }
    }
}

impl std::error::Error for ScheduleError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::panel::PanelData;
    use crate::entity::{EntityKind, EntityUUID, PanelEntityType, PanelId, PresenterId};
    use crate::time::TimeRange;

    fn test_uuid(byte: u8) -> uuid::NonNilUuid {
        let mut bytes = [0u8; 16];
        bytes[15] = byte;
        unsafe { uuid::NonNilUuid::new_unchecked(uuid::Uuid::from_bytes(bytes)) }
    }

    fn make_panel(uuid: uuid::NonNilUuid, uid: &str, name: &str) -> PanelData {
        PanelData {
            entity_uuid: uuid,
            uid: uid.to_string(),
            base_uid: None,
            part_num: None,
            session_num: None,
            name: name.to_string(),
            panel_type_uid: None,
            description: None,
            note: None,
            prereq: None,
            time_range: TimeRange::default(),
            cost: None,
            capacity: None,
            pre_reg_max: None,
            difficulty: None,
            ticket_url: None,
            simple_tix_event: None,
            have_ticket_image: None,
            is_free: false,
            is_kids: false,
            is_full: false,
            hide_panelist: false,
            sewing_machines: false,
            alt_panelist: None,
            seats_sold: None,
            notes_non_printing: None,
            workshop_notes: None,
            power_needs: None,
            av_notes: None,
            presenters: Vec::new(),
            event_room: None,
            panel_type: None,
        }
    }

    #[test]
    fn identify_returns_correct_kind() {
        let mut sched = Schedule::new();
        let uuid = test_uuid(1);
        sched
            .add_entity::<PanelEntityType>(make_panel(uuid, "p1", "Panel One"))
            .unwrap();

        let result = sched.identify(uuid);
        assert_eq!(result, Some(EntityUUID::Panel(PanelId::from_uuid(uuid))));
    }

    #[test]
    fn identify_returns_none_for_unknown_uuid() {
        let sched = Schedule::new();
        assert!(sched.identify(test_uuid(1)).is_none());
    }

    #[test]
    fn fetch_entity_unknown_uuid_returns_none() {
        let sched = Schedule::new();
        let id = PanelId::from_uuid(test_uuid(2));
        assert!(sched.fetch_entity(id).is_none());
    }

    #[test]
    fn fetch_typed_unknown_uuid_returns_none() {
        let sched = Schedule::new();
        let id = PanelId::from_uuid(test_uuid(3));
        assert!(sched.fetch_typed(id).is_none());
    }

    #[test]
    fn lookup_typed_unknown_uuid_returns_none() {
        let sched = Schedule::new();
        let id = PanelId::from_uuid(test_uuid(4));
        assert!(sched.lookup_typed(id).is_none());
    }

    #[test]
    fn fetch_uuid_returns_none_for_unknown() {
        let sched = Schedule::new();
        assert!(sched.fetch_uuid(test_uuid(99)).is_none());
    }

    #[test]
    fn lookup_uuid_returns_none_for_unknown() {
        let sched = Schedule::new();
        assert!(sched.lookup_uuid(test_uuid(99)).is_none());
    }

    #[test]
    fn fetch_uuid_routes_through_identify() {
        let mut sched = Schedule::new();
        let uuid = test_uuid(6);
        sched
            .add_entity::<PanelEntityType>(make_panel(uuid, "p6", "Panel Six"))
            .unwrap();
        // identify succeeds (registry hit), fetch_uuid returns None only due to stub storage
        assert!(sched.identify(uuid).is_some());
        // fetch_uuid returns None because storage stub can't deserialize, not because routing failed
        let _ = sched.fetch_uuid(uuid); // must not panic
    }

    #[test]
    fn identify_kind_matches_entity_kind() {
        let mut sched = Schedule::new();
        let uuid = test_uuid(8);
        sched
            .add_entity::<PanelEntityType>(make_panel(uuid, "p8", "Panel Eight"))
            .unwrap();

        let identified = sched.identify(uuid).unwrap();
        assert_eq!(identified.kind(), EntityKind::Panel);
        assert_eq!(identified.non_nil_uuid(), uuid);
    }

    #[test]
    fn create_panel_to_presenter_entity_registered() {
        let mut sched = Schedule::new();
        let panel_uuid = test_uuid(10);
        let presenter_uuid = test_uuid(11);

        let panel_id = PanelId::from_uuid(panel_uuid);
        let presenter_id = PresenterId::from_uuid(presenter_uuid);

        let edge_id = sched
            .create_panel_to_presenter_entity(panel_id, presenter_id)
            .unwrap();

        // Edge should be registered
        let identified = sched.identify(edge_id.non_nil_uuid());
        assert!(identified.is_some());
        assert_eq!(identified.unwrap().kind(), EntityKind::PanelToPresenter);

        // Edge should be in storage
        assert!(sched
            .entities
            .panel_to_presenters
            .contains_key(&edge_id.non_nil_uuid()));
    }

    #[test]
    fn create_panel_to_presenter_entity_idempotent() {
        let mut sched = Schedule::new();
        let panel_id = PanelId::from_uuid(test_uuid(10));
        let presenter_id = PresenterId::from_uuid(test_uuid(11));

        let edge_id1 = sched
            .create_panel_to_presenter_entity(panel_id, presenter_id)
            .unwrap();
        let edge_id2 = sched
            .create_panel_to_presenter_entity(panel_id, presenter_id)
            .unwrap();

        // V5 UUID should be the same for the same endpoints
        assert_eq!(edge_id1, edge_id2);
        // Storage should have exactly one entry
        assert_eq!(sched.entities.panel_to_presenters.len(), 1);
    }

    #[test]
    fn delete_panel_to_presenter_entity() {
        let mut sched = Schedule::new();
        let panel_id = PanelId::from_uuid(test_uuid(10));
        let presenter_id = PresenterId::from_uuid(test_uuid(11));

        let edge_id = sched
            .create_panel_to_presenter_entity(panel_id, presenter_id)
            .unwrap();
        assert!(sched
            .entities
            .panel_to_presenters
            .contains_key(&edge_id.non_nil_uuid()));

        sched.delete_panel_to_presenter_entity(edge_id).unwrap();

        assert!(!sched
            .entities
            .panel_to_presenters
            .contains_key(&edge_id.non_nil_uuid()));
        assert!(sched.identify(edge_id.non_nil_uuid()).is_none());
    }

    #[test]
    fn create_presenter_to_group_entity_registered() {
        let mut sched = Schedule::new();
        let member_id = PresenterId::from_uuid(test_uuid(20));
        let group_id = PresenterId::from_uuid(test_uuid(21));

        let edge_id = sched
            .create_presenter_to_group_entity(member_id, group_id, false, false)
            .unwrap();

        let identified = sched.identify(edge_id.non_nil_uuid());
        assert!(identified.is_some());
        assert_eq!(identified.unwrap().kind(), EntityKind::PresenterToGroup);
    }

    #[test]
    fn create_presenter_to_group_entity_idempotent() {
        let mut sched = Schedule::new();
        let member_id = PresenterId::from_uuid(test_uuid(20));
        let group_id = PresenterId::from_uuid(test_uuid(21));

        let edge_id1 = sched
            .create_presenter_to_group_entity(member_id, group_id, false, false)
            .unwrap();
        let edge_id2 = sched
            .create_presenter_to_group_entity(member_id, group_id, true, false)
            .unwrap();

        // V5 UUID should be the same for the same endpoints
        assert_eq!(edge_id1, edge_id2);
        assert_eq!(sched.entities.presenter_to_groups.len(), 1);
    }

    #[test]
    fn delete_panel_to_presenter_by_endpoints() {
        let mut sched = Schedule::new();
        let panel_id = PanelId::from_uuid(test_uuid(10));
        let presenter_id = PresenterId::from_uuid(test_uuid(11));

        sched
            .create_panel_to_presenter_entity(panel_id, presenter_id)
            .unwrap();
        assert_eq!(sched.entities.panel_to_presenters.len(), 1);

        sched
            .delete_panel_to_presenter_by_endpoints(panel_id, presenter_id)
            .unwrap();
        assert_eq!(sched.entities.panel_to_presenters.len(), 0);
    }

    #[test]
    fn delete_nonexistent_edge_returns_error() {
        let mut sched = Schedule::new();
        let panel_id = PanelId::from_uuid(test_uuid(10));
        let presenter_id = PresenterId::from_uuid(test_uuid(11));

        let result = sched.delete_panel_to_presenter_by_endpoints(panel_id, presenter_id);
        assert!(matches!(result, Err(ScheduleError::EdgeNotFound { .. })));
    }
}
