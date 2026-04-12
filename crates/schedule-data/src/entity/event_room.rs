/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoom entity — a logical room name used in the schedule.
//!
//! An event room is the name that appears in the **Room** column of the
//! Schedule sheet and must match the **Room Name** column of the Rooms sheet.
//! The physical hotel rooms it maps to are stored in `hotel_room_ids` directly
//! on this entity (virtual edge forward side), with the reverse index
//! `event_rooms_by_hotel_room` on [`EntityStorage`].
//!
//! [`EntityStorage`]: crate::schedule::EntityStorage

use crate::entity::{HotelRoomId, PanelId};
use crate::EntityFields;

/// A logical event room as it appears in the schedule.
///
/// Sourced from the **Rooms** sheet of the schedule spreadsheet.  `room_name`
/// is the short name that must match the **Room** column in the Schedule sheet.
/// `sort_key` values ≥ 100 indicate the room should be hidden from the public
/// schedule widget.
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(EventRoom)]
pub struct EventRoom {
    #[field(
        display = "Room Name",
        description = "Short room name; must match the Room column in the Schedule sheet"
    )]
    #[alias("room_name", "name", "short_name")]
    #[required]
    #[indexable(priority = 220)]
    pub room_name: String,

    #[field(
        display = "Long Name",
        description = "Display name shown in the schedule widget"
    )]
    #[alias("long_name", "display_name", "full_name")]
    #[indexable(priority = 210)]
    pub long_name: Option<String>,

    #[field(
        display = "Sort Key",
        description = "Numeric sort order; values ≥ 100 are hidden from the public schedule"
    )]
    #[alias("sort_key", "sort_order")]
    pub sort_key: Option<i64>,

    /// Backing storage for hotel room relationships (owned forward side).
    /// Updated by the `hotel_rooms` computed field write closure.
    pub hotel_room_ids: Vec<crate::entity::HotelRoomId>,

    // --- Computed: schedule-aware -------------------------------------------
    #[computed_field(
        display = "Hotel Rooms",
        description = "Physical hotel rooms this event room maps to"
    )]
    #[alias("hotel_rooms", "physical_rooms")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &EventRoomData| {
        use crate::entity::InternalData;
        let event_room_id = EventRoomId::from_uuid(entity.uuid());
        let ids = EventRoomEntityType::hotel_rooms_of(&schedule.entities, event_room_id);
        Some(crate::field::FieldValue::hotel_room_list(ids))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut EventRoomData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let event_room_id = EventRoomId::from_uuid(entity.uuid());
        let hotel_room_ids = HotelRoomId::from_field_values(value, schedule)?;
        EventRoomEntityType::set_hotel_rooms(&mut schedule.entities, event_room_id, hotel_room_ids)
    })]
    pub hotel_rooms: Vec<crate::entity::HotelRoomId>,

    #[computed_field(
        display = "Panels",
        description = "Panels scheduled in this event room"
    )]
    #[alias("panels", "scheduled_panels")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &EventRoomData| {
        use crate::entity::InternalData;
        let event_room_id = EventRoomId::from_uuid(entity.uuid());
        let ids = EventRoomEntityType::panels_of(&schedule.entities, event_room_id);
        Some(crate::field::FieldValue::panel_list(ids))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut EventRoomData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let event_room_id = EventRoomId::from_uuid(entity.uuid());
        let panel_ids = PanelId::from_field_values(value, schedule)?;
        EventRoomEntityType::set_panels(&mut schedule.entities, event_room_id, panel_ids)
    })]
    pub panels: Vec<crate::entity::PanelId>,
}

impl EventRoomEntityType {
    /// Get all hotel rooms assigned to this event room.
    pub fn hotel_rooms_of(
        storage: &crate::schedule::EntityStorage,
        event_room_id: EventRoomId,
    ) -> Vec<HotelRoomId> {
        let uuid = event_room_id.non_nil_uuid();
        storage
            .event_rooms_by_hotel_room
            .get(&uuid)
            .map(|uuids| uuids.iter().map(|&u| HotelRoomId::from_uuid(u)).collect())
            .unwrap_or_default()
    }

    /// Set the hotel rooms assigned to this event room.
    ///
    /// Updates both the forward backing field and the reverse index.
    pub fn set_hotel_rooms(
        storage: &mut crate::schedule::EntityStorage,
        event_room_id: EventRoomId,
        hotel_room_ids: Vec<HotelRoomId>,
    ) -> Result<(), crate::field::FieldError> {
        let event_room_uuid = event_room_id.non_nil_uuid();
        let entity = storage.event_rooms.get_mut(&event_room_uuid).ok_or(
            crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            ),
        )?;

        let new_hotel_room_uuids: Vec<uuid::NonNilUuid> =
            hotel_room_ids.iter().map(|id| id.non_nil_uuid()).collect();

        // Remove event room from old hotel room reverse index entries
        for old_id in &entity.hotel_room_ids {
            let hr_uuid = old_id.non_nil_uuid();
            if let Some(rooms) = storage.event_rooms_by_hotel_room.get_mut(&hr_uuid) {
                rooms.retain(|&u| u != event_room_uuid);
            }
        }

        // Update forward backing field
        entity.hotel_room_ids = hotel_room_ids;

        // Add event room to new hotel room reverse index entries
        for &hr_uuid in &new_hotel_room_uuids {
            storage
                .event_rooms_by_hotel_room
                .entry(hr_uuid)
                .or_default()
                .push(event_room_uuid);
        }

        Ok(())
    }

    /// Get all panels scheduled in this event room.
    pub fn panels_of(
        storage: &crate::schedule::EntityStorage,
        event_room_id: EventRoomId,
    ) -> Vec<PanelId> {
        let uuid = event_room_id.non_nil_uuid();
        storage
            .panels_by_event_room
            .get(&uuid)
            .map(|uuids| uuids.iter().map(|&u| PanelId::from_uuid(u)).collect())
            .unwrap_or_default()
    }

    /// Set the panels scheduled in this event room.
    ///
    /// Stub implementation - full relationship management deferred to future.
    pub fn set_panels(
        _storage: &mut crate::schedule::EntityStorage,
        _event_room_id: EventRoomId,
        _panel_ids: Vec<PanelId>,
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
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5,
            ]))
        }
    }

    #[test]
    fn event_room_id_from_uuid() {
        let nn = test_nn();
        let id = EventRoomId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn event_room_id_try_from_nil_returns_none() {
        assert!(EventRoomId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn event_room_id_display() {
        let id = EventRoomId::from(test_nn());
        assert_eq!(
            id.to_string(),
            "event-room-00000000-0000-0000-0000-000000000005"
        );
    }

    #[test]
    fn event_room_id_serde_round_trip() {
        let id = EventRoomId::from(test_nn());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000005\"");
        let back: EventRoomId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
