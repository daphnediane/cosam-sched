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
}
