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
}
