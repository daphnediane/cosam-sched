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

use crate::entity::EntityType;
use crate::field::{FieldDescriptor, IndexableField, MatchPriority, ReadableField, WritableField};
use crate::schedule::Schedule;
use crate::value::{CrdtFieldType, FieldError, FieldValue};
use std::collections::HashMap;

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
/// on the stored key; aliases are lowercased by [`NamedField::aliases`]).
pub struct FieldSet<E: EntityType> {
    /// All descriptors in declaration order.
    fields: Vec<&'static FieldDescriptor<E>>,
    /// Maps every canonical name **and** alias → index into `fields`.
    name_map: HashMap<String, usize>,
    /// Indices of fields where `required == true`.
    required: Vec<usize>,
    /// Indices of fields that have an `index_fn`.
    indexable: Vec<usize>,
    /// Indices of fields that have a `read_fn`.
    readable: Vec<usize>,
    /// Indices of fields that have a `write_fn`.
    writable: Vec<usize>,
}

impl<E: EntityType> FieldSet<E> {
    /// Build a `FieldSet` from a slice of static field descriptor references.
    ///
    /// Registers each descriptor's canonical name and all its aliases into the
    /// internal lookup map.  Later entries with colliding names silently
    /// overwrite earlier ones (prefer no collisions).
    #[must_use]
    pub fn new(descriptors: &[&'static FieldDescriptor<E>]) -> Self {
        let fields: Vec<&'static FieldDescriptor<E>> = descriptors.to_vec();
        let mut name_map: HashMap<String, usize> = HashMap::new();
        let mut required = Vec::new();
        let mut indexable = Vec::new();
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
            if desc.index_fn.is_some() {
                indexable.push(idx);
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
            indexable,
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

    /// Descriptors that have an `index_fn` (support [`IndexableField::match_field`]).
    pub fn indexable_fields(&self) -> impl Iterator<Item = &'static FieldDescriptor<E>> + '_ {
        self.indexable.iter().map(|&i| self.fields[i])
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

    /// Run `match_field` on every indexable field and return the best
    /// [`MatchPriority`] found, or `None` if nothing matched.
    #[must_use]
    pub fn match_index(&self, query: &str, data: &E::InternalData) -> Option<MatchPriority> {
        self.indexable_fields()
            .filter_map(|d| d.match_field(query, data))
            .max()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityId, EntityType};
    use crate::field::{ReadFn, WriteFn};
    use crate::value::{CrdtFieldType, FieldError, FieldValue, ValidationError};
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
        read_fn: Some(ReadFn::Bare(|d: &MockData| {
            Some(FieldValue::String(d.label.clone()))
        })),
        write_fn: Some(WriteFn::Bare(|d: &mut MockData, v| {
            d.label = v.into_string()?;
            Ok(())
        })),
        index_fn: Some(|query, d: &MockData| {
            let q = query.to_lowercase();
            let l = d.label.to_lowercase();
            if l == q {
                Some(MatchPriority::Exact)
            } else if l.starts_with(&q) {
                Some(MatchPriority::Prefix)
            } else if l.contains(&q) {
                Some(MatchPriority::Contains)
            } else {
                None
            }
        }),
    };

    static COUNT_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "count",
        display: "Count",
        description: "An integer count.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Scalar,
        read_fn: Some(ReadFn::Bare(|d: &MockData| {
            Some(FieldValue::Integer(d.count))
        })),
        write_fn: Some(WriteFn::Bare(|d: &mut MockData, v| {
            d.count = v.into_integer()?;
            Ok(())
        })),
        index_fn: None,
    };

    static DERIVED_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "derived",
        display: "Derived",
        description: "Read-only derived value.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        read_fn: Some(ReadFn::Bare(|_: &MockData| Some(FieldValue::Integer(42)))),
        write_fn: None,
        index_fn: None,
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
    fn test_indexable_fields() {
        let fs = make_field_set();
        let names: Vec<_> = fs.indexable_fields().map(|d| d.name).collect();
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
        assert_eq!(v, Some(FieldValue::String("Hello".into())));
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
        assert_eq!(v, Some(FieldValue::String("World".into())));
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
        fs.write_field_value("label", id, &mut sched, FieldValue::String("new".into()))
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
        fs.write_field_value(
            "name",
            id,
            &mut sched,
            FieldValue::String("alias-written".into()),
        )
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
            fs.write_field_value("nofield", id, &mut sched, FieldValue::Integer(1)),
            Err(FieldError::NotFound { .. })
        ));
    }

    // ── match_index ──────────────────────────────────────────────────────────

    #[test]
    fn test_match_index_exact() {
        let fs = make_field_set();
        let data = MockData {
            label: "hello world".into(),
            count: 0,
        };
        assert_eq!(
            fs.match_index("hello world", &data),
            Some(MatchPriority::Exact)
        );
    }

    #[test]
    fn test_match_index_prefix() {
        let fs = make_field_set();
        let data = MockData {
            label: "hello world".into(),
            count: 0,
        };
        assert_eq!(fs.match_index("hello", &data), Some(MatchPriority::Prefix));
    }

    #[test]
    fn test_match_index_no_match() {
        let fs = make_field_set();
        let data = MockData {
            label: "hello world".into(),
            count: 0,
        };
        assert_eq!(fs.match_index("zzz", &data), None);
    }

    #[test]
    fn test_match_index_non_indexable_field_ignored() {
        // count has no index_fn — querying a number should return None
        let fs = make_field_set();
        let data = MockData {
            label: "".into(),
            count: 42,
        };
        assert_eq!(fs.match_index("42", &data), None);
    }

    // ── empty FieldSet ───────────────────────────────────────────────────────

    #[test]
    fn test_empty_field_set() {
        let fs: FieldSet<MockEntity> = FieldSet::new(&[]);
        assert!(fs.get_by_name("anything").is_none());
        assert_eq!(fs.fields().count(), 0);
        assert_eq!(fs.required_fields().count(), 0);
        assert_eq!(fs.indexable_fields().count(), 0);
        assert_eq!(fs.crdt_fields().count(), 0);
    }
}
