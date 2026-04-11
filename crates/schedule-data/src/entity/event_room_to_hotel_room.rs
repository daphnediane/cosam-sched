/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoomToHotelRoom edge-entity implementation.
//!
//! Connects a logical event room to the physical hotel room it occupies.
//! Multiple event rooms may share one hotel room (partitioned by time range).

use crate::EntityFields;
use uuid::NonNilUuid;

/// EventRoomToHotelRoom edge-entity.
///
/// The macro generates `EventRoomToHotelRoomId`, `EventRoomToHotelRoomData`,
/// and `EventRoomToHotelRoomEntityType`.
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(EventRoomToHotelRoom)]
pub struct EventRoomToHotelRoom {
    /// UUID of the event room (from side).
    #[field(display = "Event Room UUID", description = "UUID of the event room")]
    #[required]
    #[edge_from(EventRoom)]
    pub event_room_uuid: NonNilUuid,

    /// UUID of the hotel room (to side).
    #[field(display = "Hotel Room UUID", description = "UUID of the hotel room")]
    #[required]
    #[edge_to(HotelRoom)]
    pub hotel_room_uuid: NonNilUuid,
}

// ---------------------------------------------------------------------------
// Convenience queries on EventRoomToHotelRoomEntityType
// ---------------------------------------------------------------------------

impl EventRoomToHotelRoomEntityType {
    /// Hotel rooms mapped to an event room (outgoing edges).
    pub fn hotel_rooms_of(
        storage: &crate::schedule::EntityStorage,
        event_room: NonNilUuid,
    ) -> Vec<crate::entity::HotelRoomId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .outgoing(event_room)
            .iter()
            .filter_map(|edge_uuid| map.get(edge_uuid))
            .map(|edge| crate::entity::HotelRoomId::from(edge.to_uuid()))
            .collect()
    }

    /// Event rooms that use a hotel room (incoming edges).
    pub fn event_rooms_in(
        storage: &crate::schedule::EntityStorage,
        hotel_room: NonNilUuid,
    ) -> Vec<crate::entity::EventRoomId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .incoming(hotel_room)
            .iter()
            .filter_map(|edge_uuid| map.get(edge_uuid))
            .map(|edge| crate::entity::EventRoomId::from(edge.from_uuid()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn nn(b: u8) -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b,
            ]))
        }
    }

    #[test]
    fn event_room_to_hotel_room_id_round_trip() {
        let id = EventRoomToHotelRoomId::from(nn(1));
        assert_eq!(NonNilUuid::from(id), nn(1));
    }

    #[test]
    fn event_room_to_hotel_room_id_try_from_nil_returns_none() {
        assert!(EventRoomToHotelRoomId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn event_room_to_hotel_room_id_display() {
        let id = EventRoomToHotelRoomId::from(nn(1));
        assert_eq!(
            id.to_string(),
            "event-room-to-hotel-room-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn event_room_to_hotel_room_data_accessors() {
        let data = EventRoomToHotelRoomData {
            entity_uuid: nn(3),
            event_room_uuid: nn(1),
            hotel_room_uuid: nn(2),
        };
        assert_eq!(data.event_room_id().non_nil_uuid(), nn(1));
        assert_eq!(data.hotel_room_id().non_nil_uuid(), nn(2));
    }
}
