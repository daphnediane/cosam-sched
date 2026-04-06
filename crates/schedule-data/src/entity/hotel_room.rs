/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! HotelRoom entity implementation

use crate::EntityFields;

/// HotelRoom entity for physical hotel room information
#[derive(EntityFields, Debug, Clone)]
pub struct HotelRoom {
    #[field(display = "Hotel Room", description = "Physical hotel room")]
    #[alias("hotel", "location")]
    #[indexable(priority = 140)]
    pub hotel_room: String,

    #[field(display = "Sort Key", description = "Room display sort order")]
    #[alias("sort", "order")]
    pub sort_key: i64,

    #[computed_field(
        display = "Event Rooms",
        description = "All event rooms that map to this hotel room"
    )]
    #[alias("event_rooms", "rooms", "mapped_rooms")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &HotelRoomData| {
        let event_room_ids = schedule.find_related::<crate::entity::EventRoomEntityType>(
            entity.entity_id,
            crate::edge::EdgeType::EventRoomToHotelRoom,
            crate::schedule::RelationshipDirection::Incoming
        );
        Some(crate::field::FieldValue::List(
            schedule.get_entity_names::<crate::entity::EventRoomEntityType>(&event_room_ids)
                .into_iter()
                .map(crate::field::FieldValue::String)
                .collect()
        ))
    })]
    pub event_rooms: Vec<crate::entity::EntityId>,
}
