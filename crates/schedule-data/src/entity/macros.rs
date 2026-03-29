/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field implementation macros for entity definitions
//!
//! This module provides macros for implementing common field patterns
//! across all entity types in the schedule-data crate.

#![allow(unused_macros)]

use crate::entity::EntityType;
use crate::field::field_set::FieldSet;
use crate::field::traits::*;
use crate::field::{FieldError, FieldValue, ValidationError};
use crate::schedule::Schedule;

/// Unified macro for implementing computed fields with custom methods
/// This macro can handle any combination of read, write, and validate methods
/// and automatically implements the appropriate traits based on what's provided.
macro_rules! computed_field {
    // Read + Write + Validate (with schedule)
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            read: |$read_self:ident, $read_entity:ident, $read_schedule:ident| $read_body:expr,
            write: |$write_self:ident, $write_entity:ident, $write_schedule:ident, $write_value:ident| $write_body:expr,
            validate: |$validate_self:ident, $validate_schedule:ident, $validate_entity:ident, $validate_value:ident| $validate_body:expr
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
            fn read(&$read_self, $read_entity: &<$type as EntityType>::Data, $read_schedule: &Schedule) -> Option<FieldValue> {
                $read_body
            }
            fn is_read_computed(&self) -> bool { true }
        }

        impl WritableField<$type> for $name {
            fn write(&$write_self, $write_entity: &mut <$type as EntityType>::Data, $write_schedule: &Schedule, $write_value: FieldValue) -> Result<(), FieldError> {
                $write_body
            }
            fn is_write_computed(&self) -> bool { true }
        }

        impl CheckedField<$type> for $name {
            fn validate(&$validate_self, $validate_schedule: &Schedule, $validate_entity: &mut <$type as EntityType>::Data, $validate_value: &FieldValue) -> Result<(), ValidationError> {
                $validate_body
            }
        }
    };

    // Read + Write (with schedule)
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            read: |$read_self:ident, $read_entity:ident, $read_schedule:ident| $read_body:expr,
            write: |$write_self:ident, $write_entity:ident, $write_schedule:ident, $write_value:ident| $write_body:expr
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
            fn read(&$read_self, $read_entity: &<$type as EntityType>::Data, $read_schedule: &Schedule) -> Option<FieldValue> {
                $read_body
            }
            fn is_read_computed(&self) -> bool { true }
        }

        impl WritableField<$type> for $name {
            fn write(&$write_self, $write_entity: &mut <$type as EntityType>::Data, $write_schedule: &Schedule, $write_value: FieldValue) -> Result<(), FieldError> {
                $write_body
            }
            fn is_write_computed(&self) -> bool { true }
        }
    };

    // Read + Write + Validate (without schedule - Simple* traits)
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            read: |$read_self:ident, $read_entity:ident| $read_body:expr,
            write: |$write_self:ident, $write_entity:ident, $write_value:ident| $write_body:expr,
            validate: |$validate_self:ident, $validate_entity:ident, $validate_value:ident| $validate_body:expr
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

        impl SimpleReadableField<$type> for $name {
            fn read(&$read_self, $read_entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                $read_body
            }
            fn is_read_computed(&self) -> bool { true }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(&$write_self, $write_entity: &mut <$type as EntityType>::Data, $write_value: FieldValue) -> Result<(), FieldError> {
                $write_body
            }
            fn is_write_computed(&self) -> bool { true }
        }

        impl CheckedField<$type> for $name {
            fn validate(&$validate_self, _schedule: &Schedule, $validate_entity: &mut <$type as EntityType>::Data, $validate_value: &FieldValue) -> Result<(), ValidationError> {
                $validate_body
            }
        }
    };

    // Read + Write (without schedule - Simple* traits)
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            read: |$read_self:ident, $read_entity:ident| $read_body:expr,
            write: |$write_self:ident, $write_entity:ident, $write_value:ident| $write_body:expr
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

        impl SimpleReadableField<$type> for $name {
            fn read(&$read_self, $read_entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                $read_body
            }
            fn is_read_computed(&self) -> bool { true }
        }

        impl SimpleWritableField<$type> for $name {
            fn write(&$write_self, $write_entity: &mut <$type as EntityType>::Data, $write_value: FieldValue) -> Result<(), FieldError> {
                $write_body
            }
            fn is_write_computed(&self) -> bool { true }
        }
    };

    // Read only (with schedule)
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            read: |$read_self:ident, $read_entity:ident, $read_schedule:ident| $read_body:expr
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
            fn read(&$read_self, $read_entity: &<$type as EntityType>::Data, $read_schedule: &Schedule) -> Option<FieldValue> {
                $read_body
            }
            fn is_read_computed(&self) -> bool { true }
        }
    };

    // Write only (with schedule)
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            write: |$write_self:ident, $write_entity:ident, $write_schedule:ident, $write_value:ident| $write_body:expr
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
            fn write(&$write_self, $write_entity: &mut <$type as EntityType>::Data, $write_schedule: &Schedule, $write_value: FieldValue) -> Result<(), FieldError> {
                $write_body
            }
            fn is_write_computed(&self) -> bool { true }
        }
    };

    // Read only (without schedule - Simple* traits)
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            read: |$read_self:ident, $read_entity:ident| $read_body:expr
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

        impl SimpleReadableField<$type> for $name {
            fn read(&$read_self, $read_entity: &<$type as EntityType>::Data) -> Option<FieldValue> {
                $read_body
            }
            fn is_read_computed(&self) -> bool { true }
        }
    };

    // Write only (without schedule - Simple* traits)
    ($name:ident, $display_name:expr, $description:expr, $type:ty,
        {
            write: |$write_self:ident, $write_entity:ident, $write_value:ident| $write_body:expr
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

        impl SimpleWritableField<$type> for $name {
            fn write(&$write_self, $write_entity: &mut <$type as EntityType>::Data, $write_value: FieldValue) -> Result<(), FieldError> {
                $write_body
            }
            fn is_write_computed(&self) -> bool { true }
        }
    };
}

/// Macro for implementing direct field mappings
///
/// Supports multiple field types:
/// - String: Direct string field mapping
/// - i64: Integer field mapping  
/// - bool: Boolean field mapping
/// - Option<String>: Optional string field mapping
/// - Option<i64>: Optional integer field mapping
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

    // For Option<bool> fields (new addition)
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
}

// Re-export macros for use across entity modules
pub(crate) use computed_field;
pub(crate) use direct_field;

/// Macro to create field sets with static initialization
/// Automatically generates the name map from field names and aliases
/// Uses LazyLock for thread-safe static initialization
macro_rules! field_set {
    // Version with aliases: field_set!(Type, { fields: [FIELD1 => ["alias1", "alias2"], FIELD2 => []], required: ["field1"] })
    ($entity_type:ty, { fields: [$($field:expr => [$($alias:expr),*]),*], required: [$($required:expr),*] }) => {{
        std::sync::LazyLock::new(|| {
            // Use a static array to avoid lifetime issues
            static FIELDS: [&str; 0] = []; // Placeholder - will be replaced by actual fields

            // Create field references that live long enough
            let field_refs: Vec<&dyn NamedField<$entity_type>> = vec![$($field),*];
            let fields = field_refs.leak(); // Leak to make references 'static

            // Generate name map with primary names and aliases
            let name_map_entries: Vec<(&str, &dyn NamedField<$entity_type>)> = vec![
                $(
                    ($field.name(), $field),
                    $(
                        ($alias, $field),
                    )*
                )*
            ];
            let name_map = name_map_entries.leak(); // Leak to make references 'static

            let required: &[&str] = &[$($required),*];
            FieldSet::new(fields, name_map, required)
        })
    }};

    // Version without aliases for backwards compatibility: field_set!(Type, { fields: [FIELD1, FIELD2], required: ["field1"] })
    ($entity_type:ty, { fields: [$($field:expr),*], required: [$($required:expr),*] }) => {{
        std::sync::LazyLock::new(|| {
            // Create field references that live long enough
            let field_refs: Vec<&dyn NamedField<$entity_type>> = vec![$($field),*];
            let fields = field_refs.leak(); // Leak to make references 'static

            let name_map_entries: Vec<(&str, &dyn NamedField<$entity_type>)> = vec![$(($field.name(), $field)),*];
            let name_map = name_map_entries.leak(); // Leak to make references 'static

            let required: &[&str] = &[$($required),*];
            FieldSet::new(fields, name_map, required)
        })
    }};
}

// Re-export the collection macros as well
pub(crate) use field_set;
