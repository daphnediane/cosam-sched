/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field system with type-safe extraction, validation, and matching

pub mod field_set;
pub mod matching;
pub mod traits;
pub mod types;
pub mod update_logic;
pub mod validation;

use std::collections::HashMap;
use std::fmt;

// Re-export core field types
pub use field_set::*;
pub use matching::*;
pub use traits::*;
pub use types::*;
pub use update_logic::*;
pub use validation::*;

use uuid::NonNilUuid;

/// Universal field value type for generic operations
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(chrono::NaiveDateTime),
    Duration(chrono::Duration),
    List(Vec<FieldValue>),
    Map(HashMap<String, FieldValue>),
    OptionalString(Option<String>),
    OptionalInteger(Option<i64>),
    OptionalFloat(Option<f64>),
    OptionalBoolean(Option<bool>),
    OptionalDateTime(Option<chrono::NaiveDateTime>),
    OptionalDuration(Option<chrono::Duration>),
    NonNilUuid(NonNilUuid),
    PanelIdentifier(crate::entity::PanelId),
    PanelTypeIdentifier(crate::entity::PanelTypeId),
    EventRoomIdentifier(crate::entity::EventRoomId),
    HotelRoomIdentifier(crate::entity::HotelRoomId),
    PresenterIdentifier(crate::entity::PresenterId),
}

impl FieldValue {
    /// Create a FieldValue::List of PanelIdentifier from a Vec<PanelId>.
    pub fn panel_list(ids: Vec<crate::entity::PanelId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::PanelIdentifier(id))
                .collect(),
        )
    }

    /// Create a FieldValue::List of PanelTypeIdentifier from a Vec<PanelTypeId>.
    pub fn panel_type_list(ids: Vec<crate::entity::PanelTypeId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::PanelTypeIdentifier(id))
                .collect(),
        )
    }

    /// Create a FieldValue::List of EventRoomIdentifier from a Vec<EventRoomId>.
    pub fn event_room_list(ids: Vec<crate::entity::EventRoomId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::EventRoomIdentifier(id))
                .collect(),
        )
    }

    /// Create a FieldValue::List of HotelRoomIdentifier from a Vec<HotelRoomId>.
    pub fn hotel_room_list(ids: Vec<crate::entity::HotelRoomId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::HotelRoomIdentifier(id))
                .collect(),
        )
    }

    /// Create a FieldValue::List of PresenterIdentifier from a Vec<PresenterId>.
    pub fn presenter_list(ids: Vec<crate::entity::PresenterId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::PresenterIdentifier(id))
                .collect(),
        )
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::String(s) => write!(f, "{}", s),
            FieldValue::Integer(i) => write!(f, "{}", i),
            FieldValue::Float(fl) => write!(f, "{}", fl),
            FieldValue::Boolean(b) => write!(f, "{}", b),
            FieldValue::DateTime(dt) => write!(f, "{}", dt),
            FieldValue::Duration(d) => write!(f, "{}m", d.num_minutes()),
            FieldValue::List(list) => {
                write!(f, "[")?;
                for (i, item) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            FieldValue::Map(map) => {
                write!(f, "{{")?;
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
            FieldValue::OptionalString(opt_s) => match opt_s {
                Some(s) => write!(f, "{}", s),
                None => write!(f, "null"),
            },
            FieldValue::OptionalInteger(opt_i) => match opt_i {
                Some(i) => write!(f, "{}", i),
                None => write!(f, "null"),
            },
            FieldValue::OptionalFloat(opt_f) => match opt_f {
                Some(fl) => write!(f, "{}", fl),
                None => write!(f, "null"),
            },
            FieldValue::OptionalBoolean(opt_b) => match opt_b {
                Some(b) => write!(f, "{}", b),
                None => write!(f, "null"),
            },
            FieldValue::OptionalDateTime(opt_dt) => match opt_dt {
                Some(dt) => write!(f, "{}", dt),
                None => write!(f, "null"),
            },
            FieldValue::OptionalDuration(opt_d) => match opt_d {
                Some(d) => write!(f, "{}m", d.num_minutes()),
                None => write!(f, "null"),
            },
            FieldValue::NonNilUuid(uuid) => write!(f, "{}", uuid),
            FieldValue::PanelIdentifier(id) => write!(f, "{}", id),
            FieldValue::PanelTypeIdentifier(id) => write!(f, "{}", id),
            FieldValue::EventRoomIdentifier(id) => write!(f, "{}", id),
            FieldValue::HotelRoomIdentifier(id) => write!(f, "{}", id),
            FieldValue::PresenterIdentifier(id) => write!(f, "{}", id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::NonNilUuid;

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(uuid::Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        }
    }

    #[test]
    fn field_value_uuid_display() {
        let uuid = test_nn();
        let value = FieldValue::NonNilUuid(uuid);
        assert_eq!(format!("{}", value), "00000000-0000-0000-0000-000000000001");
    }

    #[test]
    fn field_value_uuid_clone_eq() {
        let uuid = test_nn();
        let a = FieldValue::NonNilUuid(uuid);
        let b = a.clone();
        assert_eq!(a, b);
    }
}
