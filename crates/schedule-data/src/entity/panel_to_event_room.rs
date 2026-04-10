/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToEventRoom edge-entity implementation.
//!
//! Connects a panel to the event room it is assigned to.  A panel has at most
//! one event room; replacing the edge replaces the assignment.

use crate::EntityFields;
use uuid::NonNilUuid;

/// PanelToEventRoom edge-entity.
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PanelToEventRoom)]
pub struct PanelToEventRoom {
    /// UUID of the panel (from side).
    #[field(display = "Panel UUID", description = "UUID of the panel")]
    #[required]
    #[edge_from(Panel)]
    pub panel_uuid: NonNilUuid,

    /// UUID of the event room (to side).
    #[field(display = "Event Room UUID", description = "UUID of the event room")]
    #[required]
    #[edge_to(EventRoom)]
    pub event_room_uuid: NonNilUuid,
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
    fn panel_to_event_room_id_round_trip() {
        let id = PanelToEventRoomId::from(nn(1));
        assert_eq!(NonNilUuid::from(id), nn(1));
    }

    #[test]
    fn panel_to_event_room_id_try_from_nil_returns_none() {
        assert!(PanelToEventRoomId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn panel_to_event_room_id_display() {
        let id = PanelToEventRoomId::from(nn(1));
        assert_eq!(
            id.to_string(),
            "panel-to-event-room-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn panel_to_event_room_data_accessors() {
        let data = PanelToEventRoomData {
            entity_uuid: nn(3),
            panel_uuid: nn(1),
            event_room_uuid: nn(2),
        };
        assert_eq!(data.panel_id().non_nil_uuid(), nn(1));
        assert_eq!(data.event_room_id().non_nil_uuid(), nn(2));
    }
}
