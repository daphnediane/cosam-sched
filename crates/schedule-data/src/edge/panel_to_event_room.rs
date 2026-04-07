/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToEventRoom edge implementation

use crate::edge::{Edge, EdgeType};
use crate::entity::EntityId;

/// PanelToEventRoom edge implementation
#[derive(Debug, Clone)]
pub struct PanelToEventRoomEdge {
    pub from_id: EntityId, // Panel
    pub to_id: EntityId,   // EventRoom
    pub data: PanelToEventRoomData,
}

#[derive(Debug, Clone)]
pub struct PanelToEventRoomData {
    // No additional data needed for this simple relationship
}

impl PanelToEventRoomEdge {
    pub fn new(panel_id: EntityId, event_room_id: EntityId) -> Self {
        Self {
            from_id: panel_id,
            to_id: event_room_id,
            data: PanelToEventRoomData {},
        }
    }
}

impl Edge for PanelToEventRoomEdge {
    type FromEntity = crate::entity::PanelEntityType;
    type ToEntity = crate::entity::EventRoomEntityType;
    type Data = PanelToEventRoomData;

    fn from_id(&self) -> Option<EntityId> {
        Some(self.from_id)
    }

    fn to_id(&self) -> Option<EntityId> {
        Some(self.to_id)
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.data
    }

    fn edge_type(&self) -> EdgeType {
        EdgeType::PanelToEventRoom
    }
}
