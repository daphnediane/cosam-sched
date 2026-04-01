/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge trait hierarchy for schedule-data relationships

use crate::entity::{EntityId, EntityType};
use std::fmt::Debug;

/// Base edge trait with common operations
pub trait Edge: Debug + Clone {
    type FromEntity: EntityType;
    type ToEntity: EntityType;
    type Data: Debug + Clone;

    fn from_id(&self) -> EntityId;
    fn to_id(&self) -> EntityId;
    fn data(&self) -> &Self::Data;
    fn data_mut(&mut self) -> &mut Self::Data;
    fn edge_type(&self) -> EdgeType;
}

/// Relationship edge for presenter-group relationships with transitive closure
pub trait RelationshipEdge: Edge {
    fn get_inclusive_members(&self, storage: &dyn RelationshipStorage) -> Vec<EntityId>;
    fn get_inclusive_groups(&self, storage: &dyn RelationshipStorage) -> Vec<EntityId>;
    fn add_member(&mut self, member_id: EntityId) -> Result<(), EdgeError>;
    fn remove_member(&mut self, member_id: EntityId) -> Result<(), EdgeError>;
    fn make_group(&mut self) -> Result<(), EdgeError>;
}

/// Simple edge for basic relationships (panel-room, panel-type)
pub trait SimpleEdge: Edge {
    fn is_bidirectional(&self) -> bool;
}

/// Trait for relationship storage operations
pub trait RelationshipStorage {
    fn get_inclusive_members(&self, group_id: EntityId) -> &[EntityId];
    fn get_inclusive_groups(&self, member_id: EntityId) -> &[EntityId];
    fn is_group(&self, presenter_id: EntityId) -> bool;
    fn is_always_grouped(&self, member_id: EntityId, group_id: EntityId) -> bool;
    fn is_always_shown_in_group(&self, group_id: EntityId) -> bool;
}

/// Edge types for relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeType {
    PresenterToGroup,
    PanelToPresenter,
    PanelToEventRoom,
    EventRoomToHotelRoom,
    PanelToPanelType,
}

impl EdgeType {
    /// Get a human-readable name for the edge type
    pub fn name(&self) -> &'static str {
        match self {
            EdgeType::PresenterToGroup => "presenter_to_group",
            EdgeType::PanelToPresenter => "panel_to_presenter",
            EdgeType::PanelToEventRoom => "panel_to_event_room",
            EdgeType::EventRoomToHotelRoom => "event_room_to_hotel_room",
            EdgeType::PanelToPanelType => "panel_to_panel_type",
        }
    }
}

/// Edge operation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum EdgeError {
    #[error("Edge not found: {edge_id}")]
    EdgeNotFound { edge_id: String },

    #[error("Duplicate edge: {from_id} -> {to_id}")]
    DuplicateEdge { from_id: String, to_id: String },

    #[error("Invalid edge operation: {reason}")]
    InvalidOperation { reason: String },

    #[error("Storage error: {message}")]
    StorageError { message: String },

    #[error("Entity not found: {entity_type} {id}")]
    EntityNotFound { entity_type: String, id: String },
}

/// Edge ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(pub u64);

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "edge-{}", self.0)
    }
}

use std::fmt;
