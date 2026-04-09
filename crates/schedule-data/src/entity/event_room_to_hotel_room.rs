/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoomToHotelRoom edge-entity implementation
//!
//! This edge type connects event rooms to their associated hotel rooms.
//! As an edge-entity, it has its own UUID and can store metadata.

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::{NonNilUuid, Uuid};

/// EventRoomToHotelRoom edge ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventRoomToHotelRoomId(NonNilUuid);

impl EventRoomToHotelRoomId {
    /// Get the NonNilUuid from this ID
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }

    /// Get the raw UUID from this ID
    pub fn uuid(&self) -> Uuid {
        self.0.into()
    }

    /// Create an EventRoomToHotelRoomId from a NonNilUuid (infallible)
    pub fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }

    /// Try to create an EventRoomToHotelRoomId from a raw UUID (boundary use only)
    pub fn try_from_raw_uuid(uuid: Uuid) -> Option<Self> {
        NonNilUuid::new(uuid).map(Self)
    }
}

impl fmt::Display for EventRoomToHotelRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event-room-to-hotel-room-{}", self.0)
    }
}

impl From<NonNilUuid> for EventRoomToHotelRoomId {
    fn from(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl From<EventRoomToHotelRoomId> for NonNilUuid {
    fn from(id: EventRoomToHotelRoomId) -> NonNilUuid {
        id.0
    }
}

impl From<EventRoomToHotelRoomId> for Uuid {
    fn from(id: EventRoomToHotelRoomId) -> Uuid {
        id.0.into()
    }
}

impl crate::entity::TypedId for EventRoomToHotelRoomId {
    type EntityType = EventRoomToHotelRoomEntityType;
    fn non_nil_uuid(&self) -> NonNilUuid { self.0 }
    fn from_uuid(uuid: NonNilUuid) -> Self { Self(uuid) }
}

/// EventRoomToHotelRoom edge-entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(EventRoomToHotelRoom)]
pub struct EventRoomToHotelRoom {
    /// UUID of the event room (from side)
    #[field(display = "Event Room UUID", description = "UUID of the event room")]
    #[required]
    pub event_room_uuid: NonNilUuid,

    /// UUID of the hotel room (to side)
    #[field(display = "Hotel Room UUID", description = "UUID of the hotel room")]
    #[required]
    pub hotel_room_uuid: NonNilUuid,
}

impl EventRoomToHotelRoomData {
    /// Get the event room ID from this edge
    pub fn event_room_id(&self) -> crate::entity::EventRoomId {
        crate::entity::EventRoomId::from_uuid(self.event_room_uuid)
    }

    /// Get the hotel room ID from this edge
    pub fn hotel_room_id(&self) -> crate::entity::HotelRoomId {
        crate::entity::HotelRoomId::from_uuid(self.hotel_room_uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nn() -> NonNilUuid {
        unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])) }
    }

    #[test]
    fn event_room_to_hotel_room_id_from_uuid() {
        let nn = test_nn();
        let id = EventRoomToHotelRoomId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn event_room_to_hotel_room_id_try_from_nil_uuid_returns_none() {
        assert!(EventRoomToHotelRoomId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn event_room_to_hotel_room_id_display() {
        let id = EventRoomToHotelRoomId::from(test_nn());
        assert_eq!(id.to_string(), "event-room-to-hotel-room-00000000-0000-0000-0000-000000000001");
    }

    #[test]
    fn event_room_to_hotel_room_data_ids() {
        let event_room_uuid = unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])) };
        let hotel_room_uuid = unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2])) };

        let data = EventRoomToHotelRoomData {
            entity_uuid: test_nn(),
            event_room_uuid,
            hotel_room_uuid,
        };

        assert_eq!(data.event_room_id().non_nil_uuid(), event_room_uuid);
        assert_eq!(data.hotel_room_id().non_nil_uuid(), hotel_room_uuid);
    }
}
