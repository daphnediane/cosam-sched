/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! HotelRoom entity — a physical hotel room.
//!
//! A single hotel room may host different [`EventRoom`]s at different times of
//! day (e.g. "Workshop 3" in the morning, "Demo Room 2" in the evening).  The
//! time-dependent relationship between hotel rooms and event rooms is managed
//! via edges (FEATURE-007).
//!
//! [`EventRoom`]: crate::entity::EventRoom

use crate::EntityFields;

/// A physical hotel room.
///
/// Hotel rooms are sourced from the **Hotel Room** column of the Rooms sheet.
/// One hotel room can serve as multiple event rooms at different times (e.g.
/// a ballroom split by moveable walls), which is represented by
/// [`EventRoomToHotelRoom`] edges with time-range attributes.
///
/// [`EventRoomToHotelRoom`]: crate::entity::EntityKind::EventRoomToHotelRoom
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(HotelRoom)]
pub struct HotelRoom {
    #[field(
        display = "Hotel Room Name",
        description = "Physical hotel room name (e.g. \"Salon EFG\", \"Panel Room 1\")"
    )]
    #[alias("hotel_room_name", "room_name", "name")]
    #[required]
    #[indexable(priority = 220)]
    pub hotel_room_name: String,

    // --- Computed: schedule-aware (edge-based) --------------------------------
    #[computed_field(
        display = "Event Rooms",
        description = "Logical event rooms that map to this hotel room"
    )]
    #[alias("event_rooms", "logical_rooms")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &HotelRoomData| {
        use crate::entity::{InternalData, EventRoomToHotelRoomEntityType};
        let ids = EventRoomToHotelRoomEntityType::event_rooms_in(&schedule.entities, entity.uuid());
        if ids.is_empty() {
            None
        } else {
            Some(crate::field::FieldValue::List(
                ids.into_iter()
                    .map(|id| crate::field::FieldValue::NonNilUuid(id.non_nil_uuid()))
                    .collect(),
            ))
        }
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, _entity: &mut HotelRoomData, _value: crate::field::FieldValue| {
        let _ = schedule;
        Err(crate::field::FieldError::CannotStoreRelationshipField)
    })]
    pub event_rooms: Vec<crate::entity::EventRoomId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::{NonNilUuid, Uuid};

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4,
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
    fn hotel_room_id_try_from_nil_returns_none() {
        assert!(HotelRoomId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn hotel_room_id_display() {
        let id = HotelRoomId::from(test_nn());
        assert_eq!(
            id.to_string(),
            "hotel-room-00000000-0000-0000-0000-000000000004"
        );
    }

    #[test]
    fn hotel_room_id_serde_round_trip() {
        let id = HotelRoomId::from(test_nn());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000004\"");
        let back: HotelRoomId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
