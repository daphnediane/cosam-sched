/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Room entity implementation

use crate::EntityFields;

/// Room entity with enhanced EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
pub struct Room {
    #[field(display = "Room Name", description = "Short room name")]
    #[alias("short", "room_name")]
    #[indexable(priority = 180)]
    pub short_name: String,

    #[field(display = "Long Name", description = "Long room name")]
    #[alias("long", "full_name")]
    #[indexable(priority = 160)]
    #[required]
    pub long_name: String,

    #[field(display = "Hotel Room", description = "Physical hotel room")]
    #[alias("hotel", "location")]
    #[indexable(priority = 140)]
    pub hotel_room: String,

    #[field(display = "Sort Key", description = "Room display sort order")]
    #[alias("sort", "order")]
    pub sort_key: i64,

    #[field(
        display = "Is Break",
        description = "Whether this room is a virtual break room"
    )]
    #[alias("break_room", "virtual")]
    pub is_break: bool,
}
