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

use std::fmt;

// Re-export core field types
pub use field_set::*;
pub use matching::*;
pub use traits::*;
pub use types::*;
pub use update_logic::*;
pub use validation::*;

use uuid::NonNilUuid;

use crate::entity::EntityUUID;

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
    /// Generic optional wrapper - allows nesting (e.g., Optional(Some(String("foo"))))
    Optional(Option<Box<FieldValue>>),
    NonNilUuid(NonNilUuid),
    /// Generic entity identifier for any entity type.
    /// Use `EntityUUID::to_typed_id()` to extract a specific typed ID.
    EntityIdentifier(EntityUUID),
}

impl From<Option<String>> for FieldValue {
    fn from(opt: Option<String>) -> Self {
        Self::Optional(opt.map(|s| Box::new(Self::String(s))))
    }
}

impl From<Option<i64>> for FieldValue {
    fn from(opt: Option<i64>) -> Self {
        Self::Optional(opt.map(|i| Box::new(Self::Integer(i))))
    }
}

impl From<Option<f64>> for FieldValue {
    fn from(opt: Option<f64>) -> Self {
        Self::Optional(opt.map(|f| Box::new(Self::Float(f))))
    }
}

impl From<Option<bool>> for FieldValue {
    fn from(opt: Option<bool>) -> Self {
        Self::Optional(opt.map(|b| Box::new(Self::Boolean(b))))
    }
}

impl From<Option<chrono::NaiveDateTime>> for FieldValue {
    fn from(opt: Option<chrono::NaiveDateTime>) -> Self {
        Self::Optional(opt.map(|dt| Box::new(Self::DateTime(dt))))
    }
}

impl From<Option<chrono::Duration>> for FieldValue {
    fn from(opt: Option<chrono::Duration>) -> Self {
        Self::Optional(opt.map(|d| Box::new(Self::Duration(d))))
    }
}

impl FieldValue {
    /// Create a FieldValue::List of EntityIdentifier from a `Vec<PanelId>`.
    pub fn panel_list(ids: Vec<crate::entity::PanelId>) -> Self {
        Self::List(ids.into_iter().map(|id| Self::EntityIdentifier(EntityUUID::Panel(id))).collect())
    }

    /// Create a FieldValue::List of EntityIdentifier from a `Vec<PanelTypeId>`.
    pub fn panel_type_list(ids: Vec<crate::entity::PanelTypeId>) -> Self {
        Self::List(ids.into_iter().map(|id| Self::EntityIdentifier(EntityUUID::PanelType(id))).collect())
    }

    /// Create a FieldValue::List of EntityIdentifier from a `Vec<EventRoomId>`.
    pub fn event_room_list(ids: Vec<crate::entity::EventRoomId>) -> Self {
        Self::List(ids.into_iter().map(|id| Self::EntityIdentifier(EntityUUID::EventRoom(id))).collect())
    }

    /// Create a FieldValue::List of EntityIdentifier from a `Vec<HotelRoomId>`.
    pub fn hotel_room_list(ids: Vec<crate::entity::HotelRoomId>) -> Self {
        Self::List(ids.into_iter().map(|id| Self::EntityIdentifier(EntityUUID::HotelRoom(id))).collect())
    }

    /// Create a FieldValue::List of EntityIdentifier from a `Vec<PresenterId>`.
    pub fn presenter_list(ids: Vec<crate::entity::PresenterId>) -> Self {
        Self::List(ids.into_iter().map(|id| Self::EntityIdentifier(EntityUUID::Presenter(id))).collect())
    }

    /// Convert FieldValue to bool.
    ///
    /// - Boolean: returns the value directly
    /// - String: returns false for "false", "0", or empty string; true otherwise
    /// - Other: returns false
    pub fn as_bool(&self) -> bool {
        match self {
            FieldValue::Boolean(b) => *b,
            FieldValue::String(s) => !matches!(s.to_lowercase().as_str(), "false" | "0" | ""),
            _ => false,
        }
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
            FieldValue::Optional(opt) => match opt {
                Some(inner) => write!(f, "{}", inner),
                None => write!(f, "null"),
            },
            FieldValue::NonNilUuid(uuid) => write!(f, "{}", uuid),
            FieldValue::EntityIdentifier(euuid) => write!(f, "{:?}", euuid),
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
