/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field traits for type-safe entity field operations.
//!
//! # Trait hierarchy
//!
//! All field traits require [`NamedField`] which provides the canonical name,
//! display name, and description.  `NamedField` is **not** generic over `T`.
//!
//! Two parallel trait families exist — *Simple* (no schedule access) and *Full*
//! (receives `&Schedule`).  Blanket impls auto-promote Simple → Full by
//! discarding the unused schedule reference.
//!
//! ```text
//! NamedField                    name(), display_name(), description()
//! ├── SimpleReadableField<T>    read(&entity) → Option<FieldValue>
//! │   └── (blanket) ReadableField<T>
//! ├── SimpleWritableField<T>    write(&mut entity, FieldValue) → Result
//! │   └── (blanket) WritableField<T>
//! ├── SimpleCheckedField<T>     validate(&mut entity, &FieldValue) → Result
//! │   └── (blanket) CheckedField<T>
//! ├── IndexableField<T>         match_field(query, &entity) → Option<MatchStrength>
//! ├── ReadableField<T>          read(&Schedule, &entity) → Option<FieldValue>
//! ├── WritableField<T>          write(&Schedule, &mut entity, FieldValue) → Result
//! └── CheckedField<T>           validate(&Schedule, &mut entity, &FieldValue) → Result
//! ```
//!
//! Combo traits: [`SimpleField`] = `SimpleReadableField + SimpleWritableField`,
//! [`Field`] = `ReadableField + WritableField`.
//!
//! # Generated code
//!
//! The `#[derive(EntityFields)]` proc-macro in `schedule-macro` generates
//! `SimpleReadableField` and `SimpleWritableField` impls for direct fields,
//! and `ReadableField` / `WritableField` for computed fields that reference
//! the schedule.  See `.windsurf/rules/field-system.md` for the full
//! attribute reference.

#![allow(unused_macros)]

use crate::entity::EntityType;
#[allow(unused_imports)]
use crate::field::validation::ConversionError;
use crate::field::{FieldError, FieldValue, ValidationError};
use crate::schedule::Schedule;
use std::fmt::Debug;

/// Match priority for field-based indexing and lookup
/// Higher values = better matches. 0 = no match.
/// Common levels: ExactMatch = 255, StrongMatch = 200, AverageMatch = 100, WeakMatch = 50, NoMatch = 0
pub type MatchPriority = u8;

/// Common match priority levels
pub mod match_priority {
    use super::MatchPriority;

    /// No match at all
    pub const NO_MATCH: MatchPriority = 0;
    /// Minimum match level (anything >= 1 is considered a match)
    pub const MIN_MATCH: MatchPriority = 1;
    /// Weak/partial match (e.g., substring within word)
    pub const WEAK_MATCH: MatchPriority = 50;
    /// Average match (e.g., word boundary)
    pub const AVERAGE_MATCH: MatchPriority = 100;
    /// Strong match (e.g., matches at beginning of string)
    pub const STRONG_MATCH: MatchPriority = 200;
    /// Exact match
    pub const EXACT_MATCH: MatchPriority = 255;
}

/// Result of a field-based lookup with match priority
#[derive(Debug, Clone)]
pub struct FieldMatchResult {
    /// The matched entity's UUID
    pub entity_uuid: uuid::NonNilUuid,
    /// The priority of the match (higher = better match, 0 = no match)
    pub priority: MatchPriority,
    /// The field priority (from indexable attribute)
    pub field_priority: u8,
    /// The field that produced this match
    pub field_name: &'static str,
    /// Optional match details for debugging
    pub details: Option<String>,
}

impl FieldMatchResult {
    pub fn new(
        entity_uuid: uuid::NonNilUuid,
        priority: MatchPriority,
        field_priority: u8,
        field_name: &'static str,
    ) -> Self {
        Self {
            entity_uuid,
            priority,
            field_priority,
            field_name,
            details: None,
        }
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

/// Generic trait for named fields
pub trait NamedField: 'static + Send + Sync + Debug {
    /// The internal name of the field
    fn name(&self) -> &'static str;

    /// The display name of the field
    fn display_name(&self) -> &'static str;

    /// The description of the field
    fn description(&self) -> &'static str;
}

pub type FieldReference = &'static dyn NamedField;

/// Trait for fields that can participate in indexed lookups
pub trait IndexableField<T: EntityType>: NamedField {
    /// Check if this field can be used for lookups
    fn is_indexable(&self) -> bool;

    /// Perform a lookup against this field with a query value
    fn match_field(&self, query: &str, entity: &T::Data) -> Option<MatchPriority>;

    /// Get the index priority for this field (higher = more important)
    fn index_priority(&self) -> u8 {
        100
    }
}

/// Field trait for static readable fields (no schedule access needed)
pub trait SimpleReadableField<T: EntityType>: NamedField + 'static + Send + Sync {
    fn read(&self, entity: &T::Data) -> Option<FieldValue>;
    fn is_read_computed(&self) -> bool;
}

/// Field trait for static writable fields (no schedule access needed)
pub trait SimpleWritableField<T: EntityType>: NamedField + 'static + Send + Sync {
    fn write(&self, entity: &mut T::Data, value: FieldValue) -> Result<(), FieldError>;
    fn is_write_computed(&self) -> bool;
}

/// Field trait for fields that can be both read and written (no schedule access needed)
pub trait SimpleField<T: EntityType>: SimpleReadableField<T> + SimpleWritableField<T> {}

/// Field trait for validating field values (no schedule access needed)
pub trait SimpleCheckedField<T: EntityType>: NamedField + 'static + Send + Sync {
    fn validate(&self, entity: &mut T::Data, value: &FieldValue) -> Result<(), ValidationError>;
}

/// Field trait for static readable fields (with schedule access for computed fields)
pub trait ReadableField<T: EntityType>: NamedField + 'static + Send + Sync {
    fn read(&self, schedule: &Schedule, entity: &T::Data) -> Option<FieldValue>;
    fn is_read_computed(&self) -> bool;
}

/// Field trait for static writable fields (with schedule access for computed fields)
pub trait WritableField<T: EntityType>: NamedField + 'static + Send + Sync {
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
pub trait CheckedField<T: EntityType>: NamedField + 'static + Send + Sync {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::field_set::FieldSet;
    use std::sync::LazyLock;

    // Mock entity for testing
    #[derive(Debug, Clone, PartialEq)]
    struct TestEntity {
        entity_uuid: uuid::NonNilUuid,
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
        entity_id_str: String, // For Id field type
    }

    impl crate::entity::InternalData for TestEntity {
        fn uuid(&self) -> uuid::NonNilUuid {
            self.entity_uuid
        }
        fn set_uuid(&mut self, uuid: uuid::NonNilUuid) {
            self.entity_uuid = uuid;
        }
    }

    // Mock EntityType implementation
    impl EntityType for TestEntity {
        type Data = TestEntity;
        const TYPE_NAME: &'static str = "TestEntity";
        const KIND: crate::entity::EntityKind = crate::entity::EntityKind::Panel;

        fn validate(_data: &Self::Data) -> Result<(), crate::field::validation::ValidationError> {
            Ok(())
        }

        fn field_set() -> &'static FieldSet<Self> {
            // Return a minimal field set for testing
            static TEST_FIELD_SET: LazyLock<FieldSet<TestEntity>> = LazyLock::new(|| {
                static FIELD_MAP: LazyLock<Vec<(&str, &dyn NamedField)>> = LazyLock::new(|| {
                    vec![
                        ("id", &TestIdField as &dyn NamedField),
                        ("value", &TestValueField as &dyn NamedField),
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
        Schedule
    }

    fn create_test_entity() -> TestEntity {
        TestEntity {
            entity_uuid: unsafe {
                uuid::NonNilUuid::new_unchecked(uuid::Uuid::from_bytes([
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                ]))
            },
            id: "123".to_string(),
            name: "Test".to_string(),
            value: 42,
            optional_value: Some(100),
            float_value: std::f64::consts::PI,
            optional_float: Some(2.71),
            flag: true,
            optional_flag: Some(false),
            optional_string: Some("optional".to_string()),
            timestamp: chrono::NaiveDateTime::default(),
            optional_timestamp: Some(chrono::NaiveDateTime::default()),
            duration: chrono::Duration::zero(),
            optional_duration: Some(chrono::Duration::minutes(30)),
            tags: vec!["test".to_string(), "sample".to_string()],
            entity_id_str: "entity-123".to_string(),
        }
    }

    // Simple test field implementations
    #[derive(Debug)]
    pub struct TestIdField;
    impl NamedField for TestIdField {
        fn name(&self) -> &'static str {
            "id"
        }
        fn display_name(&self) -> &'static str {
            "Test ID"
        }
        fn description(&self) -> &'static str {
            "Test ID field"
        }
    }
    impl SimpleReadableField<TestEntity> for TestIdField {
        fn read(&self, entity: &TestEntity) -> Option<FieldValue> {
            Some(FieldValue::String(entity.id.clone()))
        }
        fn is_read_computed(&self) -> bool {
            false
        }
    }
    impl SimpleWritableField<TestEntity> for TestIdField {
        fn write(&self, entity: &mut TestEntity, value: FieldValue) -> Result<(), FieldError> {
            if let FieldValue::String(v) = value {
                entity.id = v;
                Ok(())
            } else {
                Err(FieldError::ConversionError(
                    ConversionError::UnsupportedType,
                ))
            }
        }
        fn is_write_computed(&self) -> bool {
            false
        }
    }

    #[derive(Debug)]
    pub struct TestValueField;
    impl NamedField for TestValueField {
        fn name(&self) -> &'static str {
            "value"
        }
        fn display_name(&self) -> &'static str {
            "Value"
        }
        fn description(&self) -> &'static str {
            "Value field"
        }
    }
    impl SimpleReadableField<TestEntity> for TestValueField {
        fn read(&self, entity: &TestEntity) -> Option<FieldValue> {
            Some(FieldValue::Integer(entity.value))
        }
        fn is_read_computed(&self) -> bool {
            false
        }
    }
    impl SimpleWritableField<TestEntity> for TestValueField {
        fn write(&self, entity: &mut TestEntity, value: FieldValue) -> Result<(), FieldError> {
            if let FieldValue::Integer(v) = value {
                entity.value = v;
                Ok(())
            } else {
                Err(FieldError::ConversionError(
                    ConversionError::UnsupportedType,
                ))
            }
        }
        fn is_write_computed(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_named_field_trait() {
        let field = TestIdField;

        assert_eq!(field.name(), "id");
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
    #[derive(Debug)]
    pub struct TestNameField;
    impl NamedField for TestNameField {
        fn name(&self) -> &'static str {
            "name"
        }
        fn display_name(&self) -> &'static str {
            "Name"
        }
        fn description(&self) -> &'static str {
            "Name field"
        }
    }
    impl ReadableField<TestEntity> for TestNameField {
        fn read(&self, _schedule: &Schedule, entity: &TestEntity) -> Option<FieldValue> {
            if entity.name.is_empty() {
                None
            } else {
                Some(FieldValue::String(entity.name.clone()))
            }
        }
        fn is_read_computed(&self) -> bool {
            false
        }
    }
    impl WritableField<TestEntity> for TestNameField {
        fn write(
            &self,
            _schedule: &Schedule,
            entity: &mut TestEntity,
            value: FieldValue,
        ) -> Result<(), FieldError> {
            if let FieldValue::String(v) = value {
                entity.name = v;
                Ok(())
            } else {
                Err(FieldError::ConversionError(
                    ConversionError::UnsupportedType,
                ))
            }
        }
        fn is_write_computed(&self) -> bool {
            false
        }
    }

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

    // Test field with i64 (reuse TestValueField from above)

    // Test field with bool
    #[derive(Debug)]
    pub struct TestFlagField;
    impl NamedField for TestFlagField {
        fn name(&self) -> &'static str {
            "flag"
        }
        fn display_name(&self) -> &'static str {
            "Flag"
        }
        fn description(&self) -> &'static str {
            "Flag field"
        }
    }
    impl ReadableField<TestEntity> for TestFlagField {
        fn read(&self, _schedule: &Schedule, entity: &TestEntity) -> Option<FieldValue> {
            Some(FieldValue::Boolean(entity.flag))
        }
        fn is_read_computed(&self) -> bool {
            false
        }
    }
    impl WritableField<TestEntity> for TestFlagField {
        fn write(
            &self,
            _schedule: &Schedule,
            entity: &mut TestEntity,
            value: FieldValue,
        ) -> Result<(), FieldError> {
            if let FieldValue::Boolean(v) = value {
                entity.flag = v;
                Ok(())
            } else {
                Err(FieldError::ConversionError(
                    ConversionError::UnsupportedType,
                ))
            }
        }
        fn is_write_computed(&self) -> bool {
            false
        }
    }

    // Test field with Option<bool>
    #[derive(Debug)]
    #[allow(dead_code)]
    pub struct TestOptionalFlagField;
    impl NamedField for TestOptionalFlagField {
        fn name(&self) -> &'static str {
            "optional_flag"
        }
        fn display_name(&self) -> &'static str {
            "Optional Flag"
        }
        fn description(&self) -> &'static str {
            "Optional flag field"
        }
    }
    impl ReadableField<TestEntity> for TestOptionalFlagField {
        fn read(&self, _schedule: &Schedule, entity: &TestEntity) -> Option<FieldValue> {
            entity.optional_flag.map(FieldValue::Boolean)
        }
        fn is_read_computed(&self) -> bool {
            false
        }
    }
    impl WritableField<TestEntity> for TestOptionalFlagField {
        fn write(
            &self,
            _schedule: &Schedule,
            entity: &mut TestEntity,
            value: FieldValue,
        ) -> Result<(), FieldError> {
            if let FieldValue::Boolean(v) = value {
                entity.optional_flag = Some(v);
                Ok(())
            } else {
                Err(FieldError::ConversionError(
                    ConversionError::UnsupportedType,
                ))
            }
        }
        fn is_write_computed(&self) -> bool {
            false
        }
    }

    // Test field with Option<String>
    #[derive(Debug)]
    pub struct TestOptionalField;
    impl NamedField for TestOptionalField {
        fn name(&self) -> &'static str {
            "optional_string"
        }
        fn display_name(&self) -> &'static str {
            "Optional"
        }
        fn description(&self) -> &'static str {
            "Optional field"
        }
    }
    impl ReadableField<TestEntity> for TestOptionalField {
        fn read(&self, _schedule: &Schedule, entity: &TestEntity) -> Option<FieldValue> {
            entity.optional_string.clone().map(FieldValue::String)
        }
        fn is_read_computed(&self) -> bool {
            false
        }
    }
    impl WritableField<TestEntity> for TestOptionalField {
        fn write(
            &self,
            _schedule: &Schedule,
            entity: &mut TestEntity,
            value: FieldValue,
        ) -> Result<(), FieldError> {
            if let FieldValue::String(v) = value {
                entity.optional_string = Some(v);
                Ok(())
            } else {
                Err(FieldError::ConversionError(
                    ConversionError::UnsupportedType,
                ))
            }
        }
        fn is_write_computed(&self) -> bool {
            false
        }
    }

    // Test field with Option<i64>
    #[derive(Debug)]
    pub struct TestOptionalValueField;
    impl NamedField for TestOptionalValueField {
        fn name(&self) -> &'static str {
            "optional_value"
        }
        fn display_name(&self) -> &'static str {
            "Optional Value"
        }
        fn description(&self) -> &'static str {
            "Optional value field"
        }
    }
    impl ReadableField<TestEntity> for TestOptionalValueField {
        fn read(&self, _schedule: &Schedule, entity: &TestEntity) -> Option<FieldValue> {
            entity.optional_value.map(FieldValue::Integer)
        }
        fn is_read_computed(&self) -> bool {
            false
        }
    }
    impl WritableField<TestEntity> for TestOptionalValueField {
        fn write(
            &self,
            _schedule: &Schedule,
            entity: &mut TestEntity,
            value: FieldValue,
        ) -> Result<(), FieldError> {
            if let FieldValue::Integer(v) = value {
                entity.optional_value = Some(v);
                Ok(())
            } else {
                Err(FieldError::ConversionError(
                    ConversionError::UnsupportedType,
                ))
            }
        }
        fn is_write_computed(&self) -> bool {
            false
        }
    }

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
        assert!(!entity.flag);
    }

    // Test field with bool (reuse TestFlagField from above)

    #[test]
    fn test_boolean_field_read() {
        let field = TestFlagField;
        let entity = create_test_entity();
        let schedule = create_mock_schedule();

        let value = ReadableField::read(&field, &schedule, &entity);
        assert!(value.is_some());

        match value.unwrap() {
            FieldValue::Boolean(b) => assert!(b),
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
        assert!(!entity.flag);
    }

    // Computed field for testing
    #[derive(Debug)]
    pub struct ComputedTestField;

    impl NamedField for ComputedTestField {
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
            Some(FieldValue::Integer(entity.value * 2))
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
            FieldValue::Integer(i) => assert_eq!(i, 84), // 42 * 2
            _ => panic!("Expected Integer value"),
        }

        assert!(field.is_read_computed());
    }

    // Test that Field trait is automatically implemented
    #[derive(Debug)]
    struct FullTestField;

    impl NamedField for FullTestField {
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
        assert_eq!(field.name(), "id");
        assert_eq!(field.display_name(), "Test ID");
        assert_eq!(field.description(), "Test ID field");

        let field = TestOptionalField;
        assert_eq!(field.name(), "optional_string");
        assert_eq!(field.display_name(), "Optional");
        assert_eq!(field.description(), "Optional field");

        let field = TestValueField;
        assert_eq!(field.name(), "value");
        assert_eq!(field.display_name(), "Value");
        assert_eq!(field.description(), "Value field");

        let field = TestFlagField;
        assert_eq!(field.name(), "flag");
        assert_eq!(field.display_name(), "Flag");
        assert_eq!(field.description(), "Flag field");
    }
}
