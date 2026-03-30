/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Room entity implementation

use crate::entity::EntityType;
use crate::field::{FieldSet, ValidationError};

/// Room entity
#[derive(Debug, Clone)]
pub struct Room {
    pub short_name: String,
    pub long_name: String,
    pub hotel_room: String, /// @TODO: Hotel room should be a separate entity, and the relationship should be an edge
    pub sort_key: i64, /// @TODO: Should be part of Hotel room
    pub is_break: bool,
}

/// Field constants for Room
pub mod room_fields {
    use super::Room;

    // Import macros from the dedicated macros module
    use crate::entity::macros::direct_field;
    use crate::{IndexableField, MatchStrength};

    // Short name field using new macro system
    direct_field!(
        SHORT_NAME,
        "Room Name",
        "Short room name",
        Room,
        short_name,
        String
    );

    impl IndexableField<Room> for SHORT_NAME {
        fn is_indexable(&self) -> bool {
            true
        }

        fn match_field(&self, query: &str, entity: &Room) -> Option<MatchStrength> {
            if entity.short_name.eq_ignore_ascii_case(query) {
                Some(MatchStrength::ExactMatch)
            } else if entity
                .short_name
                .to_lowercase()
                .contains(&query.to_lowercase())
            {
                Some(MatchStrength::StrongMatch)
            } else {
                Some(MatchStrength::NotMatch)
            }
        }

        fn index_priority(&self) -> u8 {
            180
        } // High priority for short name
    }

    // Long name field using new macro system
    direct_field!(
        LONG_NAME,
        "Long Name",
        "Long room name",
        Room,
        long_name,
        String
    );

    // Hotel room field using new macro system
    direct_field!(
        HOTEL_ROOM,
        "Hotel Room",
        "Physical hotel room",
        Room,
        hotel_room,
        String
    );

    // Sort key field using new macro system
    direct_field!(
        SORT_KEY,
        "Sort Key",
        "Room display sort order",
        Room,
        sort_key,
        i64
    );

    // Is break field using new macro system
    direct_field!(
        IS_BREAK,
        "Is Break",
        "Whether this room is a virtual break room",
        Room,
        is_break,
        bool
    );
}

impl Room {}

impl EntityType for Room {
    type Data = Room;

    const TYPE_NAME: &'static str = "room";

    fn field_set() -> &'static crate::field::field_set::FieldSet<Self> {
        use crate::entity::macros::field_set;
        use std::sync::LazyLock;

        static FIELD_SET: LazyLock<crate::field::field_set::FieldSet<Room>> = field_set!(Room, {
            fields: [
                &room_fields::SHORT_NAME => [],
                &room_fields::LONG_NAME => [],
                &room_fields::HOTEL_ROOM => [],
                &room_fields::SORT_KEY => [],
                &room_fields::IS_BREAK => []
            ],
            required: ["long_name"],
            indexable: [&room_fields::LONG_NAME, &room_fields::SHORT_NAME]
        });

        &FIELD_SET
    }

    fn validate(data: &Self::Data) -> Result<(), ValidationError> {
        if data.uid.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "uid".to_string(),
            });
        }
        if data.short_name.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "short_name".to_string(),
            });
        }
        Ok(())
    }
}
