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

use crate::edge::{Edge, EdgeStorage as _};

use crate::edge::{
    EventRoomToHotelRoomStorage, GenericEdgeStorage, PanelToPanelTypeStorage,
    PanelToPresenterStorage, PresenterToGroupStorage,
};

/// Relationship direction for edge queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
    Outgoing, // Entity -> Related
    Incoming, // Related -> Entity
}
use crate::entity::{EntityId, EntityType};
use crate::field::validation::ValidationError;
use crate::field::FieldValue;
use crate::query::{FieldMatch, QueryOptions};

// Re-export storage types
pub use storage::*;

/// Unique identifier for edges
pub type EdgeId = u64;

#[derive(Debug, Clone)]
pub struct IdAllocators {
    next_by_type: HashMap<String, u64>,
    next_edge_id: u64,
}

impl IdAllocators {
    pub fn new() -> Self {
        Self {
            next_by_type: HashMap::new(),
            next_edge_id: 0,
        }
    }

    pub fn allocate_entity_id(&mut self, type_name: &str) -> u64 {
        let next = self.next_by_type.entry(type_name.to_string()).or_insert(0);
        let allocated = *next;
        *next = allocated.saturating_add(1);
        allocated
    }

    pub fn allocate_edge_id(&mut self) -> u64 {
        let allocated = self.next_edge_id;
        self.next_edge_id = allocated.saturating_add(1);
        allocated
    }
}

impl Default for IdAllocators {
    fn default() -> Self {
        Self::new()
    }
}

/// Schedule container holding all entities and relationships
#[derive(Debug, Clone)]
pub struct Schedule {
    pub entities: EntityStorage,
    pub presenter_to_group: PresenterToGroupStorage,
    pub panel_to_presenter: PanelToPresenterStorage,
    pub panel_to_event_room: GenericEdgeStorage<crate::edge::PanelToEventRoomEdge>,
    pub event_room_to_hotel_room: EventRoomToHotelRoomStorage,
    pub panel_to_panel_type: PanelToPanelTypeStorage,
    pub id_allocators: IdAllocators,
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
            id_allocators: IdAllocators::new(),
            metadata: ScheduleMetadata::new(),
        }
    }

    /// Get entity by type and ID
    pub fn get_entity<T: EntityType>(&self, id: EntityId) -> Option<&T::Data> {
        self.entities.get::<T>(id)
    }

    /// Get entity by internal monotonic ID
    pub fn get_entity_by_internal_id<T: EntityType>(&self, internal_id: u64) -> Option<&T::Data> {
        self.entities.get_by_internal_id::<T>(internal_id)
    }

    /// Find entities matching field conditions
    pub fn find_entities<T: EntityType>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<EntityId> {
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
    ) -> Result<EntityId, ScheduleError> {
        let internal_id = self.id_allocators.allocate_entity_id(T::TYPE_NAME);
        self.entities.add_with_id::<T>(internal_id, entity)?;
        Ok(internal_id)
    }

    /// Add entity and return both internal and external IDs
    pub fn add_entity_with_internal_id<T: EntityType>(
        &mut self,
        entity: T::Data,
    ) -> Result<u64, ScheduleError> {
        let internal_id = self.id_allocators.allocate_entity_id(T::TYPE_NAME);
        self.entities.add_with_id::<T>(internal_id, entity)?;
        Ok(internal_id)
    }

    /// Update entity fields
    pub fn update_entity<T: EntityType>(
        &mut self,
        id: EntityId,
        updates: &[(String, FieldValue)],
    ) -> Result<(), ScheduleError> {
        self.entities.update::<T>(id, updates)
    }

    /// Find entities related to a given entity (dispatches to appropriate typed storage)
    pub fn find_related<T: EntityType>(
        &self,
        entity_id: EntityId,
        edge_type: crate::edge::EdgeType,
        direction: RelationshipDirection,
    ) -> Vec<EntityId> {
        use crate::edge::EdgeType;
        let internal_id = crate::entity::InternalId::new::<T>(entity_id);
        match edge_type {
            EdgeType::PanelToPresenter => {
                if direction == RelationshipDirection::Outgoing {
                    self.panel_to_presenter
                        .find_outgoing(internal_id)
                        .iter()
                        .filter_map(|e| e.to_id().map(|id| id.entity_id))
                        .collect()
                } else {
                    self.panel_to_presenter
                        .find_incoming(internal_id)
                        .iter()
                        .filter_map(|e| e.from_id().map(|id| id.entity_id))
                        .collect()
                }
            }
            EdgeType::PanelToEventRoom => {
                if direction == RelationshipDirection::Outgoing {
                    self.panel_to_event_room
                        .find_outgoing(internal_id)
                        .iter()
                        .filter_map(|e| e.to_id().map(|id| id.entity_id))
                        .collect()
                } else {
                    self.panel_to_event_room
                        .find_incoming(internal_id)
                        .iter()
                        .filter_map(|e| e.from_id().map(|id| id.entity_id))
                        .collect()
                }
            }
            EdgeType::PanelToPanelType => {
                if direction == RelationshipDirection::Outgoing {
                    self.panel_to_panel_type
                        .find_outgoing(internal_id)
                        .iter()
                        .filter_map(|e| e.to_id().map(|id| id.entity_id))
                        .collect()
                } else {
                    self.panel_to_panel_type
                        .find_incoming(internal_id)
                        .iter()
                        .filter_map(|e| e.from_id().map(|id| id.entity_id))
                        .collect()
                }
            }
            EdgeType::PresenterToGroup => {
                if direction == RelationshipDirection::Outgoing {
                    self.presenter_to_group.direct_groups_of(entity_id).to_vec()
                } else {
                    self.presenter_to_group
                        .direct_members_of(entity_id)
                        .to_vec()
                }
            }
            EdgeType::EventRoomToHotelRoom => {
                if direction == RelationshipDirection::Outgoing {
                    self.event_room_to_hotel_room
                        .find_outgoing(internal_id)
                        .iter()
                        .filter_map(|e| e.to_id().map(|id| id.entity_id))
                        .collect()
                } else {
                    self.event_room_to_hotel_room
                        .find_incoming(internal_id)
                        .iter()
                        .filter_map(|e| e.from_id().map(|id| id.entity_id))
                        .collect()
                }
            }
        }
    }

    // === Entity Relationship Methods ===

    /// Get all presenters for a panel (returns EntityIds)
    pub fn get_panel_presenters(&self, panel_id: EntityId) -> Vec<EntityId> {
        let internal_id =
            crate::entity::InternalId::new::<crate::entity::PanelEntityType>(panel_id);
        self.panel_to_presenter
            .find_outgoing(internal_id)
            .iter()
            .filter_map(|e| e.to_id().map(|id| id.entity_id))
            .collect()
    }

    /// Get the primary event room for a panel (returns EntityId)
    pub fn get_panel_event_room(&self, panel_id: EntityId) -> Option<EntityId> {
        let internal_id =
            crate::entity::InternalId::new::<crate::entity::PanelEntityType>(panel_id);
        self.panel_to_event_room
            .find_outgoing(internal_id)
            .first()
            .map(|e| e.to_id().map(|id| id.entity_id))
            .flatten()
    }

    /// Get the panel type for a panel (returns EntityId)
    pub fn get_panel_type(&self, panel_id: EntityId) -> Option<EntityId> {
        self.panel_to_panel_type.get_panel_type(panel_id)
    }

    /// Get all groups a presenter belongs to (returns EntityIds)
    pub fn get_presenter_groups(&self, presenter_id: EntityId) -> Vec<EntityId> {
        self.presenter_to_group
            .direct_groups_of(presenter_id)
            .to_vec()
    }

    /// Get all members of a presenter group (returns EntityIds)
    pub fn get_presenter_members(&self, presenter_id: EntityId) -> Vec<EntityId> {
        self.presenter_to_group
            .direct_members_of(presenter_id)
            .to_vec()
    }

    /// Get all panels a presenter participates in (returns EntityIds)
    pub fn get_presenter_panels(&self, presenter_id: EntityId) -> Vec<EntityId> {
        let internal_id =
            crate::entity::InternalId::new::<crate::entity::PresenterEntityType>(presenter_id);
        self.panel_to_presenter
            .find_incoming(internal_id)
            .iter()
            .filter_map(|e| e.from_id().map(|id| id.entity_id))
            .collect()
    }

    /// Connect a panel to a presenter
    pub fn connect_panel_to_presenter(
        &mut self,
        panel_id: EntityId,
        presenter_id: EntityId,
    ) -> Result<EdgeId, ScheduleError> {
        let edge = crate::edge::PanelToPresenterEdge::new(panel_id, presenter_id);
        self.panel_to_presenter
            .add_edge(edge)
            .map(|id| id.0)
            .map_err(|e| ScheduleError::StorageError {
                message: e.to_string(),
            })
    }

    /// Connect a panel to an event room
    pub fn connect_panel_to_event_room(
        &mut self,
        panel_id: EntityId,
        room_id: EntityId,
    ) -> Result<EdgeId, ScheduleError> {
        let edge = crate::edge::PanelToEventRoomEdge::new(panel_id, room_id);
        self.panel_to_event_room
            .add_edge(edge)
            .map(|id| id.0)
            .map_err(|e| ScheduleError::StorageError {
                message: e.to_string(),
            })
    }

    /// Connect a panel to a panel type
    pub fn connect_panel_to_panel_type(
        &mut self,
        panel_id: EntityId,
        type_id: EntityId,
    ) -> Result<EdgeId, ScheduleError> {
        let edge = crate::edge::PanelToPanelTypeEdge::new(panel_id, type_id);
        self.panel_to_panel_type
            .add_edge(edge)
            .map(|id| id.0)
            .map_err(|e| ScheduleError::StorageError {
                message: e.to_string(),
            })
    }

    /// Connect a presenter to a group
    pub fn connect_presenter_to_group(
        &mut self,
        presenter_id: EntityId,
        group_id: EntityId,
    ) -> Result<EdgeId, ScheduleError> {
        let edge = crate::edge::presenter_to_group::PresenterToGroupEdge::new(
            presenter_id,
            group_id,
            false,
            false,
        );
        self.presenter_to_group
            .add_edge(edge)
            .map(|id| id.0)
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
    pub fn get_panel_inclusive_presenters(&mut self, panel_id: EntityId) -> Vec<EntityId> {
        let internal_id =
            crate::entity::InternalId::new::<crate::entity::PanelEntityType>(panel_id);
        self.panel_to_presenter
            .get_inclusive_presenters(internal_id, &mut self.presenter_to_group)
            .to_vec()
    }

    /// Get all panels for a presenter, including those from presenter groups
    pub fn get_presenter_inclusive_panels(&mut self, presenter_id: EntityId) -> Vec<EntityId> {
        let internal_id =
            crate::entity::InternalId::new::<crate::entity::PresenterEntityType>(presenter_id);
        self.panel_to_presenter
            .get_inclusive_panels(internal_id, &mut self.presenter_to_group)
            .to_vec()
    }

    // === Data Resolution Methods ===

    /// Get entity names for a list of EntityIds
    pub fn get_entity_names<T: EntityType>(&self, entity_ids: &[EntityId]) -> Vec<String> {
        entity_ids
            .iter()
            .filter_map(|&id| self.get_entity::<T>(id))
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
}

impl ScheduleMetadata {
    pub fn new() -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            version: "1.0".to_string(),
            created_at: now,
            updated_at: now,
            generator: "schedule-data".to_string(),
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
