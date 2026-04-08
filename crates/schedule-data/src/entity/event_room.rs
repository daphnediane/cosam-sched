/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoom entity implementation

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::{NonNilUuid, Uuid};

/// EventRoom ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventRoomId(NonNilUuid);

impl fmt::Display for EventRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event-room-{}", self.0)
    }
}

impl From<NonNilUuid> for EventRoomId {
    fn from(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl From<EventRoomId> for NonNilUuid {
    fn from(id: EventRoomId) -> NonNilUuid {
        id.0
    }
}

impl From<EventRoomId> for Uuid {
    fn from(id: EventRoomId) -> Uuid {
        id.0.into()
    }
}

impl crate::entity::TypedId for EventRoomId {
    type EntityType = EventRoomEntityType;
    fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }
    fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl EventRoomId {
    /// Get the NonNilUuid from this ID
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }

    /// Get the raw UUID from this ID
    pub fn uuid(&self) -> Uuid {
        self.0.into()
    }

    /// Create an EventRoomId from a NonNilUuid (infallible)
    pub fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }

    /// Try to create an EventRoomId from a raw UUID (boundary use only)
    pub fn try_from_raw_uuid(uuid: Uuid) -> Option<Self> {
        NonNilUuid::new(uuid).map(Self)
    }
}

/// EventRoom entity for event/convention rooms
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(EventRoom)]
pub struct EventRoom {
    #[field(display = "Room Name", description = "Short room name")]
    #[alias("short", "Room_Name", "room_name")]
    #[indexable(priority = 180)]
    pub short_name: String,

    #[field(display = "Long Name", description = "Long room name")]
    #[alias("long", "Long_Name", "full_name")]
    #[indexable(priority = 160)]
    #[required]
    pub long_name: String,

    #[field(
        display = "Is Break",
        description = "Whether this room is a virtual break room"
    )]
    #[alias("break_room", "virtual")]
    pub is_break: bool,

    #[computed_field(
        name = "get_panels",
        display = "Panels in this room",
        description = "Panels scheduled in this room"
    )]
    #[alias("panels", "scheduled_panels")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &EventRoomData| {
        let panel_ids = schedule.find_related::<crate::entity::PanelEntityType>(
            entity.entity_uuid,
            crate::edge::EdgeType::PanelToEventRoom,
            crate::schedule::RelationshipDirection::Incoming
        );
        Some(crate::field::FieldValue::List(
            schedule.get_entity_names::<crate::entity::PanelEntityType>(&panel_ids)
                .into_iter()
                .map(crate::field::FieldValue::String)
                .collect()
        ))
    })]
    pub get_panels: Vec<crate::entity::PanelId>,

    #[computed_field(
        name = "hotel_room",
        display = "Hotel Room",
        description = "Hotel room that maps to this event room"
    )]
    #[alias(
        "hotel_room",
        "Hotel_Room",
        "HotelRoom",
        "hotel",
        "physical_room",
        "Building"
    )]
    #[read(|schedule: &crate::schedule::Schedule, entity: &EventRoomData| {
        let hotel_room_ids = schedule.find_related::<crate::entity::HotelRoomEntityType>(
            entity.entity_uuid,
            crate::edge::EdgeType::EventRoomToHotelRoom,
            crate::schedule::RelationshipDirection::Outgoing
        );
        if let Some(hotel_room_id) = hotel_room_ids.first() {
            if let Some(hotel_room) = schedule.get_entity::<crate::entity::HotelRoomEntityType>(*hotel_room_id) {
                return Some(crate::field::FieldValue::String(hotel_room.hotel_room.clone()));
            }
        }
        None
    })]
    pub hotel_room: Option<String>,

    #[computed_field(
        name = "sort_key",
        display = "Sort Key",
        description = "Sort key from hotel room"
    )]
    #[alias("sort_key", "Sort_Key", "SortKey", "sort", "order")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &EventRoomData| {
        let hotel_room_ids = schedule.find_related::<crate::entity::HotelRoomEntityType>(
            entity.entity_uuid,
            crate::edge::EdgeType::EventRoomToHotelRoom,
            crate::schedule::RelationshipDirection::Outgoing
        );
        if let Some(hotel_room_id) = hotel_room_ids.first() {
            if let Some(hotel_room) = schedule.get_entity::<crate::entity::HotelRoomEntityType>(*hotel_room_id) {
                return Some(crate::field::FieldValue::Integer(hotel_room.sort_key));
            }
        }
        None
    })]
    pub sort_key_computed: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
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
    fn event_room_id_try_from_nil_uuid_returns_none() {
        assert!(EventRoomId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn event_room_id_display() {
        let id = EventRoomId::from(test_nn());
        assert_eq!(
            id.to_string(),
            "event-room-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn event_room_id_serde_round_trip() {
        let id = EventRoomId::from(test_nn());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000001\"");
        let back: EventRoomId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
