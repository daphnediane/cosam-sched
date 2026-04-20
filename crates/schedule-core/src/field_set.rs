/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`FieldSet<E>`] — static per-entity-type field registry.
//!
//! Built once (typically inside a `std::sync::LazyLock`) and returned by
//! [`EntityType::field_set`].  Provides O(1) name/alias lookup via an internal
//! `HashMap` populated at construction time.
//!
//! # Lookup contract
//!
//! [`FieldSet::get_by_name`] matches the query **exactly** against canonical
//! names and registered aliases (no normalization is applied inside this
//! module).  Callers that receive externally-sourced strings — e.g. XLSX
//! column headers — are responsible for normalizing before lookup.
//!
//! ## XLSX header normalization
//!
//! The XLSX import layer applies the following steps to raw column headers
//! before calling `get_by_name`:
//!
//! 1. Split at camelCase lower→upper boundaries (`PanelKind` → `Panel Kind`).
//! 2. Split at uppercase-run/UpperCamelCase boundaries (`AVNotes` → `AV Notes`).
//! 3. Collapse runs of whitespace, underscores, and punctuation to `_` and trim.
//!
//! Examples: `"PanelKind"` → `"Panel_Kind"`, `"AVNotes"` → `"AV_Notes"`,
//! `"Room Name"` → `"Room_Name"`.
//!
//! **Field authors must register normalized forms as aliases** on any
//! `FieldDescriptor` that is importable from a spreadsheet.  For example, a
//! field with canonical name `"kind"` whose spreadsheet header is `"PanelKind"`
//! should include `"Panel_Kind"` in its `aliases` list.

use crate::entity::{CollectedField, EntityType};
use crate::field::{FieldDescriptor, ReadableField, VerifiableField, WritableField};
use crate::schedule::Schedule;
use crate::value::{CrdtFieldType, FieldError, FieldValue, IntoFieldValue, VerificationError};
use std::collections::HashMap;
use thiserror::Error;

/// Reference to a field in a [`FieldSet`] — either by canonical name/alias or
/// by direct descriptor pointer.
///
/// Used as the first element of each `(FieldRef<E>, FieldValue)` pair in
/// [`FieldSet::write_multiple`].  The `Descriptor` variant is zero-cost;
/// the `Name` variant costs one `HashMap` lookup.
pub enum FieldRef<E: EntityType> {
    /// Canonical name or alias of a field registered in the [`FieldSet`].
    ///
    /// Accepted for dynamic or name-driven call sites (e.g. XLSX import).
    /// Resolution fails with [`FieldSetError::UnknownField`] if the name
    /// is neither canonical nor an alias.
    Name(&'static str),

    /// Direct reference to a static [`FieldDescriptor`].
    ///
    /// Bypasses the name-map lookup and is the form produced by the
    /// `define_entity_builder!` macro.
    Descriptor(&'static FieldDescriptor<E>),
}

impl<E: EntityType> From<&'static str> for FieldRef<E> {
    fn from(name: &'static str) -> Self {
        FieldRef::Name(name)
    }
}

impl<E: EntityType> From<&'static FieldDescriptor<E>> for FieldRef<E> {
    fn from(desc: &'static FieldDescriptor<E>) -> Self {
        FieldRef::Descriptor(desc)
    }
}

/// Errors returned by [`FieldSet::write_multiple`] and [`FieldSet::write_many`].
#[derive(Debug, Error)]
pub enum FieldSetError {
    /// A [`FieldRef::Name`] did not resolve to any canonical name or alias in
    /// the [`FieldSet`].
    #[error("unknown field '{0}'")]
    UnknownField(String),

    /// The same descriptor appeared twice in a batch update.  Two references
    /// resolve to the "same" field when they point to the same static
    /// [`FieldDescriptor`] (pointer equality), regardless of whether they
    /// arrived as `Name` or `Descriptor` variants.
    #[error("duplicate field '{0}' in batch update")]
    DuplicateField(&'static str),

    /// A write failed mid-batch.  Writes are applied in caller-provided order;
    /// earlier writes remain applied even when this error is returned.
    ///
    /// The inner [`FieldError`] is boxed because it transitively contains a
    /// [`FieldValue`], which would otherwise inflate every `Result` return
    /// value in this module past the `clippy::result_large_err` threshold.
    #[error("write failed for field '{field}': {error}")]
    WriteError {
        /// Canonical name of the field whose write failed.
        field: &'static str,
        /// The underlying [`FieldError`].
        #[source]
        error: Box<FieldError>,
    },

    /// Verification failed for a field that has a `verify_fn`.
    ///
    /// Raised during the post-write verification phase after all writes in
    /// the batch have been applied.  The inner [`VerificationError`] is boxed
    /// for the same size reasons as [`FieldSetError::WriteError`].
    #[error("verification failed for field '{field}': {error}")]
    VerificationError {
        /// Canonical name of the field that failed verification.
        field: &'static str,
        /// The underlying [`VerificationError`].
        #[source]
        error: Box<VerificationError>,
    },
}

/// Static registry of all [`FieldDescriptor`]s for an entity type.
///
/// # Construction
///
/// ```ignore
/// static FIELD_SET: LazyLock<FieldSet<MyEntityType>> = LazyLock::new(|| {
///     FieldSet::new(&[&FIELD_NAME, &FIELD_CODE])
/// });
/// ```
///
/// # Lookup
///
/// All lookups accept both canonical names and aliases (case-sensitive matching
/// on the stored key; aliases are lowercased by [`crate::field::NamedField::aliases`]).
pub struct FieldSet<E: EntityType> {
    /// All descriptors in declaration order.
    fields: Vec<&'static FieldDescriptor<E>>,
    /// Maps every canonical name **and** alias → index into `fields`.
    name_map: HashMap<String, usize>,
    /// Indices of fields where `required == true`.
    required: Vec<usize>,
    /// Indices of fields that have a `read_fn`.
    readable: Vec<usize>,
    /// Indices of fields that have a `write_fn`.
    writable: Vec<usize>,
}

impl<E: EntityType> FieldSet<E> {
    /// Build a `FieldSet` from all [`CollectedField<E>`] entries submitted via
    /// `inventory::submit!`.  Fields are sorted by [`FieldDescriptor::order`]
    /// (ascending) before the lookup map is built.
    ///
    /// Each entity type module must declare `inventory::collect!(CollectedField<E>)`,
    /// and every field for that type must emit
    /// `inventory::submit! { CollectedField::<E>(&STATIC_FIELD) }`.
    #[must_use]
    pub fn from_inventory() -> Self
    where
        CollectedField<E>: inventory::Collect,
    {
        let mut descriptors: Vec<&'static FieldDescriptor<E>> =
            inventory::iter::<CollectedField<E>>()
                .map(|cf| cf.0)
                .collect();
        descriptors.sort_by_key(|d| d.order);
        Self::from_slice(&descriptors)
    }

    /// Build a `FieldSet` from a slice of static field descriptor references.
    ///
    /// Registers each descriptor's canonical name and all its aliases into the
    /// internal lookup map.  Later entries with colliding names silently
    /// overwrite earlier ones (prefer no collisions).
    ///
    /// Prefer [`FieldSet::from_inventory`] for production entity types.
    /// This constructor is kept for tests that use mock entities without
    /// an inventory collection.
    #[must_use]
    pub fn new(descriptors: &[&'static FieldDescriptor<E>]) -> Self {
        Self::from_slice(descriptors)
    }

    fn from_slice(descriptors: &[&'static FieldDescriptor<E>]) -> Self {
        let fields: Vec<&'static FieldDescriptor<E>> = descriptors.to_vec();
        let mut name_map: HashMap<String, usize> = HashMap::new();
        let mut required = Vec::new();
        let mut readable = Vec::new();
        let mut writable = Vec::new();

        for (idx, desc) in fields.iter().enumerate() {
            name_map.insert(desc.name.to_string(), idx);
            for alias in desc.aliases {
                name_map.insert(alias.to_string(), idx);
            }
            if desc.required {
                required.push(idx);
            }
            if desc.read_fn.is_some() {
                readable.push(idx);
            }
            if desc.write_fn.is_some() {
                writable.push(idx);
            }
        }

        Self {
            fields,
            name_map,
            required,
            readable,
            writable,
        }
    }

    // ── Lookup ────────────────────────────────────────────────────────────────

    /// Look up a descriptor by canonical name or alias.
    ///
    /// Returns `None` if neither the name nor any alias matches.
    #[must_use]
    pub fn get_by_name(&self, name: &str) -> Option<&'static FieldDescriptor<E>> {
        self.name_map.get(name).map(|&i| self.fields[i])
    }

    /// Iterate all descriptors in declaration order.
    pub fn fields(&self) -> impl Iterator<Item = &'static FieldDescriptor<E>> + '_ {
        self.fields.iter().copied()
    }

    // ── Partitions ────────────────────────────────────────────────────────────

    /// Descriptors where `required == true`.
    pub fn required_fields(&self) -> impl Iterator<Item = &'static FieldDescriptor<E>> + '_ {
        self.required.iter().map(|&i| self.fields[i])
    }

    /// Descriptors that have a `read_fn`.
    pub fn readable_fields(&self) -> impl Iterator<Item = &'static FieldDescriptor<E>> + '_ {
        self.readable.iter().map(|&i| self.fields[i])
    }

    /// Descriptors that have a `write_fn`.
    pub fn writable_fields(&self) -> impl Iterator<Item = &'static FieldDescriptor<E>> + '_ {
        self.writable.iter().map(|&i| self.fields[i])
    }

    // ── CRDT fields ───────────────────────────────────────────────────────────

    /// Returns `(name, CrdtFieldType)` pairs for every field whose
    /// `crdt_type` is not [`CrdtFieldType::Derived`].
    ///
    /// Used by the Phase 4 CRDT materialization layer to know which fields
    /// need automerge backing.
    pub fn crdt_fields(&self) -> impl Iterator<Item = (&'static str, CrdtFieldType)> + '_ {
        self.fields.iter().filter_map(|d| {
            if d.crdt_type != CrdtFieldType::Derived {
                Some((d.name, d.crdt_type))
            } else {
                None
            }
        })
    }

    // ── Dispatch helpers ──────────────────────────────────────────────────────

    /// Read a field by name (or alias) from the schedule.
    ///
    /// Returns `Err(FieldError::NotFound)` if no field matches `name`.
    pub fn read_field_value(
        &self,
        name: &str,
        id: crate::entity::EntityId<E>,
        schedule: &Schedule,
    ) -> Result<Option<FieldValue>, FieldError> {
        let desc = self
            .get_by_name(name)
            .ok_or(FieldError::NotFound { name: "field" })?;
        desc.read(id, schedule)
    }

    /// Write a field value by name (or alias) into the schedule.
    ///
    /// Returns `Err(FieldError::NotFound)` if no field matches `name`.
    pub fn write_field_value(
        &self,
        name: &str,
        id: crate::entity::EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        let desc = self
            .get_by_name(name)
            .ok_or(FieldError::NotFound { name: "field" })?;
        desc.write(id, schedule, value)
    }

    // ── Batch writes ──────────────────────────────────────────────────────────

    /// Apply a batch of field writes atomically (from the caller's point of
    /// view), then run [`VerifiableField::verify`] for every descriptor that
    /// has a `verify_fn`.
    ///
    /// # Resolution
    ///
    /// Each [`FieldRef`] is resolved to a `&'static FieldDescriptor<E>`:
    /// - [`FieldRef::Descriptor`] is used directly (zero-cost).
    /// - [`FieldRef::Name`] is looked up in the name/alias map; an unknown
    ///   name returns [`FieldSetError::UnknownField`].
    ///
    /// # De-duplication
    ///
    /// Two references that resolve to the same static descriptor are treated
    /// as duplicates and the batch aborts with
    /// [`FieldSetError::DuplicateField`] before any writes occur.
    ///
    /// # Write phase
    ///
    /// Writes are applied in the order supplied by the caller.  The first
    /// write failure aborts the batch and returns
    /// [`FieldSetError::WriteError`].  **No rollback** is performed; prior
    /// writes remain applied.  Higher-level builders (see `builder.rs`) are
    /// responsible for rolling back at the entity level if required.
    ///
    /// # Verify phase
    ///
    /// After all writes succeed, every resolved descriptor whose `verify_fn`
    /// is `Some(_)` has its [`VerifiableField::verify`] called with the
    /// originally-attempted value.  The first verification failure returns
    /// [`FieldSetError::VerificationError`].  Descriptors with `verify_fn:
    /// None` are not verified here.
    pub fn write_multiple(
        &self,
        id: crate::entity::EntityId<E>,
        schedule: &mut Schedule,
        updates: &[(FieldRef<E>, FieldValue)],
    ) -> Result<(), FieldSetError> {
        // Resolve every FieldRef up front so de-duplication and unknown-name
        // errors fire before any writes happen.
        let mut resolved: Vec<(&'static FieldDescriptor<E>, &FieldValue)> =
            Vec::with_capacity(updates.len());
        for (field_ref, value) in updates {
            let desc = self.resolve(field_ref)?;
            if resolved
                .iter()
                .any(|(prev, _)| std::ptr::eq(*prev as *const _, desc as *const _))
            {
                return Err(FieldSetError::DuplicateField(desc.name));
            }
            resolved.push((desc, value));
        }

        // Write phase — first failure aborts; prior writes stay applied.
        for (desc, value) in &resolved {
            desc.write(id, schedule, (*value).clone())
                .map_err(|error| FieldSetError::WriteError {
                    field: desc.name,
                    error: Box::new(error),
                })?;
        }

        // Verify phase — only descriptors with verify_fn participate.
        for (desc, value) in &resolved {
            if desc.verify_fn.is_some() {
                desc.verify(id, schedule, value).map_err(|error| {
                    FieldSetError::VerificationError {
                        field: desc.name,
                        error: Box::new(error),
                    }
                })?;
            }
        }
        Ok(())
    }

    /// Ergonomic wrapper over [`FieldSet::write_multiple`] that accepts any
    /// [`IntoFieldValue`]-typed value.
    ///
    /// Saves call sites (and the `define_entity_builder!` macro expansion)
    /// from constructing `FieldValue` explicitly.  Internally allocates a
    /// `Vec<(FieldRef<E>, FieldValue)>` and dispatches to
    /// [`FieldSet::write_multiple`].
    pub fn write_many<I, V>(
        &self,
        id: crate::entity::EntityId<E>,
        schedule: &mut Schedule,
        updates: I,
    ) -> Result<(), FieldSetError>
    where
        I: IntoIterator<Item = (FieldRef<E>, V)>,
        V: IntoFieldValue,
    {
        let batch: Vec<(FieldRef<E>, FieldValue)> = updates
            .into_iter()
            .map(|(r, v)| (r, v.into_field_value()))
            .collect();
        self.write_multiple(id, schedule, &batch)
    }

    fn resolve(
        &self,
        field_ref: &FieldRef<E>,
    ) -> Result<&'static FieldDescriptor<E>, FieldSetError> {
        match field_ref {
            FieldRef::Descriptor(d) => Ok(*d),
            FieldRef::Name(name) => self
                .get_by_name(name)
                .ok_or_else(|| FieldSetError::UnknownField((*name).to_string())),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityId, EntityType};
    use crate::field::{ReadFn, WriteFn};
    use crate::field_value;
    use crate::value::{CrdtFieldType, FieldError, ValidationError};
    use crate::value::{FieldCardinality, FieldType, FieldTypeItem};
    use uuid::Uuid;

    // ── Mock entity ──────────────────────────────────────────────────────────

    #[derive(PartialEq, Eq, Hash)]
    struct MockEntity;

    #[derive(Clone, Debug)]
    struct MockData {
        label: String,
        count: i64,
    }

    #[derive(Clone)]
    struct MockExport;

    impl EntityType for MockEntity {
        type InternalData = MockData;
        type Data = MockExport;
        const TYPE_NAME: &'static str = "mock";
        fn uuid_namespace() -> &'static Uuid {
            static NS: std::sync::LazyLock<Uuid> =
                std::sync::LazyLock::new(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"mock"));
            &NS
        }
        fn field_set() -> &'static FieldSet<Self> {
            unimplemented!()
        }
        fn export(_: &Self::InternalData) -> Self::Data {
            MockExport
        }
        fn validate(_: &Self::InternalData) -> Vec<ValidationError> {
            vec![]
        }
    }

    // ── Static field descriptors ─────────────────────────────────────────────

    static LABEL_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "label",
        display: "Label",
        description: "A text label.",
        aliases: &["tag", "name"],
        required: true,
        crdt_type: CrdtFieldType::Scalar,
        field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
        example: "Hello World",
        order: 0,
        read_fn: Some(ReadFn::Bare(|d: &MockData| {
            Some(field_value!(d.label.clone()))
        })),
        write_fn: Some(WriteFn::Bare(|d: &mut MockData, v| {
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
        field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
        example: "7",
        order: 100,
        read_fn: Some(ReadFn::Bare(|d: &MockData| Some(field_value!(d.count)))),
        write_fn: Some(WriteFn::Bare(|d: &mut MockData, v| {
            d.count = v.into_integer()?;
            Ok(())
        })),
        verify_fn: None,
    };

    static DERIVED_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "derived",
        display: "Derived",
        description: "Read-only derived value.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
        example: "42",
        order: 200,
        read_fn: Some(ReadFn::Bare(|_: &MockData| Some(field_value!(42)))),
        write_fn: None,
        verify_fn: None,
    };

    fn make_field_set() -> FieldSet<MockEntity> {
        FieldSet::new(&[&LABEL_FIELD, &COUNT_FIELD, &DERIVED_FIELD])
    }

    fn make_id() -> EntityId<MockEntity> {
        EntityId::new(Uuid::new_v4()).expect("v4 is never nil")
    }

    fn make_schedule_with(id: EntityId<MockEntity>, data: MockData) -> crate::schedule::Schedule {
        let mut sched = crate::schedule::Schedule::default();
        sched.insert(id, data);
        sched
    }

    // ── get_by_name ──────────────────────────────────────────────────────────

    #[test]
    fn test_get_by_name_canonical() {
        let fs = make_field_set();
        assert!(fs.get_by_name("label").is_some());
        assert!(fs.get_by_name("count").is_some());
    }

    #[test]
    fn test_get_by_name_alias() {
        let fs = make_field_set();
        assert!(fs.get_by_name("tag").is_some());
        assert!(fs.get_by_name("name").is_some());
        // alias resolves to the same descriptor as canonical
        assert_eq!(
            fs.get_by_name("tag").unwrap().name,
            fs.get_by_name("label").unwrap().name
        );
    }

    #[test]
    fn test_get_by_name_unknown_returns_none() {
        let fs = make_field_set();
        assert!(fs.get_by_name("nonexistent").is_none());
    }

    // ── Partitions ───────────────────────────────────────────────────────────

    #[test]
    fn test_required_fields() {
        let fs = make_field_set();
        let names: Vec<_> = fs.required_fields().map(|d| d.name).collect();
        assert_eq!(names, vec!["label"]);
    }

    #[test]
    fn test_readable_fields() {
        let fs = make_field_set();
        let names: Vec<_> = fs.readable_fields().map(|d| d.name).collect();
        assert!(names.contains(&"label"));
        assert!(names.contains(&"count"));
        assert!(names.contains(&"derived"));
    }

    #[test]
    fn test_writable_fields() {
        let fs = make_field_set();
        let names: Vec<_> = fs.writable_fields().map(|d| d.name).collect();
        assert!(names.contains(&"label"));
        assert!(names.contains(&"count"));
        // derived is read-only
        assert!(!names.contains(&"derived"));
    }

    #[test]
    fn test_fields_order() {
        let fs = make_field_set();
        let names: Vec<_> = fs.fields().map(|d| d.name).collect();
        assert_eq!(names, vec!["label", "count", "derived"]);
    }

    // ── crdt_fields ──────────────────────────────────────────────────────────

    #[test]
    fn test_crdt_fields_excludes_derived() {
        let fs = make_field_set();
        let crdt: Vec<_> = fs.crdt_fields().collect();
        let names: Vec<_> = crdt.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"label"));
        assert!(names.contains(&"count"));
        assert!(!names.contains(&"derived"));
    }

    #[test]
    fn test_crdt_fields_type() {
        let fs = make_field_set();
        let crdt: Vec<_> = fs.crdt_fields().collect();
        for (name, ct) in &crdt {
            assert_ne!(
                *ct,
                CrdtFieldType::Derived,
                "field {name} should not be Derived"
            );
        }
    }

    // ── read_field_value / write_field_value ─────────────────────────────────

    #[test]
    fn test_read_field_value_by_canonical_name() {
        let fs = make_field_set();
        let id = make_id();
        let sched = make_schedule_with(
            id,
            MockData {
                label: "Hello".into(),
                count: 3,
            },
        );
        let v = fs.read_field_value("label", id, &sched).unwrap();
        assert_eq!(v, Some(field_value!("Hello")));
    }

    #[test]
    fn test_read_field_value_by_alias() {
        let fs = make_field_set();
        let id = make_id();
        let sched = make_schedule_with(
            id,
            MockData {
                label: "World".into(),
                count: 0,
            },
        );
        let v = fs.read_field_value("tag", id, &sched).unwrap();
        assert_eq!(v, Some(field_value!("World")));
    }

    #[test]
    fn test_read_field_value_unknown_name_is_error() {
        let fs = make_field_set();
        let id = make_id();
        let sched = make_schedule_with(
            id,
            MockData {
                label: "x".into(),
                count: 0,
            },
        );
        assert!(matches!(
            fs.read_field_value("nofield", id, &sched),
            Err(FieldError::NotFound { .. })
        ));
    }

    #[test]
    fn test_write_field_value_by_canonical_name() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(
            id,
            MockData {
                label: "old".into(),
                count: 0,
            },
        );
        fs.write_field_value("label", id, &mut sched, field_value!("new"))
            .unwrap();
        assert_eq!(sched.get_internal::<MockEntity>(id).unwrap().label, "new");
    }

    #[test]
    fn test_write_field_value_by_alias() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(
            id,
            MockData {
                label: "old".into(),
                count: 0,
            },
        );
        fs.write_field_value("name", id, &mut sched, field_value!("alias-written"))
            .unwrap();
        assert_eq!(
            sched.get_internal::<MockEntity>(id).unwrap().label,
            "alias-written"
        );
    }

    #[test]
    fn test_write_field_value_unknown_name_is_error() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(
            id,
            MockData {
                label: "x".into(),
                count: 0,
            },
        );
        assert!(matches!(
            fs.write_field_value("nofield", id, &mut sched, field_value!(1)),
            Err(FieldError::NotFound { .. })
        ));
    }

    // ── empty FieldSet ───────────────────────────────────────────────────────

    #[test]
    fn test_empty_field_set() {
        let fs: FieldSet<MockEntity> = FieldSet::new(&[]);
        assert!(fs.get_by_name("anything").is_none());
        assert_eq!(fs.fields().count(), 0);
        assert_eq!(fs.required_fields().count(), 0);
        assert_eq!(fs.crdt_fields().count(), 0);
    }

    // ── write_multiple / write_many ──────────────────────────────────────────

    use crate::field::VerifyFn;
    use crate::value::VerificationError;

    /// A field with `VerifyFn::ReRead` — used to prove the verify phase runs
    /// after writes and catches drift.
    static REREAD_LABEL_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "label_rr",
        display: "Label (ReRead)",
        description: "Mirror of `label` but with ReRead verification enabled.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Scalar,
        field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
        example: "Hello",
        order: 300,
        read_fn: Some(ReadFn::Bare(|d: &MockData| {
            Some(field_value!(d.label.clone()))
        })),
        write_fn: Some(WriteFn::Bare(|d: &mut MockData, v| {
            d.label = v.into_string()?;
            Ok(())
        })),
        verify_fn: Some(VerifyFn::ReRead),
    };

    /// A field whose write *clobbers* `count` to force drift on any verified
    /// field that re-reads `count`.
    static CLOBBER_COUNT_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "clobber_count",
        display: "Clobber Count",
        description: "Ignores its argument and resets `count` to 0.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
        example: "0",
        order: 400,
        read_fn: None,
        write_fn: Some(WriteFn::Bare(|d: &mut MockData, _v| {
            d.count = 0;
            Ok(())
        })),
        verify_fn: None,
    };

    /// A verify-only sibling of `count` that checks the backing field equals
    /// the attempted value via the `Bare` verify variant.
    static VERIFIED_COUNT_FIELD: FieldDescriptor<MockEntity> =
        FieldDescriptor {
            name: "count_verified",
            display: "Count (Verified)",
            description: "Writes and then verifies via Bare verify_fn.",
            aliases: &[],
            required: false,
            crdt_type: CrdtFieldType::Scalar,
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
            example: "7",
            order: 500,
            read_fn: Some(ReadFn::Bare(|d: &MockData| Some(field_value!(d.count)))),
            write_fn: Some(WriteFn::Bare(|d: &mut MockData, v| {
                d.count = v.into_integer()?;
                Ok(())
            })),
            verify_fn: Some(VerifyFn::Bare(|d: &MockData, attempted| {
                let want = attempted.clone().into_integer().map_err(|_| {
                    VerificationError::NotVerifiable {
                        field: "count_verified",
                    }
                })?;
                if d.count == want {
                    Ok(())
                } else {
                    Err(VerificationError::ValueChanged {
                        field: "count_verified",
                        requested: attempted.clone(),
                        actual: field_value!(d.count),
                    })
                }
            })),
        };

    /// Schedule-variant verifier — equivalent check via `VerifyFn::Schedule`.
    static VERIFIED_SCHED_COUNT_FIELD: FieldDescriptor<MockEntity> =
        FieldDescriptor {
            name: "count_sched_verified",
            display: "Count (Schedule Verified)",
            description: "Writes and then verifies via Schedule verify_fn.",
            aliases: &[],
            required: false,
            crdt_type: CrdtFieldType::Scalar,
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
            example: "7",
            order: 600,
            read_fn: Some(ReadFn::Bare(|d: &MockData| Some(field_value!(d.count)))),
            write_fn: Some(WriteFn::Bare(|d: &mut MockData, v| {
                d.count = v.into_integer()?;
                Ok(())
            })),
            verify_fn: Some(VerifyFn::Schedule(|sched, id, attempted| {
                let d = sched.get_internal::<MockEntity>(id).ok_or(
                    VerificationError::NotVerifiable {
                        field: "count_sched_verified",
                    },
                )?;
                let want = attempted.clone().into_integer().map_err(|_| {
                    VerificationError::NotVerifiable {
                        field: "count_sched_verified",
                    }
                })?;
                if d.count == want {
                    Ok(())
                } else {
                    Err(VerificationError::ValueChanged {
                        field: "count_sched_verified",
                        requested: attempted.clone(),
                        actual: field_value!(d.count),
                    })
                }
            })),
        };

    fn make_verify_field_set() -> FieldSet<MockEntity> {
        FieldSet::new(&[
            &LABEL_FIELD,
            &COUNT_FIELD,
            &REREAD_LABEL_FIELD,
            &CLOBBER_COUNT_FIELD,
            &VERIFIED_COUNT_FIELD,
            &VERIFIED_SCHED_COUNT_FIELD,
        ])
    }

    fn initial_data() -> MockData {
        MockData {
            label: "start".into(),
            count: 0,
        }
    }

    #[test]
    fn test_write_multiple_empty_batch_is_ok() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        assert!(fs.write_multiple(id, &mut sched, &[]).is_ok());
    }

    #[test]
    fn test_write_multiple_by_name() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        fs.write_multiple(
            id,
            &mut sched,
            &[
                (FieldRef::Name("label"), field_value!("hi")),
                (FieldRef::Name("count"), field_value!(7_i64)),
            ],
        )
        .unwrap();
        let d = sched.get_internal::<MockEntity>(id).unwrap();
        assert_eq!(d.label, "hi");
        assert_eq!(d.count, 7);
    }

    #[test]
    fn test_write_multiple_by_descriptor() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        fs.write_multiple(
            id,
            &mut sched,
            &[
                (FieldRef::Descriptor(&LABEL_FIELD), field_value!("hi")),
                (FieldRef::Descriptor(&COUNT_FIELD), field_value!(9_i64)),
            ],
        )
        .unwrap();
        let d = sched.get_internal::<MockEntity>(id).unwrap();
        assert_eq!(d.label, "hi");
        assert_eq!(d.count, 9);
    }

    #[test]
    fn test_write_multiple_mixed_name_and_descriptor() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        fs.write_multiple(
            id,
            &mut sched,
            &[
                (FieldRef::Name("label"), field_value!("hi")),
                (FieldRef::Descriptor(&COUNT_FIELD), field_value!(11_i64)),
            ],
        )
        .unwrap();
        let d = sched.get_internal::<MockEntity>(id).unwrap();
        assert_eq!(d.label, "hi");
        assert_eq!(d.count, 11);
    }

    #[test]
    fn test_write_multiple_unknown_name_is_error() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[(FieldRef::Name("nofield"), field_value!("x"))],
            )
            .unwrap_err();
        assert!(matches!(err, FieldSetError::UnknownField(ref s) if s == "nofield"));
        // No writes should have happened.
        let d = sched.get_internal::<MockEntity>(id).unwrap();
        assert_eq!(d.label, "start");
    }

    #[test]
    fn test_write_multiple_duplicate_same_descriptor() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[
                    (FieldRef::Descriptor(&LABEL_FIELD), field_value!("a")),
                    (FieldRef::Descriptor(&LABEL_FIELD), field_value!("b")),
                ],
            )
            .unwrap_err();
        assert!(matches!(err, FieldSetError::DuplicateField("label")));
        // No writes should have happened.
        let d = sched.get_internal::<MockEntity>(id).unwrap();
        assert_eq!(d.label, "start");
    }

    #[test]
    fn test_write_multiple_duplicate_name_and_descriptor() {
        // Same descriptor referenced once by name and once by descriptor.
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[
                    (FieldRef::Name("label"), field_value!("a")),
                    (FieldRef::Descriptor(&LABEL_FIELD), field_value!("b")),
                ],
            )
            .unwrap_err();
        assert!(matches!(err, FieldSetError::DuplicateField("label")));
    }

    #[test]
    fn test_write_multiple_duplicate_name_via_alias() {
        // Same descriptor referenced once by canonical name and once by alias.
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[
                    (FieldRef::Name("label"), field_value!("a")),
                    (FieldRef::Name("tag"), field_value!("b")),
                ],
            )
            .unwrap_err();
        assert!(matches!(err, FieldSetError::DuplicateField("label")));
    }

    #[test]
    fn test_write_multiple_write_error_short_circuits() {
        // Second write supplies the wrong FieldValue variant → ConversionError.
        // The first write must remain applied (documented no-rollback behaviour).
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[
                    (FieldRef::Name("label"), field_value!("applied")),
                    (FieldRef::Name("count"), field_value!("not an integer")),
                ],
            )
            .unwrap_err();
        match err {
            FieldSetError::WriteError { field, .. } => assert_eq!(field, "count"),
            other => panic!("expected WriteError, got {other:?}"),
        }
        let d = sched.get_internal::<MockEntity>(id).unwrap();
        assert_eq!(d.label, "applied"); // earlier write stuck
    }

    #[test]
    fn test_write_multiple_verify_reread_pass() {
        let fs = make_verify_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        fs.write_multiple(
            id,
            &mut sched,
            &[(
                FieldRef::Descriptor(&REREAD_LABEL_FIELD),
                field_value!("after"),
            )],
        )
        .unwrap();
        assert_eq!(sched.get_internal::<MockEntity>(id).unwrap().label, "after");
    }

    #[test]
    fn test_write_multiple_verify_reread_catches_drift() {
        // REREAD_LABEL writes "final"; a later field stomps `label` back.
        // The ReRead verify phase must fire and catch the drift.
        static STOMP_LABEL_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
            name: "stomp_label",
            display: "Stomp Label",
            description: "Forces label = 'stomped'.",
            aliases: &[],
            required: false,
            crdt_type: CrdtFieldType::Derived,
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
            example: "stomped",
            order: 700,
            read_fn: None,
            write_fn: Some(WriteFn::Bare(|d: &mut MockData, _v| {
                d.label = "stomped".into();
                Ok(())
            })),
            verify_fn: None,
        };
        let fs = FieldSet::<MockEntity>::new(&[&REREAD_LABEL_FIELD, &STOMP_LABEL_FIELD]);
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[
                    (
                        FieldRef::Descriptor(&REREAD_LABEL_FIELD),
                        field_value!("final"),
                    ),
                    (
                        FieldRef::Descriptor(&STOMP_LABEL_FIELD),
                        field_value!("ignored"),
                    ),
                ],
            )
            .unwrap_err();
        match err {
            FieldSetError::VerificationError { field, error } => {
                assert_eq!(field, "label_rr");
                assert!(matches!(*error, VerificationError::ValueChanged { .. }));
            }
            other => panic!("expected VerificationError, got {other:?}"),
        }
    }

    #[test]
    fn test_write_multiple_verify_bare_pass() {
        let fs = make_verify_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        fs.write_multiple(
            id,
            &mut sched,
            &[(
                FieldRef::Descriptor(&VERIFIED_COUNT_FIELD),
                field_value!(42_i64),
            )],
        )
        .unwrap();
        assert_eq!(sched.get_internal::<MockEntity>(id).unwrap().count, 42);
    }

    #[test]
    fn test_write_multiple_verify_bare_catches_drift() {
        // Write count_verified=42 then clobber via CLOBBER_COUNT_FIELD.
        let fs = make_verify_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[
                    (
                        FieldRef::Descriptor(&VERIFIED_COUNT_FIELD),
                        field_value!(42_i64),
                    ),
                    (
                        FieldRef::Descriptor(&CLOBBER_COUNT_FIELD),
                        field_value!(0_i64),
                    ),
                ],
            )
            .unwrap_err();
        match err {
            FieldSetError::VerificationError { field, error } => {
                assert_eq!(field, "count_verified");
                assert!(matches!(*error, VerificationError::ValueChanged { .. }));
            }
            other => panic!("expected VerificationError, got {other:?}"),
        }
    }

    #[test]
    fn test_write_multiple_verify_schedule_pass() {
        let fs = make_verify_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        fs.write_multiple(
            id,
            &mut sched,
            &[(
                FieldRef::Descriptor(&VERIFIED_SCHED_COUNT_FIELD),
                field_value!(99_i64),
            )],
        )
        .unwrap();
        assert_eq!(sched.get_internal::<MockEntity>(id).unwrap().count, 99);
    }

    #[test]
    fn test_write_multiple_verify_schedule_catches_drift() {
        let fs = make_verify_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[
                    (
                        FieldRef::Descriptor(&VERIFIED_SCHED_COUNT_FIELD),
                        field_value!(99_i64),
                    ),
                    (
                        FieldRef::Descriptor(&CLOBBER_COUNT_FIELD),
                        field_value!(0_i64),
                    ),
                ],
            )
            .unwrap_err();
        match err {
            FieldSetError::VerificationError { field, error } => {
                assert_eq!(field, "count_sched_verified");
                assert!(matches!(*error, VerificationError::ValueChanged { .. }));
            }
            other => panic!("expected VerificationError, got {other:?}"),
        }
    }

    #[test]
    fn test_write_multiple_verify_skipped_when_verify_fn_none() {
        // LABEL_FIELD has verify_fn: None — even if writes mutate the backing
        // data, no verification error is raised.
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        fs.write_multiple(
            id,
            &mut sched,
            &[(FieldRef::Name("label"), field_value!("anything"))],
        )
        .unwrap();
    }

    // ── write_many (IntoFieldValue wrapper) ──────────────────────────────────

    #[test]
    fn test_write_many_typed_values() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        fs.write_many(
            id,
            &mut sched,
            [
                (FieldRef::Descriptor(&LABEL_FIELD), "typed-str"),
                // Different IntoFieldValue V per tuple isn't allowed by the
                // array literal's homogeneous type requirement; test int via
                // a second call.
            ],
        )
        .unwrap();
        fs.write_many(
            id,
            &mut sched,
            [(FieldRef::Descriptor(&COUNT_FIELD), 13_i64)],
        )
        .unwrap();
        let d = sched.get_internal::<MockEntity>(id).unwrap();
        assert_eq!(d.label, "typed-str");
        assert_eq!(d.count, 13);
    }

    #[test]
    fn test_write_many_propagates_unknown_field() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_many(id, &mut sched, [(FieldRef::Name("nofield"), "x")])
            .unwrap_err();
        assert!(matches!(err, FieldSetError::UnknownField(_)));
    }
}
