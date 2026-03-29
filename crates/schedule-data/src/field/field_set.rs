/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! FieldSet system for managing collections of NamedFields for entities

use std::fmt;
use std::sync::LazyLock;

use crate::entity::EntityType;
use crate::field::traits::NamedField;

/// A static collection of fields for a specific entity type
pub struct FieldSet<T: EntityType> {
    /// Static list of all named fields for this entity
    pub fields: &'static [&'static dyn NamedField<T>],

    /// Map of field names (including aliases) to field references
    pub name_map: &'static [(&'static str, &'static dyn NamedField<T>)],

    /// Collection of required field names
    pub required_fields: &'static [&'static str],
}

impl<T: EntityType> FieldSet<T> {
    /// Create a new FieldSet
    pub const fn new(
        fields: &'static [&'static dyn NamedField<T>],
        name_map: &'static [(&'static str, &'static dyn NamedField<T>)],
        required_fields: &'static [&'static str],
    ) -> Self {
        Self {
            fields,
            name_map,
            required_fields,
        }
    }

    /// Get a field by name (including aliases)
    pub fn get_field(&self, name: &str) -> Option<&'static dyn NamedField<T>> {
        self.name_map
            .iter()
            .find(|(field_name, _)| *field_name == name)
            .map(|(_, field)| *field)
    }

    /// Check if a field is required
    pub fn is_required(&self, name: &str) -> bool {
        self.required_fields.contains(&name)
    }

    /// Get all required fields
    pub fn get_required_fields(&self) -> Vec<&'static dyn NamedField<T>> {
        self.required_fields
            .iter()
            .filter_map(|name| self.get_field(name))
            .collect()
    }

    /// Get all field names (including aliases)
    pub fn get_all_names(&self) -> impl Iterator<Item = &'static str> {
        self.name_map.iter().map(|(name, _)| *name)
    }

    /// Get primary field names (no aliases)
    pub fn get_primary_names(&self) -> impl Iterator<Item = &'static str> {
        self.fields.iter().map(|field| field.name())
    }

    /// Validate that all required fields are present and valid
    pub fn validate_required_fields(
        &self,
        _entity: &T::Data,
        _schedule: &crate::schedule::Schedule,
    ) -> Result<(), FieldValidationError> {
        let mut missing_fields = Vec::new();

        for field_name in self.required_fields {
            if let Some(_field) = self.get_field(field_name) {
                // For now, just check if the field exists
                // In a full implementation, we'd try to read the field value
                // and check if it's present/valid
            } else {
                missing_fields.push(*field_name);
            }
        }

        if missing_fields.is_empty() {
            Ok(())
        } else {
            Err(FieldValidationError::MissingRequiredFields(missing_fields))
        }
    }
}

/// Errors that can occur during field set operations
#[derive(Debug, Clone)]
pub enum FieldSetError {
    FieldNotFound(String),
    FieldNotWritable(String),
    WriteError(String, crate::field::FieldError),
}

impl fmt::Display for FieldSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldSetError::FieldNotFound(name) => write!(f, "Field '{}' not found", name),
            FieldSetError::FieldNotWritable(name) => write!(f, "Field '{}' is not writable", name),
            FieldSetError::WriteError(name, err) => {
                write!(f, "Error writing field '{}': {}", name, err)
            }
        }
    }
}

impl std::error::Error for FieldSetError {}

/// Errors that can occur during field validation
#[derive(Debug, Clone)]
pub enum FieldValidationError {
    MissingRequiredFields(Vec<&'static str>),
}

impl fmt::Display for FieldValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValidationError::MissingRequiredFields(fields) => {
                write!(f, "Missing required fields: ")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", field)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for FieldValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    // Import macros from the new location
    use crate::entity::macros::{field_map, field_set};

    // Mock entity type for testing
    #[derive(Debug, Clone)]
    struct TestEntity {
        id: u32,
        name: String,
        value: i32,
    }

    // Mock EntityType implementation
    impl EntityType for TestEntity {
        type Data = TestEntity;
        const TYPE_NAME: &'static str = "TestEntity";

        fn validate(_data: &Self::Data) -> Result<(), crate::field::validation::ValidationError> {
            Ok(())
        }

        fn field_set() -> &'static FieldSet<Self> {
            static TEST_FIELD_SET: LazyLock<FieldSet<TestEntity>> = LazyLock::new(|| {
                let field_map = vec![
                    ("id", &ID_FIELD as &dyn NamedField<TestEntity>),
                    ("name", &NAME_FIELD as &dyn NamedField<TestEntity>),
                    ("value", &VALUE_FIELD as &dyn NamedField<TestEntity>),
                ];
                FieldSet::new(
                    &[&ID_FIELD, &NAME_FIELD, &VALUE_FIELD],
                    &field_map,
                    &["id"], // only id is required
                )
            });
            &TEST_FIELD_SET
        }
    }

    // Mock field implementations
    struct TestField {
        name: &'static str,
        display_name: &'static str,
        description: &'static str,
    }

    impl TestField {
        const fn new(
            name: &'static str,
            display_name: &'static str,
            description: &'static str,
        ) -> Self {
            Self {
                name,
                display_name,
                description,
            }
        }
    }

    impl NamedField<TestEntity> for TestField {
        fn name(&self) -> &'static str {
            self.name
        }

        fn display_name(&self) -> &'static str {
            self.display_name
        }

        fn description(&self) -> &'static str {
            self.description
        }
    }

    // Test field instances
    static ID_FIELD: TestField = TestField::new("id", "ID", "Unique identifier");
    static NAME_FIELD: TestField = TestField::new("name", "Name", "Entity name");
    static VALUE_FIELD: TestField = TestField::new("value", "Value", "Numeric value");

    // Field name mapping with aliases
    static FIELD_MAP: &[(&str, &dyn NamedField<TestEntity>)] = &[
        ("id", &ID_FIELD),
        ("uid", &ID_FIELD), // alias
        ("name", &NAME_FIELD),
        ("title", &NAME_FIELD), // alias
        ("value", &VALUE_FIELD),
        ("number", &VALUE_FIELD), // alias
    ];

    // Test field set
    static TEST_FIELD_SET: FieldSet<TestEntity> = FieldSet::new(
        &[&ID_FIELD, &NAME_FIELD, &VALUE_FIELD],
        FIELD_MAP,
        &["id", "name"], // required fields
    );

    fn create_test_entity() -> TestEntity {
        TestEntity {
            id: 123,
            name: "Test Entity".to_string(),
            value: 42,
        }
    }

    #[test]
    fn test_field_set_creation() {
        static TEST_FIELDS: &[&dyn NamedField<TestEntity>] = &[&ID_FIELD, &NAME_FIELD];
        static TEST_NAME_MAP: &[(&str, &dyn NamedField<TestEntity>)] =
            &[("id", &ID_FIELD), ("name", &NAME_FIELD)];
        static TEST_REQUIRED: &[&str] = &["id"];

        let field_set = FieldSet::<TestEntity>::new(TEST_FIELDS, TEST_NAME_MAP, TEST_REQUIRED);

        assert_eq!(field_set.fields.len(), 2);
        assert_eq!(field_set.name_map.len(), 2);
        assert_eq!(field_set.required_fields.len(), 1);
    }

    #[test]
    fn test_get_field_by_name() {
        // Test primary names
        assert!(TEST_FIELD_SET.get_field("id").is_some());
        assert!(TEST_FIELD_SET.get_field("name").is_some());
        assert!(TEST_FIELD_SET.get_field("value").is_some());

        // Test aliases
        assert!(TEST_FIELD_SET.get_field("uid").is_some());
        assert!(TEST_FIELD_SET.get_field("title").is_some());
        assert!(TEST_FIELD_SET.get_field("number").is_some());

        // Test non-existent field
        assert!(TEST_FIELD_SET.get_field("nonexistent").is_none());
    }

    #[test]
    fn test_is_required() {
        assert!(TEST_FIELD_SET.is_required("id"));
        assert!(TEST_FIELD_SET.is_required("name"));
        assert!(!TEST_FIELD_SET.is_required("value"));

        // Test with aliases
        assert!(TEST_FIELD_SET.is_required("uid")); // alias for required field
        assert!(!TEST_FIELD_SET.is_required("number")); // alias for optional field
        assert!(TEST_FIELD_SET.is_required("title")); // alias for required field
    }

    #[test]
    fn test_get_required_fields() {
        let required_fields = TEST_FIELD_SET.get_required_fields();
        assert_eq!(required_fields.len(), 2);

        let required_names: Vec<&str> = required_fields.iter().map(|f| f.name()).collect();
        assert!(required_names.contains(&"id"));
        assert!(required_names.contains(&"name"));
        assert!(!required_names.contains(&"value"));
    }

    #[test]
    fn test_get_all_names() {
        let all_names: Vec<&str> = TEST_FIELD_SET.get_all_names().collect();
        assert_eq!(all_names.len(), 6); // 3 primary + 3 aliases

        assert!(all_names.contains(&"id"));
        assert!(all_names.contains(&"uid"));
        assert!(all_names.contains(&"name"));
        assert!(all_names.contains(&"title"));
        assert!(all_names.contains(&"value"));
        assert!(all_names.contains(&"number"));
    }

    #[test]
    fn test_get_primary_names() {
        let primary_names: Vec<&str> = TEST_FIELD_SET.get_primary_names().collect();
        assert_eq!(primary_names.len(), 3);

        assert!(primary_names.contains(&"id"));
        assert!(primary_names.contains(&"name"));
        assert!(primary_names.contains(&"value"));

        // Should not contain aliases
        assert!(!primary_names.contains(&"uid"));
        assert!(!primary_names.contains(&"title"));
        assert!(!primary_names.contains(&"number"));
    }

    #[test]
    fn test_validate_required_fields_success() {
        let entity = create_test_entity();
        let schedule = create_mock_schedule();

        let result = TEST_FIELD_SET.validate_required_fields(&entity, &schedule);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_required_fields_missing() {
        // Create a field set with a required field that doesn't exist
        static TEST_FIELDS: &[&dyn NamedField<TestEntity>] = &[&ID_FIELD];
        static TEST_NAME_MAP: &[(&str, &dyn NamedField<TestEntity>)] = &[("id", &ID_FIELD)];
        static TEST_REQUIRED: &[&str] = &["id", "nonexistent"]; // "nonexistent" doesn't exist

        let incomplete_field_set =
            FieldSet::<TestEntity>::new(TEST_FIELDS, TEST_NAME_MAP, TEST_REQUIRED);

        let entity = create_test_entity();
        let schedule = create_mock_schedule();

        let result = incomplete_field_set.validate_required_fields(&entity, &schedule);
        assert!(result.is_err());

        match result.unwrap_err() {
            FieldValidationError::MissingRequiredFields(fields) => {
                assert_eq!(fields.len(), 1);
                assert!(fields.contains(&"nonexistent"));
            }
        }
    }

    #[test]
    fn test_field_error_display() {
        let not_found = FieldSetError::FieldNotFound("test".to_string());
        assert_eq!(not_found.to_string(), "Field 'test' not found");

        let not_writable = FieldSetError::FieldNotWritable("test".to_string());
        assert_eq!(not_writable.to_string(), "Field 'test' is not writable");
    }

    #[test]
    fn test_validation_error_display() {
        let error = FieldValidationError::MissingRequiredFields(vec!["field1", "field2"]);
        let display = error.to_string();
        assert!(display.contains("Missing required fields"));
        assert!(display.contains("field1"));
        assert!(display.contains("field2"));
    }

    fn create_mock_schedule() -> crate::schedule::Schedule {
        // This is a minimal mock - in real usage this would be a proper Schedule
        crate::schedule::Schedule::default()
    }

    #[test]
    fn test_field_map_macro() {
        let field_map = field_map!(TestEntity,
            ID_FIELD => ["uid", "identifier"],
            NAME_FIELD => ["title"]
        );

        assert_eq!(field_map.len(), 5); // 2 primary + 3 aliases

        // Check primary mappings
        assert!(field_map
            .iter()
            .any(|(name, field)| *name == "id" && *field as *const _ == &ID_FIELD as *const _));
        assert!(field_map
            .iter()
            .any(|(name, field)| *name == "name" && *field as *const _ == &NAME_FIELD as *const _));

        // Check alias mappings
        assert!(field_map
            .iter()
            .any(|(name, field)| *name == "uid" && *field as *const _ == &ID_FIELD as *const _));
        assert!(field_map
            .iter()
            .any(|(name, field)| *name == "identifier"
                && *field as *const _ == &ID_FIELD as *const _));
        assert!(
            field_map
                .iter()
                .any(|(name, field)| *name == "title"
                    && *field as *const _ == &NAME_FIELD as *const _)
        );
    }

    #[test]
    fn test_field_set_macro() {
        // Test the new unified field_set macro
        static TEST_FIELD_SET: std::sync::LazyLock<FieldSet<TestEntity>> = field_set!(TestEntity, {
            fields: [&ID_FIELD, &NAME_FIELD],
            required: ["id"]
        });

        let field_set = &*TEST_FIELD_SET;
        assert_eq!(field_set.fields.len(), 2);
        assert!(field_set.get_field("id").is_some());
        assert!(field_set.get_field("name").is_some());
        assert!(field_set.is_required("id"));
        assert!(!field_set.is_required("name"));
    }
}
