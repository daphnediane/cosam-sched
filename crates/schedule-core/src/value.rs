/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use chrono::{Duration, NaiveDateTime};
use std::fmt;
use thiserror::Error;

use crate::entity::RuntimeEntityId;

/// Base value types for fields.
///
/// `String` and `Text` are distinct variants so the CRDT layer can route
/// short scalars vs. long prose to the appropriate automerge operation type.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValueItem {
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
    /// Identifier for an entity
    EntityIdentifier(RuntimeEntityId),
}

/// Universal value enum used for all field read/write operations.
///
/// This is a wrapper around `FieldValueItem` that allows for single values or lists of values.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// FieldValue
    Single(FieldValueItem),
    /// Multi-value list.
    List(Vec<FieldValueItem>),
}

impl fmt::Display for FieldValueItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) | Self::Text(s) => write!(f, "{s}"),
            Self::Integer(n) => write!(f, "{n}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Boolean(b) => write!(f, "{b}"),
            Self::DateTime(dt) => write!(f, "{dt}"),
            Self::Duration(d) => write!(f, "{}m", d.num_minutes()),
            Self::EntityIdentifier(ei) => write!(f, "{ei}"),
        }
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(item) => write!(f, "{item}"),
            Self::List(items) => {
                let parts: Vec<_> = items.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", parts.join(", "))
            }
        }
    }
}

// @TOD: These will be superseded by FEATURE-038.md
impl FieldValueItem {
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

    /// Consume `self` and return a `RuntimeEntityId` value.
    pub fn into_entity_identifier(self) -> Result<RuntimeEntityId, ConversionError> {
        match self {
            Self::EntityIdentifier(id) => Ok(id),
            other => Err(ConversionError::WrongVariant {
                expected: "EntityIdentifier",
                got: other.variant_name(),
            }),
        }
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
            Self::EntityIdentifier(_) => "EntityIdentifier",
        }
    }
}

impl FieldValue {
    /// Returns `true` if this value is empty (no items).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Single(_) => false,
            Self::List(items) => items.is_empty(),
        }
    }

    /// Returns `true` if this value is a single item.
    #[must_use]
    pub fn is_single(&self) -> bool {
        match self {
            Self::Single(_) => true,
            Self::List(items) => items.len() == 1,
        }
    }

    /// Consume `self` and return the inner `List` value.
    pub fn into_list(self) -> Result<Vec<FieldValueItem>, ConversionError> {
        match self {
            Self::Single(item) => Ok(vec![item]),
            Self::List(items) => Ok(items),
        }
    }

    /// Consume `self` and return a single value.
    pub fn into_single(self) -> Result<FieldValueItem, ConversionError> {
        match self {
            Self::Single(item) => Ok(item),
            Self::List(items) => {
                if items.len() != 1 {
                    return Err(ConversionError::WrongVariant {
                        expected: "Single",
                        got: "List that is not exactly one item",
                    });
                }
                Ok(items[0].clone())
            }
        }
    }

    /// Consume `self` and return a String value.
    pub fn into_string(self) -> Result<String, ConversionError> {
        self.into_single()?.into_string()
    }

    /// Consume `self` and return a Text value.
    pub fn into_text(self) -> Result<String, ConversionError> {
        self.into_single()?.into_text()
    }

    /// Consume `self` and return an Integer value.
    pub fn into_integer(self) -> Result<i64, ConversionError> {
        self.into_single()?.into_integer()
    }

    /// Consume `self` and return a Float value.
    pub fn into_float(self) -> Result<f64, ConversionError> {
        self.into_single()?.into_float()
    }

    /// Consume `self` and return a Boolean value.
    pub fn into_bool(self) -> Result<bool, ConversionError> {
        self.into_single()?.into_bool()
    }

    /// Consume `self` and return a DateTime value.
    pub fn into_datetime(self) -> Result<chrono::NaiveDateTime, ConversionError> {
        self.into_single()?.into_datetime()
    }

    /// Consume `self` and return a Duration value.
    pub fn into_duration(self) -> Result<chrono::Duration, ConversionError> {
        self.into_single()?.into_duration()
    }

    /// Consume `self` and return an EntityIdentifier value.
    pub fn into_entity_identifier(self) -> Result<crate::entity::RuntimeEntityId, ConversionError> {
        self.into_single()?.into_entity_identifier()
    }
}

/// Scalar field type tags — the `Copy` type-level mirror of [`FieldValueItem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldTypeItem {
    String,
    Text,
    Integer,
    Float,
    Boolean,
    DateTime,
    Duration,
    /// Typed entity reference. The `&'static str` is the entity's `TYPE_NAME`.
    EntityIdentifier(&'static str),
}

/// Field type with cardinality — the `Copy` type-level mirror of [`FieldValue`].
///
/// `FieldType` retains an `Optional` variant because type declarations need to
/// distinguish "required scalar" from "optional scalar". At the value level,
/// absence is expressed as `Option<FieldValue>` returning `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    Single(FieldTypeItem),
    Optional(FieldTypeItem),
    List(FieldTypeItem),
}

impl fmt::Display for FieldTypeItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String => write!(f, "String"),
            Self::Text => write!(f, "Text"),
            Self::Integer => write!(f, "Integer"),
            Self::Float => write!(f, "Float"),
            Self::Boolean => write!(f, "Boolean"),
            Self::DateTime => write!(f, "DateTime"),
            Self::Duration => write!(f, "Duration"),
            Self::EntityIdentifier(name) => write!(f, "EntityIdentifier({name})"),
        }
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(t) => write!(f, "{t}"),
            Self::Optional(t) => write!(f, "{t}?"),
            Self::List(t) => write!(f, "List<{t}>"),
        }
    }
}

fn value_item_to_type_item(item: &FieldValueItem) -> FieldTypeItem {
    match item {
        FieldValueItem::String(_) => FieldTypeItem::String,
        FieldValueItem::Text(_) => FieldTypeItem::Text,
        FieldValueItem::Integer(_) => FieldTypeItem::Integer,
        FieldValueItem::Float(_) => FieldTypeItem::Float,
        FieldValueItem::Boolean(_) => FieldTypeItem::Boolean,
        FieldValueItem::DateTime(_) => FieldTypeItem::DateTime,
        FieldValueItem::Duration(_) => FieldTypeItem::Duration,
        FieldValueItem::EntityIdentifier(id) => FieldTypeItem::EntityIdentifier(id.type_name()),
    }
}

impl FieldType {
    /// Return the scalar item type, discarding cardinality.
    #[must_use]
    pub fn item_type(self) -> FieldTypeItem {
        match self {
            Self::Single(t) | Self::Optional(t) | Self::List(t) => t,
        }
    }

    #[must_use]
    pub fn is_single(self) -> bool {
        matches!(self, Self::Single(_))
    }

    #[must_use]
    pub fn is_optional(self) -> bool {
        matches!(self, Self::Optional(_))
    }

    #[must_use]
    pub fn is_list(self) -> bool {
        matches!(self, Self::List(_))
    }

    /// Infer a `FieldType::Single` or `FieldType::List` from a `FieldValue`.
    ///
    /// Returns `None` only for empty lists (element type unknown).
    pub fn of(value: &FieldValue) -> Option<Self> {
        match value {
            FieldValue::Single(item) => Some(Self::Single(value_item_to_type_item(item))),
            FieldValue::List(items) => Some(Self::List(value_item_to_type_item(items.first()?))),
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
        assert_eq!(FieldValueItem::String("hello".into()).to_string(), "hello");
    }

    #[test]
    fn test_field_value_display_text() {
        assert_eq!(FieldValueItem::Text("bio".into()).to_string(), "bio");
    }

    #[test]
    fn test_field_value_display_integer() {
        assert_eq!(FieldValueItem::Integer(42).to_string(), "42");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_field_value_display_float() {
        assert_eq!(FieldValueItem::Float(3.14).to_string(), "3.14");
    }

    #[test]
    fn test_field_value_display_boolean() {
        assert_eq!(FieldValueItem::Boolean(true).to_string(), "true");
        assert_eq!(FieldValueItem::Boolean(false).to_string(), "false");
    }

    #[test]
    fn test_field_value_display_datetime() {
        let v = FieldValueItem::DateTime(sample_datetime());
        assert!(v.to_string().contains("2026"));
    }

    #[test]
    fn test_field_value_display_duration() {
        let d = Duration::try_minutes(90).unwrap();
        assert_eq!(FieldValueItem::Duration(d).to_string(), "90m");
    }

    #[test]
    fn test_field_value_display_list() {
        let v = FieldValue::List(vec![
            FieldValueItem::String("a".into()),
            FieldValueItem::String("b".into()),
        ]);
        assert_eq!(v.to_string(), "[a, b]");
    }

    #[test]
    fn test_into_string_ok() {
        let v = FieldValueItem::String("test".into());
        assert_eq!(v.into_string().unwrap(), "test");
    }

    #[test]
    fn test_into_string_wrong_variant() {
        let v = FieldValueItem::Integer(1);
        assert!(v.into_string().is_err());
    }

    #[test]
    fn test_into_integer_ok() {
        assert_eq!(FieldValueItem::Integer(7).into_integer().unwrap(), 7);
    }

    #[test]
    fn test_into_bool_ok() {
        assert!(FieldValueItem::Boolean(true).into_bool().unwrap());
    }

    #[test]
    fn test_is_empty() {
        assert!(FieldValue::List(vec![]).is_empty());
        assert!(!FieldValue::Single(FieldValueItem::Integer(0)).is_empty());
    }

    #[test]
    fn test_is_single() {
        assert!(FieldValue::Single(FieldValueItem::Integer(0)).is_single());
        assert!(
            !FieldValue::List(vec![FieldValueItem::Integer(0), FieldValueItem::Integer(1)])
                .is_single()
        );
    }

    #[test]
    fn test_into_list_ok() {
        let v = FieldValue::List(vec![FieldValueItem::Integer(1)]);
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
        let v = FieldValueItem::String("clone_me".into());
        assert_eq!(v.clone(), v);
    }

    // FieldTypeItem / FieldType tests

    #[test]
    fn test_field_type_item_display() {
        assert_eq!(FieldTypeItem::String.to_string(), "String");
        assert_eq!(FieldTypeItem::Text.to_string(), "Text");
        assert_eq!(FieldTypeItem::Integer.to_string(), "Integer");
        assert_eq!(FieldTypeItem::Float.to_string(), "Float");
        assert_eq!(FieldTypeItem::Boolean.to_string(), "Boolean");
        assert_eq!(FieldTypeItem::DateTime.to_string(), "DateTime");
        assert_eq!(FieldTypeItem::Duration.to_string(), "Duration");
        assert_eq!(
            FieldTypeItem::EntityIdentifier("presenter").to_string(),
            "EntityIdentifier(presenter)"
        );
    }

    #[test]
    fn test_field_type_display() {
        assert_eq!(
            FieldType::Single(FieldTypeItem::Integer).to_string(),
            "Integer"
        );
        assert_eq!(
            FieldType::Optional(FieldTypeItem::String).to_string(),
            "String?"
        );
        assert_eq!(
            FieldType::List(FieldTypeItem::Boolean).to_string(),
            "List<Boolean>"
        );
        assert_eq!(
            FieldType::List(FieldTypeItem::EntityIdentifier("panel")).to_string(),
            "List<EntityIdentifier(panel)>"
        );
    }

    #[test]
    fn test_field_type_item_type() {
        assert_eq!(
            FieldType::Single(FieldTypeItem::Float).item_type(),
            FieldTypeItem::Float
        );
        assert_eq!(
            FieldType::Optional(FieldTypeItem::Text).item_type(),
            FieldTypeItem::Text
        );
        assert_eq!(
            FieldType::List(FieldTypeItem::Integer).item_type(),
            FieldTypeItem::Integer
        );
    }

    #[test]
    fn test_field_type_predicates() {
        assert!(FieldType::Single(FieldTypeItem::Boolean).is_single());
        assert!(!FieldType::Single(FieldTypeItem::Boolean).is_optional());
        assert!(!FieldType::Single(FieldTypeItem::Boolean).is_list());

        assert!(FieldType::Optional(FieldTypeItem::Integer).is_optional());
        assert!(!FieldType::Optional(FieldTypeItem::Integer).is_single());
        assert!(!FieldType::Optional(FieldTypeItem::Integer).is_list());

        assert!(FieldType::List(FieldTypeItem::String).is_list());
        assert!(!FieldType::List(FieldTypeItem::String).is_single());
        assert!(!FieldType::List(FieldTypeItem::String).is_optional());
    }

    #[test]
    fn test_field_type_of_single() {
        assert_eq!(
            FieldType::of(&FieldValue::Single(FieldValueItem::Integer(5))),
            Some(FieldType::Single(FieldTypeItem::Integer))
        );
        assert_eq!(
            FieldType::of(&FieldValue::Single(FieldValueItem::Boolean(true))),
            Some(FieldType::Single(FieldTypeItem::Boolean))
        );
    }

    #[test]
    fn test_field_type_of_list() {
        let v = FieldValue::List(vec![
            FieldValueItem::String("a".into()),
            FieldValueItem::String("b".into()),
        ]);
        assert_eq!(
            FieldType::of(&v),
            Some(FieldType::List(FieldTypeItem::String))
        );
    }

    #[test]
    fn test_field_type_of_empty_list_is_none() {
        assert_eq!(FieldType::of(&FieldValue::List(vec![])), None);
    }

    #[test]
    fn test_field_type_of_entity_identifier() {
        use crate::entity::{EntityId, RuntimeEntityId};
        use crate::panel::PanelEntityType;
        use uuid::Uuid;
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let typed: EntityId<PanelEntityType> = EntityId::new(uuid).unwrap();
        let rid = RuntimeEntityId::from_typed(typed);
        assert_eq!(
            FieldType::of(&FieldValue::Single(FieldValueItem::EntityIdentifier(rid))),
            Some(FieldType::Single(FieldTypeItem::EntityIdentifier("panel")))
        );
    }

    #[test]
    fn test_field_type_copy() {
        let t = FieldType::Single(FieldTypeItem::Integer);
        let _copy = t;
        let _ = t; // still usable after copy
    }
}
