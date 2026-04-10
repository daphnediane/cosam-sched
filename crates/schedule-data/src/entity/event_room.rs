/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoom entity — a logical room name used in the schedule.
//!
//! An event room is the name that appears in the **Room** column of the
//! Schedule sheet and must match the **Room Name** column of the Rooms sheet.
//! The physical hotel room it maps to (and any time-of-day partitioning) is
//! represented by [`EventRoomToHotelRoom`] edges with time-range attributes
//! (FEATURE-007).
//!
//! [`EventRoomToHotelRoom`]: crate::entity::EntityKind::EventRoomToHotelRoom

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
