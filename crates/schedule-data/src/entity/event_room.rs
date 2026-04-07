/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoom entity implementation

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// EventRoom ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventRoomId(Uuid);

impl fmt::Display for EventRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event-room-{}", self.0)
    }
}

impl From<Uuid> for EventRoomId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<EventRoomId> for Uuid {
    fn from(id: EventRoomId) -> Uuid {
        id.0
    }
}

/// EventRoom entity for event/convention rooms
#[derive(EntityFields, Debug, Clone)]
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
            entity.entity_id,
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

    #[test]
    fn event_room_id_from_uuid() {
        let uuid = Uuid::nil();
        let id = EventRoomId::from(uuid);
        assert_eq!(Uuid::from(id), uuid);
    }

    #[test]
    fn event_room_id_display() {
        let id = EventRoomId::from(Uuid::nil());
        assert_eq!(
            id.to_string(),
            "event-room-00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn event_room_id_serde_round_trip() {
        let id = EventRoomId::from(Uuid::nil());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000000\"");
        let back: EventRoomId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
