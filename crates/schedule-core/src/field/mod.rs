/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field trait hierarchy and [`FieldDescriptor`] for the entity/field system.
//!
//! The caller-facing API is always `(EntityId<E>, &[mut] Schedule)`.
//! [`FieldDescriptor`] holds [`ReadFn<E>`] and [`WriteFn<E>`] enums that
//! select the correct calling convention internally, avoiding the double-`&mut`
//! borrow problem for edge-mutating fields (e.g. `add_presenters`).

pub mod callback;
pub mod descriptor;
pub mod macros;
pub mod set;
pub mod traits;

// Re-export field traits from the traits module
pub use traits::NamedField;

// Re-export callback types from the callback module
pub use callback::{AddFn, FieldCallbacks, ReadFn, RemoveFn, WriteFn};
// Re-export descriptor types from the descriptor module
pub use descriptor::FieldDescriptor;

// ── CommonFieldData ─────────────────────────────────────────────────────────────

/// Generic field data shared by all field descriptors.
///
/// Fields are `pub(crate)` so entity modules and macro-generated code within
/// `schedule-core` can initialize statics using struct literal syntax.
/// External code accesses these through the [`NamedField`] trait methods.
pub struct CommonFieldData {
    /// Canonical field name (snake_case).
    pub name: &'static str,
    /// Human-readable display name.
    pub display: &'static str,
    /// Short description of the field's purpose.
    pub description: &'static str,
    /// Alternative names accepted during lookup.
    pub aliases: &'static [&'static str],
    /// Logical field type (value type and cardinality).
    pub field_type: crate::value::FieldType,
    /// Example value for documentation and UI hints.
    pub example: &'static str,
    /// Display/iteration order (lower values first).
    pub order: u32,
}

// ── Global field registry ─────────────────────────────────────────────────────

/// Wrapper for globally registering a [`FieldDescriptor`] via
/// `inventory::submit! { CollectedField(&FIELD_NAME) }`.
pub struct CollectedField(pub &'static dyn NamedField);

/// Wrapper for globally registering a [`crate::edge::HalfEdgeDescriptor`] via
/// `inventory::submit! { CollectedHalfEdge(&HALF_EDGE_NAME) }`.
pub struct CollectedHalfEdge(pub &'static dyn NamedField);

inventory::collect!(CollectedField);
inventory::collect!(CollectedHalfEdge);

/// Iterate over all registered [`FieldDescriptor`] entries.
pub fn all_fields() -> impl Iterator<Item = &'static CollectedField> {
    inventory::iter::<CollectedField>()
}

/// Iterate over all registered [`crate::edge::HalfEdgeDescriptor`] entries.
pub fn all_half_edges() -> impl Iterator<Item = &'static CollectedHalfEdge> {
    inventory::iter::<CollectedHalfEdge>()
}

/// Iterate over every registered field and half-edge descriptor.
///
/// Chains [`all_fields`] and [`all_half_edges`]; used by [`crate::registry`]
/// for global name-based lookup.
pub fn all_named_fields() -> impl Iterator<Item = &'static dyn NamedField> {
    all_fields()
        .map(|cf| cf.0)
        .chain(all_half_edges().map(|ce| ce.0))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::CrdtFieldType;
    use crate::entity::{EntityId, EntityType};
    use crate::field_value;
    use crate::value::{FieldCardinality, FieldType, FieldTypeItem};
    use crate::value::{FieldError, ValidationError};

    /// Minimal mock entity for testing field traits without real entity types.
    struct MockEntity;

    #[derive(Clone, Debug)]
    struct MockInternalData {
        label: String,
        count: i64,
    }

    #[derive(Clone)]
    struct MockData;

    impl EntityType for MockEntity {
        type InternalData = MockInternalData;
        type Data = MockData;

        const TYPE_NAME: &'static str = "mock";

        fn uuid_namespace() -> &'static uuid::Uuid {
            static NS: std::sync::LazyLock<uuid::Uuid> = std::sync::LazyLock::new(|| {
                uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, b"mock")
            });
            &NS
        }

        fn field_set() -> &'static crate::entity::FieldSet<Self> {
            // Minimal static FieldSet so tests can use `Schedule::insert`
            // (which now mirrors every non-derived field into the CRDT doc
            // via `FieldSet::fields()`).
            static FS: std::sync::OnceLock<crate::entity::FieldSet<MockEntity>> =
                std::sync::OnceLock::new();
            FS.get_or_init(|| {
                crate::entity::FieldSet::new(&[
                    &LABEL_FIELD,
                    &COUNT_FIELD,
                    &READONLY_FIELD,
                    &WRITEONLY_FIELD,
                ])
            })
        }

        fn export(_: &Self::InternalData) -> Self::Data {
            MockData
        }

        fn validate(data: &Self::InternalData) -> Vec<ValidationError> {
            if data.label.is_empty() {
                vec![ValidationError::Required { field: "label" }]
            } else {
                vec![]
            }
        }
    }

    static LABEL_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        data: CommonFieldData {
            name: "label",
            display: "Label",
            description: "A text label.",
            aliases: &["tag", "name"],
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
            example: "Hello World",
            order: 0,
        },
        crdt_type: CrdtFieldType::Scalar,
        required: true,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d: &MockInternalData| {
                Some(field_value!(d.label.clone()))
            })),
            write_fn: Some(WriteFn::Bare(|d: &mut MockInternalData, v| {
                d.label = v.into_string()?;
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    };

    static COUNT_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        data: CommonFieldData {
            name: "count",
            display: "Count",
            description: "An integer count.",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
            example: "7",
            order: 100,
        },
        crdt_type: CrdtFieldType::Scalar,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d: &MockInternalData| {
                Some(field_value!(d.count))
            })),
            write_fn: Some(WriteFn::Bare(|d: &mut MockInternalData, v| {
                d.count = v.into_integer()?;
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    };

    static READONLY_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        data: CommonFieldData {
            name: "readonly",
            display: "Read Only",
            description: "Always 42.",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
            example: "42",
            order: 200,
        },
        crdt_type: CrdtFieldType::Derived,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|_: &MockInternalData| Some(field_value!(42)))),
            write_fn: None,
            add_fn: None,
            remove_fn: None,
        },
    };

    static WRITEONLY_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        data: CommonFieldData {
            name: "writeonly",
            display: "Write Only",
            description: "Accepts a label update but cannot be read back.",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
            example: "Hello World",
            order: 300,
        },
        crdt_type: CrdtFieldType::Derived,
        required: false,
        cb: FieldCallbacks {
            read_fn: None,
            write_fn: Some(WriteFn::Bare(|d: &mut MockInternalData, v| {
                d.label = v.into_string()?;
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    };

    fn make_data() -> MockInternalData {
        MockInternalData {
            label: "Hello World".into(),
            count: 7,
        }
    }

    fn make_id() -> EntityId<MockEntity> {
        let uuid = uuid::Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        unsafe { EntityId::new_unchecked(non_nil_uuid) }
    }

    fn make_schedule_with_data() -> (EntityId<MockEntity>, crate::schedule::Schedule) {
        let id = make_id();
        let mut sched = crate::schedule::Schedule::default();
        sched.insert(id, make_data());
        (id, sched)
    }

    // --- NamedField ---

    #[test]
    fn test_named_field_name() {
        assert_eq!(LABEL_FIELD.name(), "label");
    }

    #[test]
    fn test_named_field_display_name() {
        assert_eq!(LABEL_FIELD.display_name(), "Label");
    }

    #[test]
    fn test_named_field_description() {
        assert_eq!(LABEL_FIELD.description(), "A text label.");
    }

    #[test]
    fn test_named_field_aliases() {
        assert_eq!(LABEL_FIELD.aliases(), &["tag", "name"]);
        assert_eq!(COUNT_FIELD.aliases(), &[] as &[&str]);
    }

    #[test]
    fn test_matches_name_canonical() {
        assert!(LABEL_FIELD.matches_name("label"));
        assert!(LABEL_FIELD.matches_name("LABEL"));
    }

    #[test]
    fn test_matches_name_alias() {
        assert!(LABEL_FIELD.matches_name("tag"));
        assert!(LABEL_FIELD.matches_name("NAME"));
    }

    #[test]
    fn test_matches_name_no_match() {
        assert!(!LABEL_FIELD.matches_name("notafield"));
    }

    // --- FieldDescriptor ---

    #[test]
    fn test_read_string_field() {
        let (id, sched) = make_schedule_with_data();
        assert_eq!(
            LABEL_FIELD.read(id, &sched).unwrap(),
            Some(field_value!("Hello World"))
        );
    }

    #[test]
    fn test_read_integer_field() {
        let (id, sched) = make_schedule_with_data();
        assert_eq!(COUNT_FIELD.read(id, &sched).unwrap(), Some(field_value!(7)));
    }

    #[test]
    fn test_read_readonly_field() {
        let (id, sched) = make_schedule_with_data();
        assert_eq!(
            READONLY_FIELD.read(id, &sched).unwrap(),
            Some(field_value!(42))
        );
    }

    #[test]
    fn test_read_missing_entity_returns_none() {
        let id = make_id();
        let sched = crate::schedule::Schedule::default();
        assert_eq!(LABEL_FIELD.read(id, &sched).unwrap(), None);
    }

    #[test]
    fn test_read_writeonly_returns_error() {
        let (id, sched) = make_schedule_with_data();
        assert!(matches!(
            WRITEONLY_FIELD.read(id, &sched),
            Err(FieldError::WriteOnly { .. })
        ));
    }

    #[test]
    fn test_write_string_field() {
        let (id, mut sched) = make_schedule_with_data();
        LABEL_FIELD
            .write(id, &mut sched, field_value!("Updated"))
            .unwrap();
        assert_eq!(
            sched.get_internal::<MockEntity>(id).unwrap().label,
            "Updated"
        );
    }

    #[test]
    fn test_write_integer_field() {
        let (id, mut sched) = make_schedule_with_data();
        COUNT_FIELD.write(id, &mut sched, field_value!(99)).unwrap();
        assert_eq!(sched.get_internal::<MockEntity>(id).unwrap().count, 99);
    }

    #[test]
    fn test_write_wrong_variant_converts_with_cross_type_support() {
        let (id, mut sched) = make_schedule_with_data();
        // Integer now converts to String via cross-type conversion
        LABEL_FIELD.write(id, &mut sched, field_value!(1)).unwrap();
        assert_eq!(sched.get_internal::<MockEntity>(id).unwrap().label, "1");
    }

    #[test]
    fn test_write_readonly_returns_error() {
        let (id, mut sched) = make_schedule_with_data();
        let result = READONLY_FIELD.write(id, &mut sched, field_value!(1));
        assert!(matches!(result, Err(FieldError::ReadOnly { .. })));
    }

    #[test]
    fn test_write_missing_entity_returns_error() {
        let id = make_id();
        let mut sched = crate::schedule::Schedule::default();
        let result = LABEL_FIELD.write(id, &mut sched, field_value!("x"));
        assert!(matches!(result, Err(FieldError::NotFound { .. })));
    }

    #[test]
    fn test_write_writeonly_field() {
        let (id, mut sched) = make_schedule_with_data();
        WRITEONLY_FIELD
            .write(id, &mut sched, field_value!("via writeonly"))
            .unwrap();
        assert_eq!(
            sched.get_internal::<MockEntity>(id).unwrap().label,
            "via writeonly"
        );
    }
}
