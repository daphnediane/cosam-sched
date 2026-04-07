/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge trait hierarchy for schedule-data relationships

use crate::entity::EntityType;
use std::fmt::{self, Debug};
use uuid::Uuid;

/// Core trait for all edge relationships
pub trait Edge: Debug + Clone {
    type FromEntity: EntityType;
    type ToEntity: EntityType;
    type Data: Debug + Clone;

    fn from_uuid(&self) -> Option<Uuid>;
    fn to_uuid(&self) -> Option<Uuid>;
    fn data(&self) -> &Self::Data;
    fn data_mut(&mut self) -> &mut Self::Data;
    fn edge_type(&self) -> EdgeType;
}

/// Relationship edge for presenter-group relationships with transitive closure
pub trait RelationshipEdge: Edge {
    fn get_inclusive_members(&self, storage: &dyn RelationshipStorage) -> Vec<Uuid>;
    fn get_inclusive_groups(&self, storage: &dyn RelationshipStorage) -> Vec<Uuid>;
    fn add_member(&mut self, member_id: Uuid) -> Result<(), EdgeError>;
    fn remove_member(&mut self, member_id: Uuid) -> Result<(), EdgeError>;
    fn make_group(&mut self) -> Result<(), EdgeError>;
}

/// Simple edge for basic relationships (panel-room, panel-type)
pub trait SimpleEdge: Edge {
    fn is_bidirectional(&self) -> bool;
}

/// Trait for relationship storage operations
pub trait RelationshipStorage {
    fn get_inclusive_members(&self, group_id: Uuid) -> &[Uuid];
    fn get_inclusive_groups(&self, member_id: Uuid) -> &[Uuid];
    fn is_group(&self, presenter_id: Uuid) -> bool;
    fn is_always_grouped(&self, member_id: Uuid, group_id: Uuid) -> bool;
    fn is_always_shown_in_group(&self, group_id: Uuid) -> bool;
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

/// Trait for type-safe edge storage operations
pub trait EdgeStorage<E: Edge> {
    fn add_edge(&mut self, edge: E) -> Result<EdgeId, EdgeError>;
    fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError>;
    fn get_edge(&self, edge_id: EdgeId) -> Option<&E>;
    fn find_outgoing(&self, from_uuid: Uuid) -> Vec<&E>;
    fn find_incoming(&self, to_uuid: Uuid) -> Vec<&E>;
    fn edge_exists(&self, from_uuid: Uuid, to_uuid: Uuid) -> bool;
    fn len(&self) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_id_display() {
        let id = EdgeId(42);
        assert_eq!(id.to_string(), "edge-42");
    }

    #[test]
    fn edge_id_copy() {
        let id = EdgeId(1);
        let id2 = id;
        assert_eq!(id, id2);
    }
}
