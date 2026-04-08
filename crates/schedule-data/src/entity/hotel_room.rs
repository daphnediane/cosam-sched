/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! HotelRoom entity implementation

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::{NonNilUuid, Uuid};

/// HotelRoom ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HotelRoomId(NonNilUuid);

impl fmt::Display for HotelRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hotel-room-{}", self.0)
    }
}

impl From<NonNilUuid> for HotelRoomId {
    fn from(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl From<HotelRoomId> for NonNilUuid {
    fn from(id: HotelRoomId) -> NonNilUuid {
        id.0
    }
}

impl From<HotelRoomId> for Uuid {
    fn from(id: HotelRoomId) -> Uuid {
        id.0.into()
    }
}

impl crate::entity::TypedId for HotelRoomId {
    type EntityType = HotelRoomEntityType;
    fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }
    fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl HotelRoomId {
    /// Get the NonNilUuid from this ID
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }

    /// Get the raw UUID from this ID
    pub fn uuid(&self) -> Uuid {
        self.0.into()
    }

    /// Create a HotelRoomId from a NonNilUuid (infallible)
    pub fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }

    /// Try to create a HotelRoomId from a raw UUID (boundary use only)
    pub fn try_from_raw_uuid(uuid: Uuid) -> Option<Self> {
        NonNilUuid::new(uuid).map(Self)
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

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        }
    }

    #[test]
    fn hotel_room_id_from_uuid() {
        let nn = test_nn();
        let id = HotelRoomId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn hotel_room_id_try_from_nil_uuid_returns_none() {
        assert!(HotelRoomId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn hotel_room_id_display() {
        let id = HotelRoomId::from(test_nn());
        assert_eq!(
            id.to_string(),
            "hotel-room-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn hotel_room_id_serde_round_trip() {
        let id = HotelRoomId::from(test_nn());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000001\"");
        let back: HotelRoomId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
