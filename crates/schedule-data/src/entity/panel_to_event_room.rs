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

// ---------------------------------------------------------------------------
// Convenience queries on PanelToEventRoomEntityType
// ---------------------------------------------------------------------------

impl PanelToEventRoomEntityType {
    /// The event room assigned to a panel (at most one; takes first outgoing).
    pub fn event_room_of(
        storage: &crate::schedule::EntityStorage,
        panel: NonNilUuid,
    ) -> Option<crate::entity::EventRoomId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .outgoing(panel)
            .first()
            .and_then(|edge_uuid| map.get(edge_uuid))
            .map(|edge| crate::entity::EventRoomId::from(edge.to_uuid()))
    }

    /// Panels assigned to an event room (incoming edges).
    pub fn panels_in(
        storage: &crate::schedule::EntityStorage,
        event_room: NonNilUuid,
    ) -> Vec<crate::entity::PanelId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .incoming(event_room)
            .iter()
            .filter_map(|edge_uuid| map.get(edge_uuid))
            .map(|edge| crate::entity::PanelId::from(edge.from_uuid()))
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
