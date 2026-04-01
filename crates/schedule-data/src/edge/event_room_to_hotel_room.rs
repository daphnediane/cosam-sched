/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoomToHotelRoom edge implementation

use crate::edge::{Edge, EdgeType, SimpleEdge};
use crate::entity::EntityId;

/// EventRoomToHotelRoom edge implementation (many-to-one relationship)
#[derive(Debug, Clone)]
pub struct EventRoomToHotelRoomEdge {
    pub from_id: EntityId, // EventRoom (many)
    pub to_id: EntityId,   // HotelRoom (one)
    pub data: EventRoomToHotelRoomData,
}

#[derive(Debug, Clone)]
pub struct EventRoomToHotelRoomData {
    // No additional data needed for this simple relationship
}

impl EventRoomToHotelRoomEdge {
    pub fn new(event_room_id: EntityId, hotel_room_id: EntityId) -> Self {
        Self {
            from_id: event_room_id,
            to_id: hotel_room_id,
            data: EventRoomToHotelRoomData {},
        }
    }
}

impl Edge for EventRoomToHotelRoomEdge {
    type FromEntity = crate::entity::EventRoomEntityType;
    type ToEntity = crate::entity::HotelRoomEntityType;
    type Data = EventRoomToHotelRoomData;

    fn from_id(&self) -> EntityId {
        self.from_id
    }

    fn to_id(&self) -> EntityId {
        self.to_id
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.data
    }

    fn edge_type(&self) -> EdgeType {
        EdgeType::EventRoomToHotelRoom
    }
}

impl SimpleEdge for EventRoomToHotelRoomEdge {
    fn is_bidirectional(&self) -> bool {
        false
    }
}
