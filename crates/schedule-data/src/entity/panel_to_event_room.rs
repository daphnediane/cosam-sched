/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToEventRoom edge-entity implementation
//!
//! This edge type connects panels to their event rooms.
//! As an edge-entity, it has its own UUID and can store metadata
//! like whether this is the primary room for multi-room panels.

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::{NonNilUuid, Uuid};

/// PanelToEventRoom edge ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PanelToEventRoomId(NonNilUuid);

impl PanelToEventRoomId {
    /// Get the NonNilUuid from this ID
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }

    /// Get the raw UUID from this ID
    pub fn uuid(&self) -> Uuid {
        self.0.into()
    }

    /// Create a PanelToEventRoomId from a NonNilUuid (infallible)
    pub fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }

    /// Try to create a PanelToEventRoomId from a raw UUID (boundary use only)
    pub fn try_from_raw_uuid(uuid: Uuid) -> Option<Self> {
        NonNilUuid::new(uuid).map(Self)
    }
}

impl fmt::Display for PanelToEventRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panel-to-event-room-{}", self.0)
    }
}

impl From<NonNilUuid> for PanelToEventRoomId {
    fn from(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl From<PanelToEventRoomId> for NonNilUuid {
    fn from(id: PanelToEventRoomId) -> NonNilUuid {
        id.0
    }
}

impl From<PanelToEventRoomId> for Uuid {
    fn from(id: PanelToEventRoomId) -> Uuid {
        id.0.into()
    }
}

impl crate::entity::TypedId for PanelToEventRoomId {
    type EntityType = PanelToEventRoomEntityType;
    fn non_nil_uuid(&self) -> NonNilUuid { self.0 }
    fn from_uuid(uuid: NonNilUuid) -> Self { Self(uuid) }
}

/// PanelToEventRoom edge-entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PanelToEventRoom)]
pub struct PanelToEventRoom {
    /// UUID of the panel (from side)
    #[field(display = "Panel UUID", description = "UUID of the panel")]
    #[required]
    pub panel_uuid: NonNilUuid,

    /// UUID of the event room (to side)
    #[field(display = "Event Room UUID", description = "UUID of the event room")]
    #[required]
    pub event_room_uuid: NonNilUuid,

    // @todo - This is an extension not part of our current data

    /// Whether this is the primary room for the panel
    #[field(display = "Is Primary Room", description = "Whether this is the primary room")]
    pub is_primary_room: bool,
}

impl PanelToEventRoomData {
    /// Get the panel ID from this edge
    pub fn panel_id(&self) -> crate::entity::PanelId {
        crate::entity::PanelId::from_uuid(self.panel_uuid)
    }

    /// Get the event room ID from this edge
    pub fn event_room_id(&self) -> crate::entity::EventRoomId {
        crate::entity::EventRoomId::from_uuid(self.event_room_uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nn() -> NonNilUuid {
        unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])) }
    }

    #[test]
    fn panel_to_event_room_id_from_uuid() {
        let nn = test_nn();
        let id = PanelToEventRoomId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn panel_to_event_room_id_try_from_nil_uuid_returns_none() {
        assert!(PanelToEventRoomId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn panel_to_event_room_id_display() {
        let id = PanelToEventRoomId::from(test_nn());
        assert_eq!(id.to_string(), "panel-to-event-room-00000000-0000-0000-0000-000000000001");
    }

    #[test]
    fn panel_to_event_room_data_ids() {
        let panel_uuid = unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])) };
        let event_room_uuid = unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2])) };

        let data = PanelToEventRoomData {
            entity_uuid: test_nn(),
            panel_uuid,
            event_room_uuid,
            is_primary_room: true,
        };

        assert_eq!(data.panel_id().non_nil_uuid(), panel_uuid);
        assert_eq!(data.event_room_id().non_nil_uuid(), event_room_uuid);
    }
}
