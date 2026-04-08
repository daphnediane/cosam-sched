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
use crate::entity::{
    EntityKind, EntityType, EventRoomId, HotelRoomId, InternalData, PanelId, PanelTypeId,
    PresenterId, PublicEntityRef,
};
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
    entity_registry: HashMap<uuid::Uuid, EntityKind>,
    pub metadata: ScheduleMetadata,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            entities: EntityStorage::new(),
            presenter_to_group: PresenterToGroupStorage::new(),
            panel_to_presenter: PanelToPresenterStorage::new(),
            panel_to_event_room: GenericEdgeStorage::new(),
            event_room_to_hotel_room: EventRoomToHotelRoomStorage::new(),
            panel_to_panel_type: PanelToPanelTypeStorage::new(),
            entity_registry: HashMap::new(),
            metadata: ScheduleMetadata::new(),
        }
    }

    /// Get entity by type and UUID
    pub fn get_entity<T: EntityType>(&self, uuid: uuid::Uuid) -> Option<&T::Data> {
        self.entities.get::<T>(uuid)
    }

    /// Get entity by internal UUID
    pub fn get_entity_by_uuid<T: EntityType>(&self, uuid: uuid::Uuid) -> Option<&T::Data> {
        self.entities.get_by_uuid::<T>(uuid)
    }

    /// Find entities matching field conditions
    pub fn find_entities<T: EntityType>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<uuid::Uuid> {
        self.entities.find::<T>(matches, options)
    }

    /// Get entities matching field conditions
    pub fn get_entities<T: EntityType>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<&T::Data> {
        self.entities.get_many::<T>(matches, options)
    }

    /// Add entity to schedule
    pub fn add_entity<T: EntityType>(
        &mut self,
        entity: T::Data,
    ) -> Result<uuid::Uuid, ScheduleError>
    where
        T::Data: InternalData,
    {
        let uuid = entity.uuid();
        self.entity_registry.insert(uuid, T::kind());
        self.entities.add_with_uuid::<T>(uuid, entity)?;
        Ok(uuid)
    }

    /// Add entity and return UUID
    pub fn add_entity_with_uuid<T: EntityType>(
        &mut self,
        entity: T::Data,
    ) -> Result<uuid::Uuid, ScheduleError>
    where
        T::Data: InternalData,
    {
        let uuid = entity.uuid();
        self.entity_registry.insert(uuid, T::kind());
        self.entities.add_with_uuid::<T>(uuid, entity)?;
        Ok(uuid)
    }

    /// Update entity fields
    pub fn update_entity<T: EntityType>(
        &mut self,
        uuid: uuid::Uuid,
        updates: &[(String, FieldValue)],
    ) -> Result<(), ScheduleError> {
        self.entities.update::<T>(uuid, updates)
    }

    /// Fetch entity by UUID and return public reference
    pub fn fetch_uuid(&self, uuid: uuid::Uuid) -> Option<PublicEntityRef> {
        match self.entity_registry.get(&uuid)? {
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
        }
    }

    /// Get the entity type (kind) for a given UUID
    pub fn type_of_uuid(&self, uuid: uuid::Uuid) -> Option<EntityKind> {
        self.entity_registry.get(&uuid).copied()
    }

    /// Lookup entity by UUID and return borrowed reference
    pub fn lookup_uuid(&self, uuid: uuid::Uuid) -> Option<crate::entity::EntityRef<'_>> {
        match self.entity_registry.get(&uuid)? {
            EntityKind::Panel => self
                .entities
                .get_by_uuid::<crate::entity::PanelEntityType>(uuid)
                .map(crate::entity::EntityRef::Panel),
            EntityKind::Presenter => self
                .entities
                .get_by_uuid::<crate::entity::PresenterEntityType>(uuid)
                .map(crate::entity::EntityRef::Presenter),
            EntityKind::EventRoom => self
                .entities
                .get_by_uuid::<crate::entity::EventRoomEntityType>(uuid)
                .map(crate::entity::EntityRef::EventRoom),
            EntityKind::HotelRoom => self
                .entities
                .get_by_uuid::<crate::entity::HotelRoomEntityType>(uuid)
                .map(crate::entity::EntityRef::HotelRoom),
            EntityKind::PanelType => self
                .entities
                .get_by_uuid::<crate::entity::PanelTypeEntityType>(uuid)
                .map(crate::entity::EntityRef::PanelType),
        }
    }

    /// Find entities related to a given entity (dispatches to appropriate typed storage)
    pub fn find_related<T: EntityType>(
        &self,
        uuid: uuid::Uuid,
        edge_type: crate::edge::EdgeType,
        direction: RelationshipDirection,
    ) -> Vec<uuid::Uuid> {
        use crate::edge::EdgeType;
        match edge_type {
            EdgeType::PanelToPresenter => {
                if direction == RelationshipDirection::Outgoing {
                    self.panel_to_presenter
                        .find_outgoing(uuid)
                        .iter()
                        .filter_map(|e| e.to_uuid())
                        .collect()
                } else {
                    self.panel_to_presenter
                        .find_incoming(uuid)
                        .iter()
                        .filter_map(|e| e.from_uuid())
                        .collect()
                }
            }
            EdgeType::PanelToEventRoom => {
                if direction == RelationshipDirection::Outgoing {
                    self.panel_to_event_room
                        .find_outgoing(uuid)
                        .iter()
                        .filter_map(|e| e.to_uuid())
                        .collect()
                } else {
                    self.panel_to_event_room
                        .find_incoming(uuid)
                        .iter()
                        .filter_map(|e| e.from_uuid())
                        .collect()
                }
            }
            EdgeType::PanelToPanelType => {
                if direction == RelationshipDirection::Outgoing {
                    self.panel_to_panel_type
                        .find_outgoing(uuid)
                        .iter()
                        .filter_map(|e| e.to_uuid())
                        .collect()
                } else {
                    self.panel_to_panel_type
                        .find_incoming(uuid)
                        .iter()
                        .filter_map(|e| e.from_uuid())
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
                        .filter_map(|e| e.to_uuid())
                        .collect()
                } else {
                    self.event_room_to_hotel_room
                        .find_incoming(uuid)
                        .iter()
                        .filter_map(|e| e.from_uuid())
                        .collect()
                }
            }
        }
    }

    // === Entity Relationship Methods ===

    /// Get all presenters for a panel (returns PresenterIds)
    pub fn get_panel_presenters(&self, panel_id: PanelId) -> Vec<PresenterId> {
        self.panel_to_presenter
            .find_outgoing(panel_id.uuid())
            .iter()
            .filter_map(|e| e.to_uuid().map(PresenterId::from_uuid))
            .collect()
    }

    /// Get the primary event room for a panel (returns EventRoomId)
    pub fn get_panel_event_room(&self, panel_id: PanelId) -> Option<EventRoomId> {
        self.panel_to_event_room
            .find_outgoing(panel_id.uuid())
            .first()
            .and_then(|e| e.to_uuid().map(EventRoomId::from_uuid))
    }

    /// Get the panel type for a panel (returns PanelTypeId)
    pub fn get_panel_type(&self, panel_id: PanelId) -> Option<PanelTypeId> {
        self.panel_to_panel_type
            .find_outgoing(panel_id.uuid())
            .first()
            .and_then(|e| e.to_uuid().map(PanelTypeId::from_uuid))
    }

    /// Get all groups a presenter belongs to (returns PresenterIds)
    pub fn get_presenter_groups(&self, presenter_id: PresenterId) -> Vec<PresenterId> {
        self.presenter_to_group
            .direct_groups_of(presenter_id.uuid())
            .iter()
            .map(|&uuid| PresenterId::from_uuid(uuid))
            .collect()
    }

    /// Get all members of a presenter group (returns PresenterIds)
    pub fn get_presenter_members(&self, presenter_id: PresenterId) -> Vec<PresenterId> {
        self.presenter_to_group
            .direct_members_of(presenter_id.uuid())
            .iter()
            .map(|&uuid| PresenterId::from_uuid(uuid))
            .collect()
    }

    /// Get all panels a presenter participates in (returns PanelIds)
    pub fn get_presenter_panels(&self, presenter_id: PresenterId) -> Vec<PanelId> {
        self.panel_to_presenter
            .find_incoming(presenter_id.uuid())
            .iter()
            .filter_map(|e| e.from_uuid().map(PanelId::from_uuid))
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
            .get_inclusive_presenters(panel_id.uuid(), &mut self.presenter_to_group)
            .iter()
            .map(|&uuid| PresenterId::from_uuid(uuid))
            .collect()
    }

    /// Get all panels for a presenter, including those from presenter groups
    pub fn get_presenter_inclusive_panels(&mut self, presenter_id: PresenterId) -> Vec<PanelId> {
        self.panel_to_presenter
            .get_inclusive_panels(presenter_id.uuid(), &mut self.presenter_to_group)
            .iter()
            .map(|&uuid| PanelId::from_uuid(uuid))
            .collect()
    }

    // === Data Resolution Methods ===

    /// Get entity names for a list of UUIDs
    pub fn get_entity_names<T: EntityType>(&self, uuids: &[uuid::Uuid]) -> Vec<String> {
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
    pub schedule_id: uuid::Uuid,
}

impl ScheduleMetadata {
    pub fn new() -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            version: "1.0".to_string(),
            created_at: now,
            updated_at: now,
            generator: "schedule-data".to_string(),
            schedule_id: uuid::Uuid::now_v7(),
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
