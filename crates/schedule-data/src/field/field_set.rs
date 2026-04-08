/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Per-entity field registry.
//!
//! A [`FieldSet<T>`] is a static, immutable collection of every field
//! belonging to an entity type `T`.  It is created once (inside a
//! `LazyLock` in the macro-generated `EntityType::field_set()`) and
//! provides:
//!
//! - **Name lookup** — [`get_field`](FieldSet::get_field) resolves a field
//!   name or alias to its [`NamedField`] reference.
//! - **Required-field list** — [`is_required`](FieldSet::is_required) and
//!   [`validate_entity`](FieldSet::validate_entity).
//! - **Index matching** — [`match_index`](FieldSet::match_index) iterates
//!   [`IndexableField`] entries, calls `match_field` on each, and returns
//!   the single best [`FieldMatchResult`] ranked by `(strength, priority)`.
//!
//! The macro populates all four slices (`fields`, `name_map`,
//! `required_fields`, `indexable_fields`) from struct attributes.

use crate::entity::{EntityType, InternalData};
use crate::field::traits::{FieldMatchResult, IndexableField, MatchPriority, NamedField};
use crate::field::ValidationError;

/// Field set for managing entity fields with aliases and validation
#[derive(Debug)]
pub struct FieldSet<T: EntityType> {
    /// All fields for this entity type
    pub fields: &'static [&'static dyn NamedField],
    /// Name-to-field mapping for aliases
    pub name_map: &'static [(&'static str, &'static dyn NamedField)],
    /// Required field names
    pub required_fields: &'static [&'static str],
    /// Indexable fields
    pub indexable_fields: &'static [&'static dyn IndexableField<T>],
}

impl<T: EntityType> FieldSet<T> {
    /// Create a new field set
    pub fn new(
        fields: &'static [&'static dyn NamedField],
        name_map: &'static [(&'static str, &'static dyn NamedField)],
        required_fields: &'static [&'static str],
        indexable_fields: &'static [&'static dyn IndexableField<T>],
    ) -> Self {
        Self {
            fields,
            name_map,
            required_fields,
            indexable_fields,
        }
    }

    /// Get a field by name (including aliases)
    pub fn get_field(&self, name: &str) -> Option<&'static dyn NamedField> {
        self.name_map
            .iter()
            .find(|(field_name, _)| *field_name == name)
            .map(|(_, field)| *field)
    }

    /// Check if a field is required
    pub fn is_required(&self, name: &str) -> bool {
        self.required_fields.contains(&name)
    }

    /// Get all field names (including aliases)
    pub fn all_field_names(&self) -> Vec<&'static str> {
        self.name_map.iter().map(|(name, _)| *name).collect()
    }

    /// Get indexable fields
    pub fn get_indexable_fields(&self) -> &'static [&'static dyn IndexableField<T>] {
        self.indexable_fields
    }

    /// Validate an entity's data against required fields
    pub fn validate_entity(&self, _entity: &T::Data) -> Result<(), ValidationError> {
        for required_field in self.required_fields {
            if let Some(_field) = self.get_field(required_field) {
                // For now, just check if the field exists and has a value
                // In a full implementation, this would use the field's read method
                // and check for None/empty values
            }
        }
        Ok(())
    }

    /// Try to match a query string against the indexable fields of an entity.
    ///
    /// Iterates every `IndexableField` in `indexable_fields`, calls
    /// `match_field` on each one, and returns the single best
    /// `FieldMatchResult` (highest strength, then highest priority).
    /// Returns `None` when no indexable field matches.
    pub fn match_index(&self, query: &str, entity: &T::Data) -> Option<FieldMatchResult>
    where
        T::Data: InternalData,
    {
        let mut best: Option<FieldMatchResult> = None;

        for idx_field in self.indexable_fields {
            if !idx_field.is_indexable() {
                continue;
            }

            if let Some(priority) = idx_field.match_field(query, entity) {
                if priority == crate::field::traits::match_priority::NO_MATCH {
                    continue;
                }

                let candidate = FieldMatchResult {
                    entity_uuid: entity.uuid(),
                    priority,
                    field_priority: idx_field.index_priority(),
                    field_name: idx_field.name(),
                    details: None,
                };

                best = Some(match best {
                    None => candidate,
                    Some(prev) => {
                        if (candidate.priority, candidate.field_priority)
                            > (prev.priority, prev.field_priority)
                        {
                            candidate
                        } else {
                            prev
                        }
                    }
                });
            }
        }

        best
    }
}

/// Field validation error
#[derive(Debug, Clone)]
pub struct FieldValidationError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for FieldValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Field validation error for '{}': {}",
            self.field, self.message
        )
    }
}

impl std::error::Error for FieldValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock entity type for testing
    #[derive(Debug, Clone)]
    struct TestEntity {
        entity_uuid: uuid::Uuid,
        id: u32,
        name: String,
        value: i32,
    }

    impl crate::entity::InternalData for TestEntity {
        fn uuid(&self) -> uuid::Uuid {
            self.entity_uuid
        }
        fn set_uuid(&mut self, uuid: uuid::Uuid) {
            self.entity_uuid = uuid;
        }
    }

    // Mock EntityType implementation
    impl EntityType for TestEntity {
        type Data = TestEntity;

        const TYPE_NAME: &'static str = "TestEntity";

        fn field_set() -> &'static FieldSet<Self> {
            static FIELD_SET: std::sync::LazyLock<FieldSet<TestEntity>> =
                std::sync::LazyLock::new(|| FieldSet::new(&[], &[], &[], &[]));
            &FIELD_SET
        }

        fn validate(_data: &Self::Data) -> Result<(), ValidationError> {
            Ok(())
        }
    }

    // Mock field implementations
    #[derive(Debug)]
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

    impl NamedField for TestField {
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

    // Test field constants
    const ID_FIELD: TestField = TestField::new("id", "ID", "Unique identifier");
    const NAME_FIELD: TestField = TestField::new("name", "Name", "Entity name");
    const VALUE_FIELD: TestField = TestField::new("value", "Value", "Integer value");

    #[test]
    fn test_field_set_creation() {
        let field_set: FieldSet<TestEntity> =
            FieldSet::new(&[&ID_FIELD, &NAME_FIELD, &VALUE_FIELD], &[], &["id"], &[]);

        assert_eq!(field_set.fields.len(), 3);
        assert!(field_set.is_required("id"));
        assert!(!field_set.is_required("name"));
        assert!(!field_set.is_required("value"));
    }

    #[test]
    fn test_field_lookup() {
        let field_set: FieldSet<TestEntity> = FieldSet::new(
            &[&ID_FIELD, &NAME_FIELD, &VALUE_FIELD],
            &[
                ("id", &ID_FIELD),
                ("name", &NAME_FIELD),
                ("value", &VALUE_FIELD),
            ],
            &["id"],
            &[],
        );

        assert!(field_set.get_field("id").is_some());
        assert!(field_set.get_field("name").is_some());
        assert!(field_set.get_field("value").is_some());
        assert!(field_set.get_field("nonexistent").is_none());
    }
}
