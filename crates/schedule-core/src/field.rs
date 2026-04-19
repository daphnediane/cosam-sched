/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field trait hierarchy and [`FieldDescriptor`] for the entity/field system.
//!
//! ## Trait hierarchy
//!
//! ```text
//! NamedField          name(), display_name(), description(), aliases()
//! ReadableField<E>    read(EntityId<E>, &Schedule) → Option<FieldValue>
//! WritableField<E>    write(EntityId<E>, &mut Schedule, FieldValue) → Result<(), FieldError>
//! VerifiableField<E>  verify(EntityId<E>, &Schedule, &FieldValue) → Result<(), VerificationError>
//! ```
//!
//! All traits are flat — no `Simple*` or `Schedule*` sub-traits.
//! The caller-facing API is always `(EntityId<E>, &[mut] Schedule)`.
//! Entity-level matching is handled via [`crate::lookup::EntityMatcher`].
//!
//! [`FieldDescriptor`] holds [`ReadFn<E>`] and [`WriteFn<E>`] enums that
//! select the correct calling convention internally, avoiding the double-`&mut`
//! borrow problem for edge-mutating fields (e.g. `add_presenters`).

use crate::entity::{EntityId, EntityType};
use crate::schedule::Schedule;
use crate::value::{CrdtFieldType, FieldError, FieldType, FieldValue, VerificationError};

/// How a field reads its value: directly from [`EntityType::InternalData`], or
/// via a [`Schedule`] lookup by [`EntityId`].
pub enum ReadFn<E: EntityType> {
    /// Data-only read — no schedule access needed.
    Bare(fn(&E::InternalData) -> Option<FieldValue>),
    /// Schedule-aware read — fn receives `(&Schedule, EntityId<E>)` and
    /// performs its own entity lookup internally.
    Schedule(fn(&Schedule, EntityId<E>) -> Option<FieldValue>),
}

/// How a field writes its value: directly into [`EntityType::InternalData`], or
/// via a [`Schedule`] lookup by [`EntityId`].
///
/// The `Schedule` variant avoids the double-`&mut` borrow problem: the fn
/// receives `(&mut Schedule, EntityId<E>)` with no `&mut InternalData`
/// parameter and handles its own lookup/release internally.
pub enum WriteFn<E: EntityType> {
    /// Data-only write — no schedule access needed.
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Schedule-aware write — used for edge mutations (e.g. `add_presenters`).
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
}

/// How a field verifies its value after a batch write: directly from
/// [`EntityType::InternalData`], via a [`Schedule`] lookup, or by re-reading.
///
/// Verification checks that the field still has the value that was requested
/// after all writes in a batch have completed. This catches conflicts where
/// one computed field's write modified another field's backing data.
pub enum VerifyFn<E: EntityType> {
    /// Data-only verification — no schedule access needed.
    Bare(fn(&E::InternalData, &FieldValue) -> Result<(), VerificationError>),
    /// Schedule-aware verification — fn receives `(&Schedule, EntityId<E>)`.
    Schedule(fn(&Schedule, EntityId<E>, &FieldValue) -> Result<(), VerificationError>),
    /// Re-read verification — read the field back and compare to attempted value.
    /// Uses `read_fn` internally; fails verification if field is write-only.
    ReRead,
}

/// Base trait: every field has a canonical name, display name, description,
/// and optional aliases.
pub trait NamedField {
    /// Canonical field name used in programmatic access (snake_case).
    fn name(&self) -> &'static str;

    /// Human-readable display name for UI presentation.
    fn display_name(&self) -> &'static str;

    /// Short description of the field's purpose.
    fn description(&self) -> &'static str;

    /// Alternative names accepted during lookup (e.g. singular/plural forms).
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    /// Returns `true` if `query` matches the canonical name or any alias
    /// (case-insensitive).
    fn matches_name(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        if self.name().to_lowercase() == q {
            return true;
        }
        self.aliases().iter().any(|a| a.to_lowercase() == q)
    }
}

/// Field that can produce a [`FieldValue`] given an entity ID and schedule.
///
/// Returns `Err(FieldError::WriteOnly)` for write-only fields.
pub trait ReadableField<E: EntityType>: NamedField {
    fn read(&self, id: EntityId<E>, schedule: &Schedule) -> Result<Option<FieldValue>, FieldError>;
}

/// Field that can accept a [`FieldValue`] given an entity ID and schedule.
///
/// Returns `Err(FieldError::ReadOnly)` for read-only fields.
/// Returns `Err(FieldError::NotFound)` if the entity is absent from the schedule.
pub trait WritableField<E: EntityType>: NamedField {
    fn write(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError>;
}

/// Field that can be verified after a batch write.
///
/// Verification checks that the field still has the value that was requested
/// after all writes in a batch have completed. This is essential for computed
/// fields that may have their backing data modified by other field writes.
pub trait VerifiableField<E: EntityType>: NamedField {
    /// Verify that the field has the expected value after batch writes.
    ///
    /// Called after all writes in a batch are complete. The `attempted` parameter
    /// is the value that was originally passed to `write()` for this field.
    ///
    /// Returns `Ok(())` if verification passes, or `Err(VerificationError)` if:
    /// - The field value changed during the batch (another write modified it)
    /// - The field cannot be verified (no `verify_fn` or `read_fn`)
    fn verify(
        &self,
        id: EntityId<E>,
        schedule: &Schedule,
        attempted: &FieldValue,
    ) -> Result<(), VerificationError>;
}

/// Generic field descriptor — one `static` value per field on an entity type.
///
/// Uses enum fn pointers so it can be stored as a `static` value.
/// Non-capturing closures coerce to fn pointers automatically.
///
/// - `read_fn: None` — field is write-only; `read()` returns `FieldError::WriteOnly`.
/// - `write_fn: None` — field is read-only; `write()` returns `FieldError::ReadOnly`.
/// - `verify_fn: None` — field uses automatic read-back verification if `read_fn` is present.
///
/// # Example
///
/// ```ignore
/// static FIELD_NAME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
///     name: "name",
///     display: "Panel Name",
///     description: "The title of the panel.",
///     aliases: &[],
///     required: true,
///     crdt_type: CrdtFieldType::Scalar,
///     field_type: FieldType::Single(FieldTypeItem::String),
///     read_fn: Some(ReadFn::Bare(|d| Some(FieldValue::String(d.data.name.clone())))),
///     write_fn: Some(WriteFn::Bare(|d, v| { d.data.name = v.into_string()?; Ok(()) })),
/// };
///
/// static FIELD_ADD_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
///     name: "add_presenters",
///     display: "Add Presenters",
///     description: "Add presenters to this panel.",
///     aliases: &[],
///     required: false,
///     crdt_type: CrdtFieldType::Derived,
///     field_type: FieldType::List(FieldTypeItem::EntityIdentifier("presenter")),
///     read_fn: None,
///     write_fn: Some(WriteFn::Schedule(|schedule, id, v| { todo!() })),
/// };
/// ```
pub struct FieldDescriptor<E: EntityType> {
    /// Canonical field name (snake_case).
    pub name: &'static str,
    /// Human-readable display name.
    pub display: &'static str,
    /// Short description of the field's purpose.
    pub description: &'static str,
    /// Alternative names accepted during lookup.
    pub aliases: &'static [&'static str],
    /// Whether the field is required (must be non-empty).
    pub required: bool,
    /// CRDT storage type annotation for Phase 4.
    pub crdt_type: CrdtFieldType,
    /// Logical field type (value type and cardinality).
    pub field_type: FieldType,
    /// Example value for documentation and UI hints.
    pub example: &'static str,
    /// Display/iteration order (lower values first). Used by `FieldSet::from_inventory`
    /// to produce a stable field ordering when fields self-register via inventory.
    pub order: u32,
    /// Read implementation. `None` means write-only.
    pub read_fn: Option<ReadFn<E>>,
    /// Write implementation. `None` means read-only.
    pub write_fn: Option<WriteFn<E>>,
    /// Verification implementation. `None` means use automatic read-back if `read_fn` is present.
    pub verify_fn: Option<VerifyFn<E>>,
}

impl<E: EntityType> NamedField for FieldDescriptor<E> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn display_name(&self) -> &'static str {
        self.display
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.aliases
    }
}

impl<E: EntityType> ReadableField<E> for FieldDescriptor<E> {
    fn read(&self, id: EntityId<E>, schedule: &Schedule) -> Result<Option<FieldValue>, FieldError> {
        match &self.read_fn {
            None => Err(FieldError::WriteOnly { name: self.name }),
            Some(ReadFn::Bare(f)) => Ok(schedule.get_internal::<E>(id).and_then(f)),
            Some(ReadFn::Schedule(f)) => Ok(f(schedule, id)),
        }
    }
}

impl<E: EntityType> WritableField<E> for FieldDescriptor<E> {
    fn write(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match &self.write_fn {
            None => Err(FieldError::ReadOnly { name: self.name }),
            Some(WriteFn::Bare(f)) => {
                let data = schedule
                    .get_internal_mut::<E>(id)
                    .ok_or(FieldError::NotFound { name: self.name })?;
                f(data, value)
            }
            Some(WriteFn::Schedule(f)) => f(schedule, id, value),
        }
    }
}

impl<E: EntityType> VerifiableField<E> for FieldDescriptor<E> {
    fn verify(
        &self,
        id: EntityId<E>,
        schedule: &Schedule,
        attempted: &FieldValue,
    ) -> Result<(), VerificationError> {
        match &self.verify_fn {
            // Custom verification functions
            Some(VerifyFn::Bare(f)) => {
                let data = schedule
                    .get_internal::<E>(id)
                    .ok_or(VerificationError::NotVerifiable { field: self.name })?;
                f(data, attempted)
            }
            Some(VerifyFn::Schedule(f)) => f(schedule, id, attempted),
            // Explicit opt-in to read-back verification
            Some(VerifyFn::ReRead) => {
                let actual = self
                    .read(id, schedule)
                    .map_err(|_| VerificationError::NotVerifiable { field: self.name })?
                    .ok_or(VerificationError::NotVerifiable { field: self.name })?;
                if actual == *attempted {
                    Ok(())
                } else {
                    Err(VerificationError::ValueChanged {
                        field: self.name,
                        requested: attempted.clone(),
                        actual,
                    })
                }
            }
            // No verification requested - success by default
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityId, EntityType};
    use crate::field_value;
    use crate::value::{CrdtFieldType, FieldError, ValidationError};
    use crate::value::{FieldType, FieldTypeItem};

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
            unimplemented!("not needed for field trait tests")
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
        name: "label",
        display: "Label",
        description: "A text label.",
        aliases: &["tag", "name"],
        required: true,
        crdt_type: CrdtFieldType::Scalar,
        field_type: FieldType::Single(FieldTypeItem::String),
        example: "Hello World",
        order: 0,
        read_fn: Some(ReadFn::Bare(|d: &MockInternalData| {
            Some(field_value!(d.label.clone()))
        })),
        write_fn: Some(WriteFn::Bare(|d: &mut MockInternalData, v| {
            d.label = v.into_string()?;
            Ok(())
        })),
        verify_fn: None,
    };

    static COUNT_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "count",
        display: "Count",
        description: "An integer count.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Scalar,
        field_type: FieldType::Single(FieldTypeItem::Integer),
        example: "7",
        order: 100,
        read_fn: Some(ReadFn::Bare(|d: &MockInternalData| {
            Some(field_value!(d.count))
        })),
        write_fn: Some(WriteFn::Bare(|d: &mut MockInternalData, v| {
            d.count = v.into_integer()?;
            Ok(())
        })),
        verify_fn: None,
    };

    static READONLY_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "readonly",
        display: "Read Only",
        description: "Always 42.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType::Single(FieldTypeItem::Integer),
        example: "42",
        order: 200,
        read_fn: Some(ReadFn::Bare(|_: &MockInternalData| Some(field_value!(42)))),
        write_fn: None,
        verify_fn: None,
    };

    static WRITEONLY_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "writeonly",
        display: "Write Only",
        description: "Accepts a label update but cannot be read back.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType::Single(FieldTypeItem::String),
        example: "Hello World",
        order: 300,
        read_fn: None,
        write_fn: Some(WriteFn::Bare(|d: &mut MockInternalData, v| {
            d.label = v.into_string()?;
            Ok(())
        })),
        verify_fn: None,
    };

    fn make_data() -> MockInternalData {
        MockInternalData {
            label: "Hello World".into(),
            count: 7,
        }
    }

    fn make_id() -> EntityId<MockEntity> {
        EntityId::new(uuid::Uuid::new_v4()).expect("v4 uuid is never nil")
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

    // --- ReadableField ---

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

    // --- WritableField ---

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
