/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! HotelRoom entity — a physical hotel room.
//!
//! A single hotel room may host different [`EventRoom`]s at different times of
//! day (e.g. "Workshop 3" in the morning, "Demo Room 2" in the evening).  The
//! relationship is stored as `hotel_room_ids` on each [`EventRoom`] (virtual
//! edge forward side), with the reverse index `event_rooms_by_hotel_room` on
//! [`EntityStorage`].
//!
//! [`EventRoom`]: crate::entity::EventRoom
//! [`EntityStorage`]: crate::schedule::EntityStorage

use crate::entity::EventRoomId;
use crate::EntityFields;

/// A physical hotel room.
///
/// Hotel rooms are sourced from the **Hotel Room** column of the Rooms sheet.
/// One hotel room can serve as multiple event rooms at different times (e.g.
/// a ballroom split by moveable walls).  The relationship is stored as
/// `hotel_room_ids` on each [`EventRoom`](crate::entity::EventRoom) with a
/// reverse lookup index on [`EntityStorage`](crate::schedule::EntityStorage).
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
        use crate::entity::InternalData;
        let hotel_room_id = HotelRoomId::from_uuid(entity.uuid());
        let ids = HotelRoomEntityType::event_rooms_of(&schedule.entities, hotel_room_id);
        Some(crate::field::FieldValue::event_room_list(ids))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut HotelRoomData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let hotel_room_id = HotelRoomId::from_uuid(entity.uuid());
        let event_room_ids = EventRoomId::from_field_values(value, schedule)?;
        HotelRoomEntityType::set_event_rooms(&mut schedule.entities, hotel_room_id, event_room_ids)
    })]
    pub event_rooms: Vec<crate::entity::EventRoomId>,
}

impl HotelRoomEntityType {
    /// Get all event rooms assigned to this hotel room.
    pub fn event_rooms_of(
        storage: &crate::schedule::EntityStorage,
        hotel_room_id: HotelRoomId,
    ) -> Vec<EventRoomId> {
        let uuid = hotel_room_id.non_nil_uuid();
        storage
            .event_rooms_by_hotel_room
            .get(&uuid)
            .map(|uuids| uuids.iter().map(|&u| EventRoomId::from_uuid(u)).collect())
            .unwrap_or_default()
    }

    /// Set the event rooms assigned to this hotel room.
    ///
    /// Stub implementation - full relationship management deferred to future.
    pub fn set_event_rooms(
        _storage: &mut crate::schedule::EntityStorage,
        _hotel_room_id: HotelRoomId,
        _event_room_ids: Vec<EventRoomId>,
    ) -> Result<(), crate::field::FieldError> {
        unimplemented!()
    }
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
