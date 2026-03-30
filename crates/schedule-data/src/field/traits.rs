/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field traits and macros for type-safe field operations
//!
//! This module provides:
//! - Simple field traits for basic operations (no schedule access needed)
//! - Full field traits for computed operations (with schedule access)
//! - Macros for implementing common field patterns

#![allow(unused_macros)]

use crate::entity::EntityType;
use crate::field::{FieldError, FieldValue, ValidationError};
use crate::schedule::Schedule;

/// Match strength levels for field-based indexing and lookup
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchStrength {
    /// No match at all
    NotMatch = 0,
    /// Weak/partial match (e.g., word boundary partial match)
    WeakMatch = 1,
    /// Strong match (e.g., contains match, close spelling)
    StrongMatch = 2,
    /// Exact match
    ExactMatch = 3,
}

/// Result of a field-based lookup with match strength
#[derive(Debug, Clone)]
pub struct FieldMatchResult {
    /// The matched entity's internal ID
    pub entity_id: u64,
    /// The strength of the match
    pub strength: MatchStrength,
    /// The priority of the match (higher = more important)
    pub priority: u8,
    /// The field that produced this match
    pub field_name: &'static str,
    /// Optional match details for debugging
    pub details: Option<String>,
}

impl FieldMatchResult {
    pub fn new(entity_id: u64, strength: MatchStrength, field_name: &'static str) -> Self {
        Self {
            entity_id,
            strength,
            field_name,
            details: None,
        }
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

/// Trait for fields that can participate in indexed lookups
pub trait IndexableField<T: EntityType>: NamedField<T> {
    /// Check if this field can be used for lookups
    fn is_indexable(&self) -> bool;

    /// Perform a lookup against this field with a query value
    fn match_field(&self, query: &str, entity: &T::Data) -> Option<MatchStrength>;

    /// Get the index priority for this field (higher = more important)
    fn index_priority(&self) -> u8 {
        100
    }
}

/// Generic trait for named fields
pub trait NamedField<T: EntityType>: 'static + Send + Sync {
    /// The internal name of the field
    fn name(&self) -> &'static str;

    /// The display name of the field
    fn display_name(&self) -> &'static str;

    /// The description of the field
    fn description(&self) -> &'static str;
}

/// Field trait for static readable fields (no schedule access needed)
pub trait SimpleReadableField<T: EntityType>: NamedField<T> + 'static + Send + Sync {
    fn read(&self, entity: &T::Data) -> Option<FieldValue>;
    fn is_read_computed(&self) -> bool;
}

/// Field trait for static writable fields (no schedule access needed)
pub trait SimpleWritableField<T: EntityType>: NamedField<T> + 'static + Send + Sync {
    fn write(&self, entity: &mut T::Data, value: FieldValue) -> Result<(), FieldError>;
    fn is_write_computed(&self) -> bool;
}

/// Field trait for fields that can be both read and written (no schedule access needed)
pub trait SimpleField<T: EntityType>: SimpleReadableField<T> + SimpleWritableField<T> {}

/// Field trait for validating field values (no schedule access needed)
pub trait SimpleCheckedField<T: EntityType>: NamedField<T> + 'static + Send + Sync {
    fn validate(&self, entity: &mut T::Data, value: &FieldValue) -> Result<(), ValidationError>;
}

/// Field trait for static readable fields (with schedule access for computed fields)
pub trait ReadableField<T: EntityType>: NamedField<T> + 'static + Send + Sync {
    fn read(&self, schedule: &Schedule, entity: &T::Data) -> Option<FieldValue>;
    fn is_read_computed(&self) -> bool;
}

/// Field trait for static writable fields (with schedule access for computed fields)
pub trait WritableField<T: EntityType>: NamedField<T> + 'static + Send + Sync {
    fn write(
        &self,
        schedule: &Schedule,
        entity: &mut T::Data,
        value: FieldValue,
    ) -> Result<(), FieldError>;
    fn is_write_computed(&self) -> bool;
}

/// Field trait for fields that can be both read and written (with schedule access)
pub trait Field<T: EntityType>: ReadableField<T> + WritableField<T> {}

/// Field trait for validating field values (with schedule access)
pub trait CheckedField<T: EntityType>: NamedField<T> + 'static + Send + Sync {
    fn validate(
        &self,
        schedule: &Schedule,
        entity: &mut T::Data,
        value: &FieldValue,
    ) -> Result<(), ValidationError>;
}

/// Blanket implementation: any simple readable field can be used as a readable field
impl<T: EntityType, F: SimpleReadableField<T>> ReadableField<T> for F {
    fn read(&self, _schedule: &Schedule, entity: &T::Data) -> Option<FieldValue> {
        self.read(entity)
    }

    fn is_read_computed(&self) -> bool {
        self.is_read_computed()
    }
}

/// Blanket implementation: any simple writable field can be used as a writable field
impl<T: EntityType, F: SimpleWritableField<T>> WritableField<T> for F {
    fn write(
        &self,
        _schedule: &Schedule,
        entity: &mut T::Data,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        self.write(entity, value)
    }

    fn is_write_computed(&self) -> bool {
        self.is_write_computed()
    }
}

/// Blanket implementation: any simple checked field can be used as a checked field
impl<T: EntityType, F: SimpleCheckedField<T>> CheckedField<T> for F {
    fn validate(
        &self,
        _schedule: &Schedule,
        entity: &mut T::Data,
        value: &FieldValue,
    ) -> Result<(), ValidationError> {
        self.validate(entity, value)
    }
}

macro_rules! computed_readonly_field {
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            $($trait_method:ident($($param:ident: $param_type:ty),*) $trait_body:block)*
        }) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl ReadableField<$type> for $name {
            fn read(
                &self,
                schedule: &Schedule,
                entity: &<$type as EntityType>::Data,
            ) -> Option<FieldValue> {
                // Implementation would depend on the specific field
                unimplemented!()
            }

            fn is_read_computed(&self) -> bool {
                true
            }
        }
    };
}

macro_rules! computed_read_write_field {
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            $($trait_method:ident($($param:ident: $param_type:ty),*) $trait_body:block)*
        }) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl ReadableField<$type> for $name {
            fn read(
                &self,
                schedule: &Schedule,
                entity: &<$type as EntityType>::Data,
            ) -> Option<FieldValue> {
                // Implementation would depend on the specific field
                unimplemented!()
            }

            fn is_read_computed(&self) -> bool {
                true
            }
        }

        impl WritableField<$type> for $name {
            fn write(
                &self,
                schedule: &Schedule,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                // Implementation would depend on the specific field
                unimplemented!()
            }

            fn is_write_computed(&self) -> bool {
                true
            }
        }
    };
}

macro_rules! computed_write_only_field {
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            $($trait_method:ident($($param:ident: $param_type:ty),*) $trait_body:block)*
        }) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl WritableField<$type> for $name {
            fn write(
                &self,
                schedule: &Schedule,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                // Implementation would depend on the specific field
                unimplemented!()
            }

            fn is_write_computed(&self) -> bool {
                true
            }
        }
    };
}

macro_rules! computed_field {
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            $(fn $trait_method:ident($($param:ident: $param_type:ty),*) -> $return_type:ty $trait_body:block)*
        }) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl ReadableField<$type> for $name {
            fn read(
                &self,
                schedule: &Schedule,
                entity: &<$type as EntityType>::Data,
            ) -> Option<FieldValue> {
                // Look for a custom read implementation
                $(
                    if stringify!($trait_method) == "read" {
                        return $trait_body
                    }
                )*
                // Default implementation
                None
            }

            fn is_read_computed(&self) -> bool {
                true
            }
        }

        impl WritableField<$type> for $name {
            fn write(
                &self,
                schedule: &Schedule,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                // Look for a custom write implementation
                $(
                    if stringify!($trait_method) == "write" {
                        return $trait_body
                    }
                )*
                // Default implementation
                Err(FieldError::CannotStoreComputedField)
            }

            fn is_write_computed(&self) -> bool {
                true
            }
        }

        impl CheckedField<$type> for $name {
            fn validate(
                &self,
                schedule: &Schedule,
                entity: &mut <$type as EntityType>::Data,
                value: &FieldValue,
            ) -> Result<(), ValidationError> {
                // Look for a custom validate implementation
                $(
                    if stringify!($trait_method) == "validate" {
                        return $trait_body
                    }
                )*
                // Default implementation - no validation
                Ok(())
            }
        }
    };
}

macro_rules! direct_field {
    // For String fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, String) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                Some(FieldValue::String(entity.$field.clone()))
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::String(v) = value {
                    entity.$field = v;
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For Option<String> fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, Option<String>) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                entity.$field.clone().map(FieldValue::String)
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::String(v) = value {
                    entity.$field = Some(v);
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For i64 fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, i64) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                Some(FieldValue::Integer(entity.$field))
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::Integer(v) = value {
                    entity.$field = v;
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For Option<i64> fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, Option<i64>) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                entity.$field.map(FieldValue::Integer)
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::Integer(v) = value {
                    entity.$field = Some(v);
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For f64 fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, f64) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                Some(FieldValue::Float(entity.$field))
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::Float(v) = value {
                    entity.$field = v;
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For Option<f64> fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, Option<f64>) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                entity.$field.map(FieldValue::Float)
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::Float(v) = value {
                    entity.$field = Some(v);
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For bool fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, bool) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                Some(FieldValue::Boolean(entity.$field))
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::Boolean(v) = value {
                    entity.$field = v;
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For Option<bool> fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, Option<bool>) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                entity.$field.map(FieldValue::Boolean)
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::Boolean(v) = value {
                    entity.$field = Some(v);
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For chrono::NaiveDateTime fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, chrono::NaiveDateTime) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                Some(FieldValue::DateTime(entity.$field))
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::DateTime(v) = value {
                    entity.$field = v;
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For Option<chrono::NaiveDateTime> fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, Option<chrono::NaiveDateTime>) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                entity.$field.map(FieldValue::DateTime)
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::DateTime(v) = value {
                    entity.$field = Some(v);
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For chrono::Duration fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, chrono::Duration) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                Some(FieldValue::Duration(entity.$field))
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::Duration(v) = value {
                    entity.$field = v;
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For Option<chrono::Duration> fields
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, Option<chrono::Duration>) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                entity.$field.map(FieldValue::Duration)
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::Duration(v) = value {
                    entity.$field = Some(v);
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For Vec<String> fields (List)
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, Vec<String>) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                Some(FieldValue::List(
                    entity
                        .$field
                        .iter()
                        .map(|s| FieldValue::String(s.clone()))
                        .collect(),
                ))
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::List(values) = value {
                    entity.$field = values
                        .into_iter()
                        .filter_map(|fv| {
                            if let FieldValue::String(s) = fv {
                                Some(s)
                            } else {
                                None
                            }
                        })
                        .collect();
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };

    // For Id fields (String with special handling)
    ($name:ident, $display_name:expr, $description:expr, $type:ty, $field:ident, Id) => {
        pub struct $name;

        impl NamedField<$type> for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn display_name(&self) -> &'static str {
                $display_name
            }

            fn description(&self) -> &'static str {
                $description
            }
        }

        impl SimpleReadableField<$type> for $name {
            fn read(&self, entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                Some(FieldValue::Id(entity.$field.clone()))
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(
                &self,
                entity: &mut <$type as EntityType>::Data,
                value: FieldValue,
            ) -> Result<(), FieldError> {
                if let FieldValue::Id(v) = value {
                    entity.$field = v;
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }

            fn is_write_computed(&self) -> bool {
                false
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::field_set::FieldSet;
    use std::sync::LazyLock;

    // Mock entity for testing
    #[derive(Debug, Clone, PartialEq)]
    struct TestEntity {
        id: String,
        name: String,
        value: i64,
        optional_value: Option<i64>,
        float_value: f64,
        optional_float: Option<f64>,
        flag: bool,
        optional_flag: Option<bool>,
        optional_string: Option<String>,
        timestamp: chrono::NaiveDateTime,
        optional_timestamp: Option<chrono::NaiveDateTime>,
        duration: chrono::Duration,
        optional_duration: Option<chrono::Duration>,
        tags: Vec<String>,
        entity_id: String, // For Id field type
    }

    // Mock EntityType implementation
    impl EntityType for TestEntity {
        type Data = TestEntity;
        const TYPE_NAME: &'static str = "TestEntity";

        fn validate(_data: &Self::Data) -> Result<(), crate::field::validation::ValidationError> {
            Ok(())
        }

        fn field_set() -> &'static FieldSet<Self> {
            // Return a minimal field set for testing
            static TEST_FIELD_SET: LazyLock<FieldSet<TestEntity>> = LazyLock::new(|| {
                static FIELD_MAP: LazyLock<Vec<(&str, &dyn NamedField<TestEntity>)>> =
                    LazyLock::new(|| {
                        vec![
                            ("id", &TestIdField as &dyn NamedField<TestEntity>),
                            ("value", &TestValueField as &dyn NamedField<TestEntity>),
                        ]
                    });
                FieldSet::new(
                    &[&TestIdField, &TestValueField],
                    &FIELD_MAP,
                    &["id"], // only id is required
                    &[],     // no indexable fields for this test
                )
            });
            &TEST_FIELD_SET
        }
    }

    // Create a mock schedule
    fn create_mock_schedule() -> Schedule {
        Schedule::default()
    }

    fn create_test_entity() -> TestEntity {
        TestEntity {
            id: "123".to_string(),
            name: "Test".to_string(),
            value: 42,
            optional_value: Some(100),
            float_value: 3.14,
            optional_float: Some(2.71),
            flag: true,
            optional_flag: Some(false),
            optional_string: Some("optional".to_string()),
            timestamp: chrono::NaiveDateTime::default(),
            optional_timestamp: Some(chrono::NaiveDateTime::default()),
            duration: chrono::Duration::zero(),
            optional_duration: Some(chrono::Duration::minutes(30)),
            tags: vec!["test".to_string(), "sample".to_string()],
            entity_id: "entity-123".to_string(),
        }
    }

    // Test field using direct_field macro
    direct_field!(
        TestIdField,
        "Test ID",
        "Test ID field",
        TestEntity,
        id,
        String
    );

    // Test field with i64
    direct_field!(
        TestValueField,
        "Value",
        "Value field",
        TestEntity,
        value,
        i64
    );

    // Test field with Option<i64>
    direct_field!(
        TestOptionalValueField,
        "Optional Value",
        "Optional value field",
        TestEntity,
        optional_value,
        Option<i64>
    );

    // Test field with bool
    direct_field!(TestFlagField, "Flag", "Flag field", TestEntity, flag, bool);

    // Test field with Option<bool>
    direct_field!(
        TestOptionalFlagField,
        "Optional Flag",
        "Optional flag field",
        TestEntity,
        optional_flag,
        Option<bool>
    );

    // Test field with Option<String> for testing optional field behavior
    direct_field!(
        TestOptionalField,
        "Optional",
        "Optional field",
        TestEntity,
        optional_string,
        Option<String>
    );

    #[test]
    fn test_named_field_trait() {
        let field = TestIdField;

        assert_eq!(field.name(), "TestIdField");
        assert_eq!(field.display_name(), "Test ID");
        assert_eq!(field.description(), "Test ID field");
    }

    #[test]
    fn test_readable_field_trait() {
        let field = TestIdField;
        let entity = create_test_entity();
        let schedule = create_mock_schedule();

        let value = ReadableField::read(&field, &schedule, &entity);
        assert!(value.is_some());

        match value.unwrap() {
            FieldValue::String(s) => assert_eq!(s, "123"),
            _ => panic!("Expected String value"),
        }

        assert!(!ReadableField::is_read_computed(&field));
    }

    #[test]
    fn test_writable_field_trait() {
        let field = TestIdField;
        let mut entity = create_test_entity();
        let schedule = create_mock_schedule();

        let result = WritableField::write(
            &field,
            &schedule,
            &mut entity,
            FieldValue::String("456".to_string()),
        );
        assert!(result.is_ok());
        assert_eq!(entity.id, "456");

        assert!(!WritableField::is_write_computed(&field));
    }

    #[test]
    fn test_writable_field_wrong_type() {
        let field = TestIdField;
        let mut entity = create_test_entity();
        let schedule = create_mock_schedule();

        let result = WritableField::write(&field, &schedule, &mut entity, FieldValue::Integer(999));
        assert!(result.is_err());
        assert_eq!(entity.id, "123"); // Should remain unchanged
    }

    // Test field with String
    direct_field!(
        TestNameField,
        "Name",
        "Name field",
        TestEntity,
        name,
        String
    );

    #[test]
    fn test_name_field_read() {
        let field = TestNameField;
        let entity = create_test_entity();
        let schedule = create_mock_schedule();

        let value = ReadableField::read(&field, &schedule, &entity);
        assert!(value.is_some());

        match value.unwrap() {
            FieldValue::String(s) => assert_eq!(s, "Test"),
            _ => panic!("Expected String value"),
        }
    }

    #[test]
    fn test_name_field_read_none() {
        let field = TestNameField;
        let mut entity = create_test_entity();
        entity.name = "".to_string(); // Empty string for this test
        let schedule = create_mock_schedule();

        let value = ReadableField::read(&field, &schedule, &entity);
        assert!(value.is_none());
    }

    #[test]
    fn test_optional_field_write() {
        let field = TestOptionalValueField;
        let mut entity = create_test_entity();
        entity.optional_value = None; // Start with None for this test
        let schedule = create_mock_schedule();

        let result = WritableField::write(&field, &schedule, &mut entity, FieldValue::Integer(999));
        assert!(result.is_ok());
        assert_eq!(entity.optional_value, Some(999));
    }

    // Test field with i64
    direct_field!(
        TestIntField,
        "Test Int",
        "Test integer field",
        TestEntity,
        value,
        i64
    );

    #[test]
    fn test_integer_field_read() {
        let field = TestValueField;
        let entity = create_test_entity();
        let schedule = create_mock_schedule();

        let value = ReadableField::read(&field, &schedule, &entity);
        assert!(value.is_some());

        match value.unwrap() {
            FieldValue::Integer(i) => assert_eq!(i, 42),
            _ => panic!("Expected Integer value"),
        }
    }

    #[test]
    fn test_integer_field_write() {
        let field = TestValueField;
        let mut entity = create_test_entity();
        let schedule = create_mock_schedule();

        let result = WritableField::write(&field, &schedule, &mut entity, FieldValue::Integer(999));
        assert!(result.is_ok());
        assert_eq!(entity.value, 999);
    }

    #[test]
    fn test_bool_field_write() {
        let field = TestFlagField;
        let mut entity = create_test_entity();
        let schedule = create_mock_schedule();

        let result =
            WritableField::write(&field, &schedule, &mut entity, FieldValue::Boolean(false));
        assert!(result.is_ok());
        assert_eq!(entity.flag, false);
    }

    // Test field with bool
    direct_field!(
        TestBoolField,
        "Test Bool",
        "Test boolean field",
        TestEntity,
        flag,
        bool
    );

    #[test]
    fn test_boolean_field_read() {
        let field = TestFlagField;
        let entity = create_test_entity();
        let schedule = create_mock_schedule();

        let value = ReadableField::read(&field, &schedule, &entity);
        assert!(value.is_some());

        match value.unwrap() {
            FieldValue::Boolean(b) => assert_eq!(b, true),
            _ => panic!("Expected Boolean value"),
        }
    }

    #[test]
    fn test_boolean_field_write() {
        let field = TestFlagField;
        let mut entity = create_test_entity();
        let schedule = create_mock_schedule();

        let result =
            WritableField::write(&field, &schedule, &mut entity, FieldValue::Boolean(false));
        assert!(result.is_ok());
        assert_eq!(entity.flag, false);
    }

    // Computed field for testing
    pub struct ComputedTestField;

    impl NamedField<TestEntity> for ComputedTestField {
        fn name(&self) -> &'static str {
            "ComputedTestField"
        }

        fn display_name(&self) -> &'static str {
            "Computed Test Field"
        }

        fn description(&self) -> &'static str {
            "A computed test field"
        }
    }

    impl ReadableField<TestEntity> for ComputedTestField {
        fn read(&self, _schedule: &Schedule, entity: &TestEntity) -> Option<FieldValue> {
            Some(FieldValue::Integer(entity.value as i64 * 2))
        }

        fn is_read_computed(&self) -> bool {
            true
        }
    }

    #[test]
    fn test_computed_field() {
        let field = ComputedTestField;
        let entity = create_test_entity();
        let schedule = create_mock_schedule();

        // Test NamedField trait
        assert_eq!(field.name(), "ComputedTestField");
        assert_eq!(field.display_name(), "Computed Test Field");
        assert_eq!(field.description(), "A computed test field");

        // Test ReadableField trait
        let value = ReadableField::read(&field, &schedule, &entity);
        assert!(value.is_some());

        match value.unwrap() {
            FieldValue::Integer(i) => assert_eq!(i, 42), // 21 * 2
            _ => panic!("Expected Integer value"),
        }

        assert!(field.is_read_computed());
    }

    // Test that Field trait is automatically implemented
    struct FullTestField;

    impl NamedField<TestEntity> for FullTestField {
        fn name(&self) -> &'static str {
            "FullTestField"
        }

        fn display_name(&self) -> &'static str {
            "Full Test Field"
        }

        fn description(&self) -> &'static str {
            "A full test field"
        }
    }

    impl ReadableField<TestEntity> for FullTestField {
        fn read(&self, _schedule: &Schedule, _entity: &TestEntity) -> Option<FieldValue> {
            Some(FieldValue::String("read".to_string()))
        }

        fn is_read_computed(&self) -> bool {
            false
        }
    }

    impl WritableField<TestEntity> for FullTestField {
        fn write(
            &self,
            _schedule: &Schedule,
            _entity: &mut TestEntity,
            value: FieldValue,
        ) -> Result<(), FieldError> {
            match value {
                FieldValue::String(_) => Ok(()),
                _ => Err(FieldError::CannotStoreComputedField),
            }
        }

        fn is_write_computed(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_field_trait_combination() {
        let field = FullTestField;
        let entity = create_test_entity();
        let schedule = create_mock_schedule();

        // Should work as ReadableField
        let value = ReadableField::read(&field, &schedule, &entity);
        assert!(value.is_some());

        // Should work as WritableField
        let result = WritableField::write(
            &field,
            &schedule,
            &mut entity.clone(),
            FieldValue::String("test".to_string()),
        );
        assert!(result.is_ok());

        let result = WritableField::write(
            &field,
            &schedule,
            &mut entity.clone(),
            FieldValue::Integer(999),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_macro_generated_field_names() {
        let field = TestIdField;
        assert_eq!(field.name(), "TestIdField");
        assert_eq!(field.display_name(), "Test ID");
        assert_eq!(field.description(), "Test ID field");

        let field = TestOptionalField;
        assert_eq!(field.name(), "TestOptionalField");
        assert_eq!(field.display_name(), "Optional");
        assert_eq!(field.description(), "Optional field");

        let field = TestIntField;
        assert_eq!(field.name(), "TestIntField");
        assert_eq!(field.display_name(), "Test Int");
        assert_eq!(field.description(), "Test integer field");

        let field = TestBoolField;
        assert_eq!(field.name(), "TestBoolField");
        assert_eq!(field.display_name(), "Test Bool");
        assert_eq!(field.description(), "Test boolean field");
    }
}
