/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

// TODO: These tests are disabled because the macro now emits fully-qualified
// crate::field and crate::entity paths that only resolve when used from within
// the schedule-data crate.  Move test coverage to schedule-data/tests/ as
// integration tests (see entity_fields_integration.rs).
//
// The mock types below no longer match the real trait signatures either
// (e.g., NamedField is no longer generic).

#![allow(dead_code, unused_imports)]
// use schedule_macro::EntityFields;  // disabled — see note above

/*
// Mock the required traits and types for testing - this avoids circular dependencies
pub trait EntityType {
    type Data;
    const TYPE_NAME: &'static str;
    fn field_set() -> &'static FieldSet<Self>
    where
        Self: Sized;
    fn validate(data: &Self::Data) -> Result<(), ValidationError>;
}

pub struct FieldSet<T: ?Sized> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: EntityType + ?Sized> FieldSet<T> {
    pub fn new(
        _fields: &[&dyn NamedField<T>],
        _name_map: &[(&str, &dyn NamedField<T>)],
        _required_fields: &[&str],
        _indexable_fields: &[&dyn IndexableField<T>],
    ) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

pub trait NamedField<T: ?Sized> {
    fn name(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn description(&self) -> &'static str;
}

pub trait IndexableField<T: ?Sized + EntityType>: NamedField<T> {
    fn is_indexable(&self) -> bool;
    fn match_field(&self, query: &str, entity: &T::Data) -> Option<MatchStrength>;
    fn index_priority(&self) -> u8 {
        100
    }
}

// Mock field traits
pub trait SimpleReadableField<T: EntityType>: NamedField<T> {
    fn read(&self, entity: &T::Data) -> Option<FieldValue>;
    fn is_read_computed(&self) -> bool;
}

pub trait SimpleWritableField<T: EntityType>: NamedField<T> {
    fn write(&self, entity: &mut T::Data, value: FieldValue) -> Result<(), FieldError>;
    fn is_write_computed(&self) -> bool;
}

pub trait SimpleCheckedField<T: EntityType>: NamedField<T> {
    fn validate(&self, entity: &mut T::Data, value: &FieldValue) -> Result<(), ValidationError>;
}

// Mock full field traits (with schedule access)
pub trait ReadableField<T: EntityType>: NamedField<T> {
    fn read(&self, schedule: &Schedule, entity: &T::Data) -> Option<FieldValue>;
    fn is_read_computed(&self) -> bool;
}

pub trait WritableField<T: EntityType>: NamedField<T> {
    fn write(
        &self,
        schedule: &Schedule,
        entity: &mut T::Data,
        value: FieldValue,
    ) -> Result<(), FieldError>;
    fn is_write_computed(&self) -> bool;
}

pub trait CheckedField<T: EntityType>: NamedField<T> {
    fn validate(
        &self,
        schedule: &Schedule,
        entity: &mut T::Data,
        value: &FieldValue,
    ) -> Result<(), ValidationError>;
}

// Mock supporting types
#[derive(Debug, Clone)]
pub enum FieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(String), // Simplified
    Duration(String), // Simplified
    List(Vec<FieldValue>),
    Map(std::collections::HashMap<String, FieldValue>),
    Id(String),
}

impl FieldValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            FieldValue::String(s) => Some(s),
            _ => None,
        }
    }
}

// Implement From for common types
impl From<&String> for FieldValue {
    fn from(s: &String) -> Self {
        FieldValue::String(s.clone())
    }
}

impl From<&str> for FieldValue {
    fn from(s: &str) -> Self {
        FieldValue::String(s.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum ConversionError {
    InvalidTimestamp,
    InvalidFormat,
    UnsupportedType,
    OutOfRange,
}

#[derive(Debug)]
pub enum FieldError {
    CannotStoreComputedField,
    CannotStoreRelationshipField,
    ConversionError(ConversionError),
    ValidationError(ValidationError),
}

#[derive(Debug)]
pub enum ValidationError {
    RequiredFieldMissing {
        field: String,
    },
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },
    ValidationFailed {
        field: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchStrength {
    NotMatch = 0,
    WeakMatch = 1,
    StrongMatch = 2,
    ExactMatch = 3,
}

#[derive(Debug)]
pub struct Schedule;

#[test]
fn test_simple_entity_compilation() {
    // This test just verifies that the macro can be used without compilation errors
    // We'll create a simple struct and derive EntityFields on it

    #[derive(EntityFields)]
    struct TestEntity {
        #[field(display = "Name", description = "Entity name")]
        #[alias("name", "title")]
        #[required]
        pub name: String,

        pub value: i32,
    }

    // If we get here, the macro compilation worked
    assert_eq!(TestEntity::TYPE_NAME, "TestEntity");
}

#[test]
fn test_explicit_field_naming() {
    #[derive(EntityFields)]
    struct TestExplicitNaming {
        #[field(display = "Custom Name", description = "A field with custom naming")]
        #[field_name("MyCustomField")]
        #[field_const("MY_CUSTOM_FIELD")]
        pub custom_field: String,

        #[field(display = "Regular Field", description = "A field with auto naming")]
        pub regular_field: i32,
    }

    // If we get here, the macro compilation worked
    assert_eq!(TestExplicitNaming::TYPE_NAME, "TestExplicitNaming");

    // Test that we can access the fields by their constants
    // This would require the actual field types to be implemented
    // For now, just test compilation
}

#[test]
fn test_edge_entity_compilation() {
    #[derive(EntityFields)]
    struct TestEdge {
        #[field(display = "From UID", description = "Source entity UID")]
        #[alias("from", "from_uid", "fromUID", "member")]
        pub from_uid: String,

        #[field(display = "To UID", description = "Target entity UID")]
        #[alias("to", "to_uid", "toUID", "group")]
        #[required]
        pub to_uid: String,

        #[field(display = "Edge Type", description = "Type of relationship")]
        #[alias("type", "edge_type", "edgeType")]
        #[required]
        pub edge_type: TestEdgeType,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum TestEdgeType {
        PanelToRoom,
        PanelToPanelType,
    }

    // If we get here, the macro compilation worked
    assert_eq!(TestEdge::TYPE_NAME, "TestEdge");
}

#[test]
fn test_field_alias_access() {
    #[derive(EntityFields)]
    struct TestAliases {
        #[field(display = "Name Field", description = "Entity name")]
        #[alias("name", "title", "label")]
        #[field_name("NameField")]
        #[field_const("NAME_FIELD")]
        pub name: String,

        #[field(display = "Value Field", description = "Entity value")]
        #[alias("value", "amount", "number")]
        #[field_name("ValueField")]
        #[field_const("VALUE_FIELD")]
        pub value: i32,
    }

    // If we get here, the macro compilation worked
    assert_eq!(TestAliases::TYPE_NAME, "TestAliases");
}

#[test]
fn test_field_constant_access() {
    #[derive(EntityFields)]
    struct TestFieldAccess {
        #[field(display = "Name", description = "Entity name")]
        #[field_name("NameField")]
        #[field_const("NAME_FIELD")]
        #[alias("name", "title", "label")]
        pub name: String,

        #[field(display = "Value", description = "Entity value")]
        #[field_name("ValueField")]
        #[field_const("VALUE_FIELD")]
        #[alias("value", "amount", "number")]
        pub value: i32,
    }

    // Test that the entity type name is correct
    assert_eq!(TestFieldAccess::TYPE_NAME, "TestFieldAccess");

    // Test that we can access the field constants
    // This demonstrates that the field constants are generated correctly
    let _name_field = &NAME_FIELD;
    let _value_field = &VALUE_FIELD;

    // Create test data
    let test_data = TestFieldAccess {
        name: "Test Name".to_string(),
        value: 42,
    };

    // Test validation (should pass since name is not empty)
    assert!(TestFieldAccess::validate(&test_data).is_ok());

    // Test validation failure (empty name)
    let empty_data = TestFieldAccess {
        name: "".to_string(),
        value: 42,
    };
}

#[test]
fn test_auto_generated_field_names() {
    #[derive(EntityFields)]
    struct TestAutoNames {
        #[field(display = "User ID", description = "Unique identifier")]
        pub user_id: String,

        #[field(display = "Created At", description = "Timestamp")]
        pub created_at: String,
    }

    // Test that auto-generated field names work
    // These should be: TestAutoNamesUserIdField and TestAutoNamesCreatedAtField
    assert_eq!(TestAutoNames::TYPE_NAME, "TestAutoNames");

    // The field constants should be auto-generated as:
    // FIELD_TESTAUTONAMESUSERIDFIELD and FIELD_TESTAUTONAMESCREATEDATFIELD

    // Test basic functionality
    let test_data = TestAutoNames {
        user_id: "user123".to_string(),
        created_at: "2026-03-30".to_string(),
    };

    assert!(TestAutoNames::validate(&test_data).is_ok());
}

// Test that unsupported types generate compile errors
// This test should fail to compile with a clear diagnostic
#[test]
fn test_unsupported_type_diagnostic() {
    // This should generate a compile_error! diagnostic
    #[derive(EntityFields)]
    struct TestUnsupportedType {
        #[field(display = "Name", description = "Name field")]
        pub name: String,

        #[field(display = "Custom", description = "Custom type")]
        pub custom_type: CustomUnsupportedType, // This should trigger the diagnostic
    }

    // If this compiles, the diagnostic isn't working
    // If it fails with our compile_error message, the diagnostic works
    assert!(true);
}

#[derive(Debug, Clone, Copy)]
pub struct CustomUnsupportedType;

// Test different closure syntaxes for computed fields
#[test]
fn test_computed_field_closure_syntaxes() {
    #[derive(EntityFields)]
    struct TestClosureSyntaxes {
        #[field(display = "Name", description = "Name field")]
        pub name: String,

        // Simple read closure (no schedule)
        #[computed_field(display = "Computed Read", description = "Simple computed read")]
        #[read(|entity| {
            Some(FieldValue::String(format!("computed_{}", entity.name)))
        })]
        pub simple_read: String,

        // Simple read and write closures (no schedule)
        #[computed_field(
            display = "Computed Read/Write",
            description = "Simple computed read/write"
        )]
        #[read(|entity| {
            Some(FieldValue::String(format!("computed_{}", entity.name)))
        })]
        #[write(|entity, value| {
            if let FieldValue::String(val) = value {
                entity.name = val;
                Ok(())
            } else {
                Err(FieldError::ConversionError(ConversionError::InvalidFormat))
            }
        })]
        pub simple_read_write: String,

        // Schedule-aware read closure
        #[computed_field(
            display = "Schedule Read",
            description = "Schedule-aware computed read"
        )]
        #[read(|schedule, entity| {
            // This would use schedule for some computation
            Some(FieldValue::String(format!("computed_{}", entity.name)))
        })]
        pub schedule_read: String,

        // Schedule-aware read and write closures
        #[computed_field(
            display = "Schedule Read/Write",
            description = "Schedule-aware computed read/write"
        )]
        #[read(|schedule, entity| {
            // This would use schedule for some computation
            Some(FieldValue::String(format!("computed_{}", entity.name)))
        })]
        #[write(|schedule, entity, value| {
            // This would use schedule for some validation or side effects
            if let FieldValue::String(val) = value {
                entity.name = val;
                Ok(())
            } else {
                Err(FieldError::ConversionError(ConversionError::InvalidFormat))
            }
        })]
        pub schedule_read_write: String,
    }

    // Test that the macro generates the correct trait implementations
    assert_eq!(TestClosureSyntaxes::TYPE_NAME, "TestClosureSyntaxes");

    let test_data = TestClosureSyntaxes {
        name: "test".to_string(),
        simple_read: "".to_string(),
        simple_read_write: "".to_string(),
        schedule_read: "".to_string(),
        schedule_read_write: "".to_string(),
    };

    assert!(TestClosureSyntaxes::validate(&test_data).is_ok());
}
*/
