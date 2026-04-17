/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use chrono::{Duration, NaiveDateTime};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// Universal value enum used for all field read/write operations.
///
/// `String` and `Text` are distinct variants so the CRDT layer can route
/// short scalars vs. long prose to the appropriate automerge operation type.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// Short text: codes, names, URLs, enum tags.
    String(std::string::String),
    /// Long prose: descriptions, bios, notes — routed to CRDT RGA storage.
    Text(std::string::String),
    /// Integer: counts, durations in minutes, sort keys.
    Integer(i64),
    /// Fractional value.
    Float(f64),
    /// Boolean flag.
    Boolean(bool),
    /// ISO-8601 date/time.
    DateTime(NaiveDateTime),
    /// Chrono duration.
    Duration(Duration),
    /// Single entity UUID reference (non-nil).
    NonNilUuid(Uuid),
    /// UUID or string tag for entity lookup — used by callers that may supply
    /// either a UUID or a natural-key string.
    EntityIdentifier(EntityIdentifier),
    /// Multi-value list.
    List(Vec<FieldValue>),
    /// Absent / unset.
    None,
}

/// A reference to an entity by either its UUID or a natural-key string.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityIdentifier {
    /// Exact UUID reference.
    Id(Uuid),
    /// Natural-key string (name, code, etc.).
    Name(std::string::String),
}

impl fmt::Display for EntityIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(uuid) => write!(f, "{uuid}"),
            Self::Name(name) => write!(f, "{name}"),
        }
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) | Self::Text(s) => write!(f, "{s}"),
            Self::Integer(n) => write!(f, "{n}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Boolean(b) => write!(f, "{b}"),
            Self::DateTime(dt) => write!(f, "{dt}"),
            Self::Duration(d) => write!(f, "{}m", d.num_minutes()),
            Self::NonNilUuid(uuid) => write!(f, "{uuid}"),
            Self::EntityIdentifier(ei) => write!(f, "{ei}"),
            Self::List(items) => {
                let parts: Vec<_> = items.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", parts.join(", "))
            }
            Self::None => write!(f, ""),
        }
    }
}

impl FieldValue {
    /// Consume `self` and return the inner `String` value, or a
    /// [`ConversionError`] if the variant is not `String`.
    pub fn into_string(self) -> Result<std::string::String, ConversionError> {
        match self {
            Self::String(s) => Ok(s),
            other => Err(ConversionError::WrongVariant {
                expected: "String",
                got: other.variant_name(),
            }),
        }
    }

    /// Consume `self` and return the inner `Text` value, or a
    /// [`ConversionError`] if the variant is not `Text`.
    pub fn into_text(self) -> Result<std::string::String, ConversionError> {
        match self {
            Self::Text(s) => Ok(s),
            other => Err(ConversionError::WrongVariant {
                expected: "Text",
                got: other.variant_name(),
            }),
        }
    }

    /// Consume `self` and return the inner `Integer` value.
    pub fn into_integer(self) -> Result<i64, ConversionError> {
        match self {
            Self::Integer(n) => Ok(n),
            other => Err(ConversionError::WrongVariant {
                expected: "Integer",
                got: other.variant_name(),
            }),
        }
    }

    /// Consume `self` and return the inner `Float` value.
    pub fn into_float(self) -> Result<f64, ConversionError> {
        match self {
            Self::Float(v) => Ok(v),
            other => Err(ConversionError::WrongVariant {
                expected: "Float",
                got: other.variant_name(),
            }),
        }
    }

    /// Consume `self` and return the inner `Boolean` value.
    pub fn into_bool(self) -> Result<bool, ConversionError> {
        match self {
            Self::Boolean(b) => Ok(b),
            other => Err(ConversionError::WrongVariant {
                expected: "Boolean",
                got: other.variant_name(),
            }),
        }
    }

    /// Consume `self` and return the inner `DateTime` value.
    pub fn into_datetime(self) -> Result<NaiveDateTime, ConversionError> {
        match self {
            Self::DateTime(dt) => Ok(dt),
            other => Err(ConversionError::WrongVariant {
                expected: "DateTime",
                got: other.variant_name(),
            }),
        }
    }

    /// Consume `self` and return the inner `Duration` value.
    pub fn into_duration(self) -> Result<Duration, ConversionError> {
        match self {
            Self::Duration(d) => Ok(d),
            other => Err(ConversionError::WrongVariant {
                expected: "Duration",
                got: other.variant_name(),
            }),
        }
    }

    /// Consume `self` and return the inner `NonNilUuid` value.
    pub fn into_non_nil_uuid(self) -> Result<Uuid, ConversionError> {
        match self {
            Self::NonNilUuid(uuid) => Ok(uuid),
            other => Err(ConversionError::WrongVariant {
                expected: "NonNilUuid",
                got: other.variant_name(),
            }),
        }
    }

    /// Consume `self` and return the inner `List` value.
    pub fn into_list(self) -> Result<Vec<FieldValue>, ConversionError> {
        match self {
            Self::List(items) => Ok(items),
            other => Err(ConversionError::WrongVariant {
                expected: "List",
                got: other.variant_name(),
            }),
        }
    }

    /// Returns `true` if this value is `None`.
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    fn variant_name(&self) -> &'static str {
        match self {
            Self::String(_) => "String",
            Self::Text(_) => "Text",
            Self::Integer(_) => "Integer",
            Self::Float(_) => "Float",
            Self::Boolean(_) => "Boolean",
            Self::DateTime(_) => "DateTime",
            Self::Duration(_) => "Duration",
            Self::NonNilUuid(_) => "NonNilUuid",
            Self::EntityIdentifier(_) => "EntityIdentifier",
            Self::List(_) => "List",
            Self::None => "None",
        }
    }
}

/// How a field maps to CRDT storage in Phase 4.
///
/// Annotations are baked in from Phase 2 so no entity structs need changing
/// when automerge integration lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrdtFieldType {
    /// Last-write-wins scalar via `put` / `get` (automerge LWW).
    Scalar,
    /// Prose RGA text via `splice_text` / `text` (automerge RGA).
    Text,
    /// OR-Set equivalent list via `insert` / `delete` / `list` (automerge list).
    List,
    /// Computed from relationships; not stored in CRDT — lives only in RAM.
    Derived,
}

/// Top-level error for field operations.
#[derive(Debug, Error)]
pub enum FieldError {
    /// Type conversion failed.
    #[error("conversion error: {0}")]
    Conversion(#[from] ConversionError),
    /// Field value failed validation.
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),
    /// Field value failed verification after batch write.
    #[error("verification error: {0}")]
    Verification(#[from] VerificationError),
    /// Field is read-only (no write_fn).
    #[error("field '{name}' is read-only")]
    ReadOnly { name: &'static str },
    /// Field is write-only (no read_fn).
    #[error("field '{name}' is write-only")]
    WriteOnly { name: &'static str },
    /// Entity not found in the schedule.
    #[error("field '{name}': entity not found")]
    NotFound { name: &'static str },
}

/// Type conversion failure — wrong `FieldValue` variant or parse error.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// Caller supplied the wrong variant.
    #[error("expected {expected}, got {got}")]
    WrongVariant {
        expected: &'static str,
        got: &'static str,
    },
    /// A string could not be parsed into the target type.
    #[error("parse error: {message}")]
    ParseError { message: std::string::String },
}

/// Value fails field constraints.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// A required field was absent or empty.
    #[error("field '{field}' is required")]
    Required { field: &'static str },
    /// Value is outside the allowed range.
    #[error("field '{field}': value out of range — {message}")]
    OutOfRange {
        field: &'static str,
        message: std::string::String,
    },
    /// Value violates an application-specific constraint.
    #[error("field '{field}': {message}")]
    Constraint {
        field: &'static str,
        message: std::string::String,
    },
}

/// Verification failure — field value changed during batch write.
#[derive(Debug, Error)]
pub enum VerificationError {
    /// The field value was changed by another write in the same batch.
    #[error("field '{field}': value changed during batch write — requested {requested:?}, actual {actual:?}")]
    ValueChanged {
        field: &'static str,
        requested: FieldValue,
        actual: FieldValue,
    },
    /// The field cannot be verified (e.g., `ReRead` requested but field is write-only).
    #[error("field '{field}': cannot be verified — read not available for re-read verification")]
    NotVerifiable { field: &'static str },
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_datetime() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 8, 1)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
    }

    #[test]
    fn test_field_value_display_string() {
        assert_eq!(FieldValue::String("hello".into()).to_string(), "hello");
    }

    #[test]
    fn test_field_value_display_text() {
        assert_eq!(FieldValue::Text("bio".into()).to_string(), "bio");
    }

    #[test]
    fn test_field_value_display_integer() {
        assert_eq!(FieldValue::Integer(42).to_string(), "42");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_field_value_display_float() {
        assert_eq!(FieldValue::Float(3.14).to_string(), "3.14");
    }

    #[test]
    fn test_field_value_display_boolean() {
        assert_eq!(FieldValue::Boolean(true).to_string(), "true");
        assert_eq!(FieldValue::Boolean(false).to_string(), "false");
    }

    #[test]
    fn test_field_value_display_datetime() {
        let v = FieldValue::DateTime(sample_datetime());
        assert!(v.to_string().contains("2026"));
    }

    #[test]
    fn test_field_value_display_duration() {
        let d = Duration::try_minutes(90).unwrap();
        assert_eq!(FieldValue::Duration(d).to_string(), "90m");
    }

    #[test]
    fn test_field_value_display_none() {
        assert_eq!(FieldValue::None.to_string(), "");
    }

    #[test]
    fn test_field_value_display_list() {
        let v = FieldValue::List(vec![
            FieldValue::String("a".into()),
            FieldValue::String("b".into()),
        ]);
        assert_eq!(v.to_string(), "[a, b]");
    }

    #[test]
    fn test_into_string_ok() {
        let v = FieldValue::String("test".into());
        assert_eq!(v.into_string().unwrap(), "test");
    }

    #[test]
    fn test_into_string_wrong_variant() {
        let v = FieldValue::Integer(1);
        assert!(v.into_string().is_err());
    }

    #[test]
    fn test_into_integer_ok() {
        assert_eq!(FieldValue::Integer(7).into_integer().unwrap(), 7);
    }

    #[test]
    fn test_into_bool_ok() {
        assert!(FieldValue::Boolean(true).into_bool().unwrap());
    }

    #[test]
    fn test_is_none() {
        assert!(FieldValue::None.is_none());
        assert!(!FieldValue::Integer(0).is_none());
    }

    #[test]
    fn test_into_list_ok() {
        let v = FieldValue::List(vec![FieldValue::Integer(1)]);
        assert_eq!(v.into_list().unwrap().len(), 1);
    }

    #[test]
    fn test_crdt_field_type_variants() {
        let variants = [
            CrdtFieldType::Scalar,
            CrdtFieldType::Text,
            CrdtFieldType::List,
            CrdtFieldType::Derived,
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_field_error_display_read_only() {
        let e = FieldError::ReadOnly { name: "prefix" };
        assert!(e.to_string().contains("prefix"));
    }

    #[test]
    fn test_validation_error_required() {
        let e = ValidationError::Required { field: "name" };
        assert!(e.to_string().contains("name"));
    }

    #[test]
    fn test_field_value_clone_and_partial_eq() {
        let v = FieldValue::String("clone_me".into());
        assert_eq!(v.clone(), v);
    }
}
