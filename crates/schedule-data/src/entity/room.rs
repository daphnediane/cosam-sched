/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Room entity implementation

use crate::entity::EntityType;
use crate::field::{
    BooleanFieldType, FieldDescriptor, FieldError, FieldType, FieldValue, IntegerFieldType,
    StringFieldType, ValidationError,
};
use std::fmt;

/// Room ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomId(u64);

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "room-{}", self.0)
    }
}

/// Room entity
#[derive(Debug, Clone)]
pub struct Room {
    pub uid: String,
    pub short_name: String,
    pub long_name: String,
    pub hotel_room: String,
    pub sort_key: i64,
    pub is_break: bool,
}

/// Field constants for Room
pub mod room_fields {
    use super::Room;
    use crate::field::*;

    fn write_uid(room: &mut Room, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            room.uid = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_uid(room: &Room, _value: &FieldValue) -> Result<(), ValidationError> {
        if room.uid.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "uid".to_string(),
            });
        }
        Ok(())
    }

    fn write_short_name(room: &mut Room, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            room.short_name = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_short_name(room: &Room, _value: &FieldValue) -> Result<(), ValidationError> {
        if room.short_name.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "short_name".to_string(),
            });
        }
        Ok(())
    }

    fn write_long_name(room: &mut Room, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            room.long_name = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_long_name(room: &Room, _value: &FieldValue) -> Result<(), ValidationError> {
        if room.long_name.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "long_name".to_string(),
            });
        }
        Ok(())
    }

    fn write_hotel_room(room: &mut Room, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::String(v) = value {
            room.hotel_room = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_hotel_room(room: &Room, _value: &FieldValue) -> Result<(), ValidationError> {
        StringFieldType::validate(&room.hotel_room)
    }

    fn write_sort_key(room: &mut Room, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Integer(v) = value {
            room.sort_key = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_sort_key(room: &Room, _value: &FieldValue) -> Result<(), ValidationError> {
        IntegerFieldType::validate(&room.sort_key)
    }

    fn write_is_break(room: &mut Room, value: FieldValue) -> Result<(), FieldError> {
        if let FieldValue::Boolean(v) = value {
            room.is_break = v;
            return Ok(());
        }
        Err(FieldError::CannotStoreComputedField)
    }

    fn validate_is_break(room: &Room, _value: &FieldValue) -> Result<(), ValidationError> {
        BooleanFieldType::validate(&room.is_break)
    }

    fn uid_accessor(room: &Room) -> Option<FieldValue> {
        Some(FieldValue::String(room.uid.clone()))
    }

    fn short_name_accessor(room: &Room) -> Option<FieldValue> {
        Some(FieldValue::String(room.short_name.clone()))
    }

    fn long_name_accessor(room: &Room) -> Option<FieldValue> {
        Some(FieldValue::String(room.long_name.clone()))
    }

    fn hotel_room_accessor(room: &Room) -> Option<FieldValue> {
        Some(FieldValue::String(room.hotel_room.clone()))
    }

    fn sort_key_accessor(room: &Room) -> Option<FieldValue> {
        Some(FieldValue::Integer(room.sort_key))
    }

    fn is_break_accessor(room: &Room) -> Option<FieldValue> {
        Some(FieldValue::Boolean(room.is_break))
    }

    pub static UID: FieldDescriptor<Room> = FieldDescriptor {
        name: "uid",
        display_name: "UID",
        description: "Unique identifier for the room",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(uid_accessor),
        writer: Some(write_uid),
        validator: Some(validate_uid),
    };

    pub static SHORT_NAME: FieldDescriptor<Room> = FieldDescriptor {
        name: "short_name",
        display_name: "Room Name",
        description: "Short room name",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(short_name_accessor),
        writer: Some(write_short_name),
        validator: Some(validate_short_name),
    };

    pub static LONG_NAME: FieldDescriptor<Room> = FieldDescriptor {
        name: "long_name",
        display_name: "Long Name",
        description: "Long room name",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(long_name_accessor),
        writer: Some(write_long_name),
        validator: Some(validate_long_name),
    };

    pub static HOTEL_ROOM: FieldDescriptor<Room> = FieldDescriptor {
        name: "hotel_room",
        display_name: "Hotel Room",
        description: "Physical hotel room",
        required: true,
        field_type: FieldTypeEnum::String(StringFieldType),
        reader: FieldReader::Direct(hotel_room_accessor),
        writer: Some(write_hotel_room),
        validator: Some(validate_hotel_room),
    };

    pub static SORT_KEY: FieldDescriptor<Room> = FieldDescriptor {
        name: "sort_key",
        display_name: "Sort Key",
        description: "Room display sort order",
        required: true,
        field_type: FieldTypeEnum::Integer(IntegerFieldType),
        reader: FieldReader::Direct(sort_key_accessor),
        writer: Some(write_sort_key),
        validator: Some(validate_sort_key),
    };

    pub static IS_BREAK: FieldDescriptor<Room> = FieldDescriptor {
        name: "is_break",
        display_name: "Is Break",
        description: "Whether this room is a virtual break room",
        required: true,
        field_type: FieldTypeEnum::Boolean(BooleanFieldType),
        reader: FieldReader::Direct(is_break_accessor),
        writer: Some(write_is_break),
        validator: Some(validate_is_break),
    };
}

impl Room {
    pub fn all_fields() -> &'static [FieldDescriptor<Room>] {
        use std::sync::LazyLock;

        static FIELDS: LazyLock<Vec<FieldDescriptor<Room>>> = LazyLock::new(|| {
            vec![
                room_fields::UID,
                room_fields::SHORT_NAME,
                room_fields::LONG_NAME,
                room_fields::HOTEL_ROOM,
                room_fields::SORT_KEY,
                room_fields::IS_BREAK,
            ]
        });

        FIELDS.as_slice()
    }
}

impl EntityType for Room {
    type Data = Room;

    const TYPE_NAME: &'static str = "room";

    fn field_set() -> &'static crate::field::field_set::FieldSet<Self> {
        use crate::entity::macros::field_set;
        use std::sync::LazyLock;

        static FIELD_SET: LazyLock<crate::field::field_set::FieldSet<Room>> = field_set!(Room, {
            fields: [
                &room_fields::UID,
                &room_fields::SHORT_NAME,
                &room_fields::LONG_NAME,
                &room_fields::HOTEL_ROOM,
                &room_fields::SORT_KEY,
                &room_fields::IS_BREAK
            ],
            required: ["uid", "short_name"]
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
