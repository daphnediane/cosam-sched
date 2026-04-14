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
    /// Short string scalar — LWW on merge. Use for names, ranks, IDs, etc.
    String(String),
    /// Long prose string — character-level RGA on merge. Use for description,
    /// note, and other free-text fields where concurrent offline edits must
    /// both survive. Reads as a plain `String`; the CRDT layer routes writes
    /// through `splice_text` rather than `put()`.
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(chrono::NaiveDateTime),
    Duration(chrono::Duration),
    List(Vec<FieldValue>),
    NonNilUuid(NonNilUuid),
    /// Generic entity identifier for any entity type.
    /// Use `EntityUUID::to_typed_id()` to extract a specific typed ID.
    EntityIdentifier(EntityUUID),
    /// Explicit null/empty value
    None,
}

impl FieldValue {
    /// Convert Option<String> to Option<FieldValue>.
    pub fn from_option_string(opt: Option<String>) -> Option<Self> {
        opt.map(Self::String)
    }

    /// Convert Option<String> to Option<FieldValue::Text> (prose fields).
    pub fn from_option_text(opt: Option<String>) -> Option<Self> {
        opt.map(Self::Text)
    }

    /// Convert Option<i64> to Option<FieldValue>.
    pub fn from_option_integer(opt: Option<i64>) -> Option<Self> {
        opt.map(Self::Integer)
    }

    /// Convert Option<f64> to Option<FieldValue>.
    pub fn from_option_float(opt: Option<f64>) -> Option<Self> {
        opt.map(Self::Float)
    }

    /// Convert Option<bool> to Option<FieldValue>.
    pub fn from_option_boolean(opt: Option<bool>) -> Option<Self> {
        opt.map(Self::Boolean)
    }

    /// Convert Option<NaiveDateTime> to Option<FieldValue>.
    pub fn from_option_datetime(opt: Option<chrono::NaiveDateTime>) -> Option<Self> {
        opt.map(Self::DateTime)
    }

    /// Convert Option<Duration> to Option<FieldValue>.
    pub fn from_option_duration(opt: Option<chrono::Duration>) -> Option<Self> {
        opt.map(Self::Duration)
    }

    /// Create a FieldValue::List of EntityIdentifier from a `Vec<PanelId>`.
    pub fn panel_list(ids: Vec<crate::entity::PanelId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::EntityIdentifier(EntityUUID::Panel(id)))
                .collect(),
        )
    }

    /// Create a FieldValue::List of EntityIdentifier from a `Vec<PanelTypeId>`.
    pub fn panel_type_list(ids: Vec<crate::entity::PanelTypeId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::EntityIdentifier(EntityUUID::PanelType(id)))
                .collect(),
        )
    }

    /// Create a FieldValue::List of EntityIdentifier from a `Vec<EventRoomId>`.
    pub fn event_room_list(ids: Vec<crate::entity::EventRoomId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::EntityIdentifier(EntityUUID::EventRoom(id)))
                .collect(),
        )
    }

    /// Create a FieldValue::List of EntityIdentifier from a `Vec<HotelRoomId>`.
    pub fn hotel_room_list(ids: Vec<crate::entity::HotelRoomId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::EntityIdentifier(EntityUUID::HotelRoom(id)))
                .collect(),
        )
    }

    /// Create a FieldValue::List of EntityIdentifier from a `Vec<PresenterId>`.
    pub fn presenter_list(ids: Vec<crate::entity::PresenterId>) -> Self {
        Self::List(
            ids.into_iter()
                .map(|id| Self::EntityIdentifier(EntityUUID::Presenter(id)))
                .collect(),
        )
    }

    /// Convert FieldValue to bool.
    ///
    /// - Boolean: returns the value directly
    /// - String: returns false for "false", "0", or empty string; true otherwise
    /// - Other: returns false
    pub fn as_bool(&self) -> bool {
        match self {
            FieldValue::Boolean(b) => *b,
            FieldValue::String(s) | FieldValue::Text(s) => {
                !matches!(s.to_lowercase().as_str(), "false" | "0" | "")
            }
            _ => false,
        }
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::String(s) | FieldValue::Text(s) => write!(f, "{}", s),
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
            FieldValue::NonNilUuid(uuid) => write!(f, "{}", uuid),
            FieldValue::EntityIdentifier(euuid) => write!(f, "{:?}", euuid),
            FieldValue::None => write!(f, "null"),
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
