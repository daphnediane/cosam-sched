/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! HotelRoom entity implementation

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// HotelRoom ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HotelRoomId(Uuid);

impl fmt::Display for HotelRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hotel-room-{}", self.0)
    }
}

impl From<Uuid> for HotelRoomId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<HotelRoomId> for Uuid {
    fn from(id: HotelRoomId) -> Uuid {
        id.0
    }
}

impl HotelRoomId {
    /// Get the UUID from this ID
    pub fn uuid(&self) -> Uuid {
        self.0
    }

    /// Create a HotelRoomId from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// HotelRoom entity for physical hotel room information
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(HotelRoom)]
pub struct HotelRoom {
    #[field(display = "Hotel Room", description = "Physical hotel room")]
    #[alias("hotel_room", "Hotel_Room", "hotel", "location")]
    #[indexable(priority = 140)]
    pub hotel_room: String,

    #[field(display = "Sort Key", description = "Room display sort order")]
    #[alias("sort_key", "Sort_Key", "sort", "order")]
    pub sort_key: i64,

    #[computed_field(
        display = "Event Rooms",
        description = "All event rooms that map to this hotel room"
    )]
    #[alias("event_rooms", "rooms", "mapped_rooms")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &HotelRoomData| {
        let event_room_ids = schedule.find_related::<crate::entity::EventRoomEntityType>(
            entity.entity_uuid,
            crate::edge::EdgeType::EventRoomToHotelRoom,
            crate::schedule::RelationshipDirection::Incoming
        );
        Some(crate::field::FieldValue::List(
            schedule.get_entity_names::<crate::entity::EventRoomEntityType>(&event_room_ids)
                .into_iter()
                .map(crate::field::FieldValue::String)
                .collect()
        ))
    })]
    pub event_rooms: Vec<crate::entity::EventRoomId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotel_room_id_from_uuid() {
        let uuid = Uuid::nil();
        let id = HotelRoomId::from(uuid);
        assert_eq!(Uuid::from(id), uuid);
    }

    #[test]
    fn hotel_room_id_display() {
        let id = HotelRoomId::from(Uuid::nil());
        assert_eq!(
            id.to_string(),
            "hotel-room-00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn hotel_room_id_serde_round_trip() {
        let id = HotelRoomId::from(Uuid::nil());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000000\"");
        let back: HotelRoomId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
