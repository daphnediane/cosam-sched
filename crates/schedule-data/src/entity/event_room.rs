/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoom entity implementation

use crate::EntityFields;

/// EventRoom entity for event/convention rooms
#[derive(EntityFields, Debug, Clone)]
pub struct EventRoom {
    #[field(display = "Room Name", description = "Short room name")]
    #[alias("short", "room_name")]
    #[indexable(priority = 180)]
    pub short_name: String,

    #[field(display = "Long Name", description = "Long room name")]
    #[alias("long", "full_name")]
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
    #[read(|schedule: &crate::schedule::Schedule, entity_id: crate::entity::EntityId, entity: &EventRoom| {
        let panel_ids = schedule.find_related::<crate::entity::PanelEntityType>(
            entity_id, 
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

    #[computed_field(
        name = "hotel_room",
        display = "Hotel Room",
        description = "Hotel room that maps to this event room"
    )]
    #[alias("hotel", "physical_room")]
    #[read(|schedule: &crate::schedule::Schedule, entity_id: crate::entity::EntityId, entity: &EventRoom| {
        let hotel_room_ids = schedule.find_related::<crate::entity::HotelRoomEntityType>(
            entity_id, 
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

    #[computed_field(
        name = "sort_key",
        display = "Sort Key",
        description = "Sort key from hotel room"
    )]
    #[alias("sort", "order")]
    #[read(|schedule: &crate::schedule::Schedule, entity_id: crate::entity::EntityId, entity: &EventRoom| {
        let hotel_room_ids = schedule.find_related::<crate::entity::HotelRoomEntityType>(
            entity_id, 
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
    
    // Internal metadata field for entities with only computed fields
    #[field(display = "Internal Version", description = "Internal struct version for compatibility")]
    pub _version: u8,
}
