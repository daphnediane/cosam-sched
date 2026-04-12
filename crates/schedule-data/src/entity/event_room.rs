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
    #[read(|_schedule: &crate::schedule::Schedule, entity: &EventRoomData| {
        if entity.hotel_room_ids.is_empty() {
            None
        } else {
            Some(crate::field::FieldValue::List(
                entity.hotel_room_ids.iter()
                    .map(|id| crate::field::FieldValue::NonNilUuid(id.non_nil_uuid()))
                    .collect(),
            ))
        }
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut EventRoomData, value: crate::field::FieldValue| {
        use crate::entity::{HotelRoomId, InternalData};
        let event_room_uuid = entity.uuid();
        let new_hotel_room_uuids: Vec<uuid::NonNilUuid> = match value {
            crate::field::FieldValue::List(items) => items
                .into_iter()
                .filter_map(|v| if let crate::field::FieldValue::NonNilUuid(u) = v { Some(u) } else { None })
                .collect(),
            crate::field::FieldValue::NonNilUuid(u) => vec![u],
            _ => return Err(crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )),
        };
        // Remove event room from old hotel room reverse index entries
        for old_id in &entity.hotel_room_ids {
            let hr_uuid = old_id.non_nil_uuid();
            if let Some(rooms) = schedule.entities.event_rooms_by_hotel_room.get_mut(&hr_uuid) {
                rooms.retain(|&u| u != event_room_uuid);
            }
        }
        // Update forward backing field
        entity.hotel_room_ids = new_hotel_room_uuids
            .iter()
            .map(|&u| HotelRoomId::from_uuid(u))
            .collect();
        // Add event room to new hotel room reverse index entries
        for &hr_uuid in &new_hotel_room_uuids {
            schedule.entities.event_rooms_by_hotel_room
                .entry(hr_uuid)
                .or_default()
                .push(event_room_uuid);
        }
        Ok(())
    })]
    pub hotel_rooms: Vec<crate::entity::HotelRoomId>,

    #[computed_field(
        display = "Panels",
        description = "Panels scheduled in this event room"
    )]
    #[alias("panels", "scheduled_panels")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &EventRoomData| {
        use crate::entity::{InternalData, PanelId};
        let uuid = entity.uuid();
        let ids: Vec<PanelId> = schedule.entities.panels_by_event_room
            .get(&uuid)
            .map(|uuids| uuids.iter().map(|&u| PanelId::from_uuid(u)).collect())
            .unwrap_or_default();
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
    #[write(|schedule: &mut crate::schedule::Schedule, _entity: &mut EventRoomData, _value: crate::field::FieldValue| {
        let _ = schedule;
        Err(crate::field::FieldError::CannotStoreRelationshipField)
    })]
    pub panels: Vec<crate::entity::PanelId>,
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
