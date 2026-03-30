/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field type implementations with validation and conversion

use std::fmt;

use super::{ConversionError, FieldMatcher, FieldValue, ValidationError};

/// Core trait for all field types
pub trait FieldType: 'static + Send + Sync {
    type Value: Clone + PartialEq + Send + Sync + fmt::Debug;
    type Storage: Clone + Send + Sync + fmt::Debug;

    const NAME: &'static str;

    /// Convert runtime value to storage format
    fn to_storage(value: Self::Value) -> Self::Storage;

    /// Convert storage format to runtime value
    fn from_storage(storage: Self::Storage) -> Self::Value;

    /// Validate value against field constraints
    fn validate(value: &Self::Value) -> Result<(), ValidationError>;

    /// Check if value matches a query condition
    fn matches(value: &Self::Value, matcher: &FieldMatcher) -> bool;

    /// Support multiple input formats (e.g., string -> DateTime)
    fn try_convert(value: &FieldValue) -> Result<Self::Value, ConversionError>;
}

/// Duration field type
#[derive(Debug, Clone, Copy)]
pub struct DurationFieldType;

impl FieldType for DurationFieldType {
    type Value = chrono::Duration;
    type Storage = chrono::Duration;

    const NAME: &'static str = "duration";

    fn to_storage(value: Self::Value) -> Self::Storage {
        value
    }

    fn from_storage(storage: Self::Storage) -> Self::Value {
        storage
    }

    fn validate(_value: &Self::Value) -> Result<(), ValidationError> {
        Ok(())
    }

    fn matches(value: &Self::Value, matcher: &FieldMatcher) -> bool {
        let mins = value.num_minutes();
        match matcher {
            FieldMatcher::Equals(other) => {
                if let Ok(other_dur) = Self::try_convert(other) {
                    mins == other_dur.num_minutes()
                } else {
                    false
                }
            }
            FieldMatcher::NotEquals(other) => {
                if let Ok(other_dur) = Self::try_convert(other) {
                    mins != other_dur.num_minutes()
                } else {
                    true
                }
            }
            FieldMatcher::Range(start, end) => {
                if let (Ok(start_dur), Ok(end_dur)) =
                    (Self::try_convert(start), Self::try_convert(end))
                {
                    let start_mins = start_dur.num_minutes();
                    let end_mins = end_dur.num_minutes();
                    start_mins <= mins && mins <= end_mins
                } else {
                    false
                }
            }
            FieldMatcher::In(values) => values.iter().any(|v| {
                if let Ok(other_dur) = Self::try_convert(v) {
                    mins == other_dur.num_minutes()
                } else {
                    false
                }
            }),
            FieldMatcher::NotIn(values) => !values.iter().any(|v| {
                if let Ok(other_dur) = Self::try_convert(v) {
                    mins == other_dur.num_minutes()
                } else {
                    false
                }
            }),
            FieldMatcher::IsNull => false,
            FieldMatcher::IsNotNull => true,
            FieldMatcher::Contains(_) | FieldMatcher::StartsWith(_) | FieldMatcher::EndsWith(_) => {
                false
            }
        }
    }

    fn try_convert(value: &FieldValue) -> Result<Self::Value, ConversionError> {
        match value {
            FieldValue::Duration(d) => Ok(*d),
            FieldValue::Integer(i) => Ok(chrono::Duration::minutes(*i)),
            FieldValue::String(s) => {
                let mins: i64 = s.parse().map_err(|_| ConversionError::InvalidFormat)?;
                Ok(chrono::Duration::minutes(mins))
            }
            _ => Err(ConversionError::UnsupportedType),
        }
    }
}

/// String field type
#[derive(Debug, Clone, Copy)]
pub struct StringFieldType;

impl FieldType for StringFieldType {
    type Value = String;
    type Storage = String;

    const NAME: &'static str = "string";

    fn to_storage(value: Self::Value) -> Self::Storage {
        value
    }

    fn from_storage(storage: Self::Storage) -> Self::Value {
        storage
    }

    fn validate(value: &Self::Value) -> Result<(), ValidationError> {
        if value.len() > 1000 {
            return Err(ValidationError::InvalidValue {
                field: "string".to_string(),
                value: value.clone(),
                reason: "String too long (max 1000 characters)".to_string(),
            });
        }
        Ok(())
    }

    fn matches(value: &Self::Value, matcher: &FieldMatcher) -> bool {
        match matcher {
            FieldMatcher::Equals(other) => {
                if let Ok(other_str) = Self::try_convert(other) {
                    value == &other_str
                } else {
                    false
                }
            }
            FieldMatcher::NotEquals(other) => {
                if let Ok(other_str) = Self::try_convert(other) {
                    value != &other_str
                } else {
                    true
                }
            }
            FieldMatcher::Contains(pattern) => value.contains(pattern),
            FieldMatcher::StartsWith(prefix) => value.starts_with(prefix),
            FieldMatcher::EndsWith(suffix) => value.ends_with(suffix),
            FieldMatcher::In(values) => values.iter().any(|v| {
                if let Ok(other_str) = Self::try_convert(v) {
                    value == &other_str
                } else {
                    false
                }
            }),
            FieldMatcher::NotIn(values) => !values.iter().any(|v| {
                if let Ok(other_str) = Self::try_convert(v) {
                    value == &other_str
                } else {
                    false
                }
            }),
            FieldMatcher::IsNull => false,
            FieldMatcher::IsNotNull => true,
            FieldMatcher::Range(_, _) => false, // Strings don't support range matching
        }
    }

    fn try_convert(value: &FieldValue) -> Result<Self::Value, ConversionError> {
        match value {
            FieldValue::String(s) => Ok(s.clone()),
            FieldValue::Integer(i) => Ok(i.to_string()),
            FieldValue::Float(f) => Ok(f.to_string()),
            FieldValue::Boolean(b) => Ok(b.to_string()),
            FieldValue::Duration(d) => Ok(d.num_minutes().to_string()),
            FieldValue::EntityId(id) => Ok(id.to_string()),
            _ => Err(ConversionError::UnsupportedType),
        }
    }
}

/// Integer field type
#[derive(Debug, Clone, Copy)]
pub struct IntegerFieldType;

impl FieldType for IntegerFieldType {
    type Value = i64;
    type Storage = i64;

    const NAME: &'static str = "integer";

    fn to_storage(value: Self::Value) -> Self::Storage {
        value
    }

    fn from_storage(storage: Self::Storage) -> Self::Value {
        storage
    }

    fn validate(_value: &Self::Value) -> Result<(), ValidationError> {
        // i64 values are inherently within range; no validation needed
        Ok(())
    }

    fn matches(value: &Self::Value, matcher: &FieldMatcher) -> bool {
        match matcher {
            FieldMatcher::Equals(other) => {
                if let Ok(other_int) = Self::try_convert(other) {
                    value == &other_int
                } else {
                    false
                }
            }
            FieldMatcher::NotEquals(other) => {
                if let Ok(other_int) = Self::try_convert(other) {
                    value != &other_int
                } else {
                    true
                }
            }
            FieldMatcher::Range(start, end) => {
                if let (Ok(start_int), Ok(end_int)) =
                    (Self::try_convert(start), Self::try_convert(end))
                {
                    start_int <= *value && *value <= end_int
                } else {
                    false
                }
            }
            FieldMatcher::In(values) => values.iter().any(|v| {
                if let Ok(other_int) = Self::try_convert(v) {
                    value == &other_int
                } else {
                    false
                }
            }),
            FieldMatcher::NotIn(values) => !values.iter().any(|v| {
                if let Ok(other_int) = Self::try_convert(v) {
                    value == &other_int
                } else {
                    false
                }
            }),
            FieldMatcher::IsNull => false,
            FieldMatcher::IsNotNull => true,
            FieldMatcher::Contains(_) | FieldMatcher::StartsWith(_) | FieldMatcher::EndsWith(_) => {
                false
            }
        }
    }

    fn try_convert(value: &FieldValue) -> Result<Self::Value, ConversionError> {
        match value {
            FieldValue::Integer(i) => Ok(*i),
            FieldValue::Duration(d) => Ok(d.num_minutes()),
            FieldValue::String(s) => s.parse().map_err(|_| ConversionError::InvalidFormat),
            FieldValue::Float(f) => {
                if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 {
                    Ok(*f as i64)
                } else {
                    Err(ConversionError::InvalidFormat)
                }
            }
            _ => Err(ConversionError::UnsupportedType),
        }
    }
}

/// Boolean field type
#[derive(Debug, Clone, Copy)]
pub struct BooleanFieldType;

impl FieldType for BooleanFieldType {
    type Value = bool;
    type Storage = bool;

    const NAME: &'static str = "boolean";

    fn to_storage(value: Self::Value) -> Self::Storage {
        value
    }

    fn from_storage(storage: Self::Storage) -> Self::Value {
        storage
    }

    fn validate(_value: &Self::Value) -> Result<(), ValidationError> {
        Ok(())
    }

    fn matches(value: &Self::Value, matcher: &FieldMatcher) -> bool {
        match matcher {
            FieldMatcher::Equals(other) => {
                if let Ok(other_bool) = Self::try_convert(other) {
                    value == &other_bool
                } else {
                    false
                }
            }
            FieldMatcher::NotEquals(other) => {
                if let Ok(other_bool) = Self::try_convert(other) {
                    value != &other_bool
                } else {
                    true
                }
            }
            FieldMatcher::IsNull => false,
            FieldMatcher::IsNotNull => true,
            _ => false, // Other matchers don't apply to booleans
        }
    }

    fn try_convert(value: &FieldValue) -> Result<Self::Value, ConversionError> {
        match value {
            FieldValue::Boolean(b) => Ok(*b),
            FieldValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Ok(true),
                "false" | "0" | "no" | "off" => Ok(false),
                _ => Err(ConversionError::InvalidFormat),
            },
            FieldValue::Integer(i) => Ok(*i != 0),
            _ => Err(ConversionError::UnsupportedType),
        }
    }
}

/// DateTime field type
#[derive(Debug, Clone, Copy)]
pub struct DateTimeFieldType;

impl FieldType for DateTimeFieldType {
    type Value = chrono::NaiveDateTime;
    type Storage = chrono::NaiveDateTime;

    const NAME: &'static str = "datetime";

    fn to_storage(value: Self::Value) -> Self::Storage {
        value
    }

    fn from_storage(storage: Self::Storage) -> Self::Value {
        storage
    }

    fn validate(value: &Self::Value) -> Result<(), ValidationError> {
        // Could validate business rules like "must be after 2020"
        if *value < chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc() {
            return Err(ValidationError::InvalidValue {
                field: "datetime".to_string(),
                value: value.to_string(),
                reason: "DateTime must be after Unix epoch".to_string(),
            });
        }
        Ok(())
    }

    fn matches(value: &Self::Value, matcher: &FieldMatcher) -> bool {
        match matcher {
            FieldMatcher::Equals(other) => {
                if let Ok(other_dt) = Self::try_convert(other) {
                    value == &other_dt
                } else {
                    false
                }
            }
            FieldMatcher::NotEquals(other) => {
                if let Ok(other_dt) = Self::try_convert(other) {
                    value != &other_dt
                } else {
                    true
                }
            }
            FieldMatcher::Range(start, end) => {
                if let (Ok(start_dt), Ok(end_dt)) =
                    (Self::try_convert(start), Self::try_convert(end))
                {
                    start_dt <= *value && *value <= end_dt
                } else {
                    false
                }
            }
            FieldMatcher::In(values) => values.iter().any(|v| {
                if let Ok(other_dt) = Self::try_convert(v) {
                    value == &other_dt
                } else {
                    false
                }
            }),
            FieldMatcher::NotIn(values) => !values.iter().any(|v| {
                if let Ok(other_dt) = Self::try_convert(v) {
                    value == &other_dt
                } else {
                    false
                }
            }),
            FieldMatcher::IsNull => false,
            FieldMatcher::IsNotNull => true,
            FieldMatcher::Contains(_) | FieldMatcher::StartsWith(_) | FieldMatcher::EndsWith(_) => {
                false
            }
        }
    }

    fn try_convert(value: &FieldValue) -> Result<Self::Value, ConversionError> {
        match value {
            FieldValue::DateTime(dt) => Ok(*dt),
            FieldValue::String(s) => {
                // Try multiple common formats
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
                    .map_err(|_| ConversionError::InvalidFormat)
            }
            FieldValue::Integer(ts) => chrono::DateTime::from_timestamp(*ts, 0)
                .map(|dt| dt.naive_utc())
                .ok_or(ConversionError::InvalidTimestamp),
            _ => Err(ConversionError::UnsupportedType),
        }
    }
}

/// List field type for arrays of field values
#[derive(Debug, Clone)]
pub struct ListFieldType;

impl FieldType for ListFieldType {
    type Value = Vec<FieldValue>;
    type Storage = Vec<FieldValue>;

    const NAME: &'static str = "list";

    fn to_storage(value: Self::Value) -> Self::Storage {
        value
    }

    fn from_storage(storage: Self::Storage) -> Self::Value {
        storage
    }

    fn validate(value: &Self::Value) -> Result<(), ValidationError> {
        if value.len() > 1000 {
            return Err(ValidationError::InvalidValue {
                field: "list".to_string(),
                value: format!("{} items", value.len()),
                reason: "List too long (max 1000 items)".to_string(),
            });
        }
        Ok(())
    }

    fn matches(value: &Self::Value, matcher: &FieldMatcher) -> bool {
        match matcher {
            FieldMatcher::Contains(pattern) => {
                value.iter().any(|item| item.to_string().contains(pattern))
            }
            FieldMatcher::In(values) => values.iter().any(|v| value.contains(v)),
            FieldMatcher::IsNull => value.is_empty(),
            FieldMatcher::IsNotNull => !value.is_empty(),
            _ => false, // Other matchers not applicable to lists
        }
    }

    fn try_convert(value: &FieldValue) -> Result<Self::Value, ConversionError> {
        match value {
            FieldValue::List(list) => Ok(list.clone()),
            FieldValue::String(s) => Ok(vec![FieldValue::String(s.clone())]),
            _ => Ok(vec![value.clone()]),
        }
    }
}

/// ID field type for entity references
#[derive(Debug, Clone, Copy)]
pub struct IdFieldType;

impl FieldType for IdFieldType {
    type Value = String;
    type Storage = String;

    const NAME: &'static str = "id";

    fn to_storage(value: Self::Value) -> Self::Storage {
        value
    }

    fn from_storage(storage: Self::Storage) -> Self::Value {
        storage
    }

    fn validate(value: &Self::Value) -> Result<(), ValidationError> {
        if value.is_empty() {
            return Err(ValidationError::InvalidValue {
                field: "id".to_string(),
                value: value.clone(),
                reason: "ID cannot be empty".to_string(),
            });
        }
        if value.len() > 100 {
            return Err(ValidationError::InvalidValue {
                field: "id".to_string(),
                value: value.clone(),
                reason: "ID too long (max 100 characters)".to_string(),
            });
        }
        Ok(())
    }

    fn matches(value: &Self::Value, matcher: &FieldMatcher) -> bool {
        // Use string matching logic for IDs
        StringFieldType::matches(value, matcher)
    }

    fn try_convert(value: &FieldValue) -> Result<Self::Value, ConversionError> {
        match value {
            FieldValue::EntityId(id) => Ok(id.to_string()),
            FieldValue::String(s) => Ok(s.clone()),
            _ => Err(ConversionError::UnsupportedType),
        }
    }
}
