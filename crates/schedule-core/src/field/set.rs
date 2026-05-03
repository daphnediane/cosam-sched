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
//! Fields ([`FieldDescriptor<E>`]) and half-edges
//! ([`crate::edge::HalfEdgeDescriptor<E>`]) are stored in separate typed vecs
//! so callers can iterate each category independently.  Name lookup covers both
//! and returns a [`ResolvedRef<E>`] that dispatches read/write/add/remove to
//! the correct concrete type without vtable indirection.
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

use crate::crdt::CrdtFieldType;
use crate::edge::HalfEdgeDescriptor;
use crate::entity::EntityType;
use crate::field::{
    traits::{AddableField, ReadableField, RemovableField, WritableField},
    FieldDescriptor, NamedField,
};
use crate::schedule::Schedule;
use crate::value::{FieldError, FieldValue, IntoFieldValue};
use std::collections::HashMap;
use thiserror::Error;

// ── FieldIndex ────────────────────────────────────────────────────────────────

/// Internal discriminant stored in `FieldSet::name_map`.
#[derive(Clone, Copy)]
enum FieldIndex {
    Field(usize),
    HalfEdge(usize),
}

// ── ResolvedRef<E> ────────────────────────────────────────────────────────────

/// Result of a name or descriptor lookup in a [`FieldSet`].
///
/// Dispatches read/write/add/remove to the concrete descriptor type without
/// requiring trait objects.  Returned by [`FieldSet::get_by_name`] and used
/// internally by [`FieldSet::write_multiple`].
pub enum ResolvedRef<E: EntityType> {
    /// A plain (non-edge) [`FieldDescriptor`].
    Field(&'static FieldDescriptor<E>),
    /// A [`HalfEdgeDescriptor`].
    HalfEdge(&'static HalfEdgeDescriptor<E>),
}

impl<E: EntityType> ResolvedRef<E> {
    /// Canonical field name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Field(d) => d.name(),
            Self::HalfEdge(d) => d.name(),
        }
    }

    /// CRDT storage type annotation.
    pub fn crdt_type(&self) -> crate::crdt::CrdtFieldType {
        match self {
            Self::Field(d) => d.crdt_type(),
            Self::HalfEdge(d) => d.crdt_type(),
        }
    }

    /// Logical field type (value type and cardinality).
    pub fn field_type(&self) -> crate::value::FieldType {
        match self {
            Self::Field(d) => d.field_type(),
            Self::HalfEdge(d) => d.field_type(),
        }
    }

    /// Raw data pointer used for deduplication in [`FieldSet::write_multiple`].
    fn as_ptr(&self) -> *const () {
        match self {
            Self::Field(f) => *f as *const _ as *const (),
            Self::HalfEdge(e) => *e as *const _ as *const (),
        }
    }

    fn read(
        &self,
        id: crate::entity::EntityId<E>,
        schedule: &Schedule,
    ) -> Result<Option<FieldValue>, FieldError> {
        match self {
            Self::Field(d) => d.read(id, schedule),
            Self::HalfEdge(d) => {
                // SAFETY: d is a &'static HalfEdgeDescriptor<E> (edge descriptors are static singletons).
                let static_field: &'static dyn crate::edge::HalfEdge =
                    unsafe { std::mem::transmute(*d as &dyn crate::edge::HalfEdge) };
                crate::schedule::edge::read_edge(schedule, id, static_field)
            }
        }
    }

    fn write(
        &self,
        id: crate::entity::EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match self {
            Self::Field(d) => d.write(id, schedule, value),
            Self::HalfEdge(d) => {
                // SAFETY: d is a &'static HalfEdgeDescriptor<E> (edge descriptors are static singletons).
                let static_field: &'static dyn crate::edge::HalfEdge =
                    unsafe { std::mem::transmute(*d as &dyn crate::edge::HalfEdge) };
                crate::schedule::edge::write_edge(schedule, id, static_field, value)
            }
        }
    }

    fn add(
        &self,
        id: crate::entity::EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match self {
            Self::Field(d) => d.add(id, schedule, value),
            Self::HalfEdge(d) => {
                // SAFETY: d is a &'static HalfEdgeDescriptor<E> (edge descriptors are static singletons).
                let static_field: &'static dyn crate::edge::HalfEdge =
                    unsafe { std::mem::transmute(*d as &dyn crate::edge::HalfEdge) };
                crate::schedule::add_edge(schedule, id, static_field, value)
            }
        }
    }

    fn remove(
        &self,
        id: crate::entity::EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match self {
            Self::Field(d) => d.remove(id, schedule, value),
            Self::HalfEdge(d) => {
                // SAFETY: d is a &'static HalfEdgeDescriptor<E> (edge descriptors are static singletons).
                let static_field: &'static dyn crate::edge::HalfEdge =
                    unsafe { std::mem::transmute(*d as &dyn crate::edge::HalfEdge) };
                crate::schedule::remove_edge(schedule, id, static_field, value)
            }
        }
    }
}

// ── FieldRef<E> ───────────────────────────────────────────────────────────────

/// Reference to a field in a [`FieldSet`] — by canonical name/alias, by
/// concrete [`FieldDescriptor`], or by concrete [`HalfEdgeDescriptor`].
///
/// Used as the first element of each `(FieldRef<E>, FieldValue)` pair in
/// [`FieldSet::write_multiple`].  The `Field` and `HalfEdge` variants are
/// zero-cost; the `Name` variant costs one `HashMap` lookup.
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
    /// `define_entity_builder!` macro for non-edge fields.
    Field(&'static FieldDescriptor<E>),

    /// Direct reference to a static [`HalfEdgeDescriptor`].
    ///
    /// Bypasses the name-map lookup and is the form produced by the
    /// `define_entity_builder!` macro for edge fields.
    HalfEdge(&'static HalfEdgeDescriptor<E>),
}

impl<E: EntityType> From<&'static str> for FieldRef<E> {
    fn from(name: &'static str) -> Self {
        FieldRef::Name(name)
    }
}

impl<E: EntityType> From<&'static FieldDescriptor<E>> for FieldRef<E> {
    fn from(desc: &'static FieldDescriptor<E>) -> Self {
        FieldRef::Field(desc)
    }
}

impl<E: EntityType> From<&'static HalfEdgeDescriptor<E>> for FieldRef<E> {
    fn from(desc: &'static HalfEdgeDescriptor<E>) -> Self {
        FieldRef::HalfEdge(desc)
    }
}

// ── FieldOp / FieldUpdate ─────────────────────────────────────────────────────

/// Operation type for [`FieldUpdate`] — determines which field method is invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldOp {
    /// Set/replace the field value (calls [`WritableField::write`]).
    Set,
    /// Add items to a list field (calls [`AddableField::add`]).
    Add,
    /// Remove items from a list field (calls [`RemovableField::remove`]).
    Remove,
}

/// A single field update operation in a batch.
///
/// Used as the element type in [`FieldSet::write_multiple`] and
/// [`FieldSet::write_many`]. Combines the operation type, field reference,
/// and value into a single struct.
pub struct FieldUpdate<E: EntityType> {
    /// The operation to perform (Set, Add, or Remove).
    pub op: FieldOp,
    /// Reference to the field (by name or descriptor).
    pub field: FieldRef<E>,
    /// The value to write, add, or remove.
    pub value: FieldValue,
}

impl<E: EntityType> FieldUpdate<E> {
    /// Create a new field update with the Set operation.
    pub fn set(field: impl Into<FieldRef<E>>, value: impl IntoFieldValue) -> Self {
        Self {
            op: FieldOp::Set,
            field: field.into(),
            value: value.into_field_value(),
        }
    }

    /// Create a new field update with the Add operation.
    pub fn add(field: impl Into<FieldRef<E>>, value: impl IntoFieldValue) -> Self {
        Self {
            op: FieldOp::Add,
            field: field.into(),
            value: value.into_field_value(),
        }
    }

    /// Create a new field update with the Remove operation.
    pub fn remove(field: impl Into<FieldRef<E>>, value: impl IntoFieldValue) -> Self {
        Self {
            op: FieldOp::Remove,
            field: field.into(),
            value: value.into_field_value(),
        }
    }
}

// ── FieldSetError ─────────────────────────────────────────────────────────────

/// Errors returned by [`FieldSet::write_multiple`] and [`FieldSet::write_many`].
#[derive(Debug, Error)]
pub enum FieldSetError {
    /// A [`FieldRef::Name`] did not resolve to any canonical name or alias in
    /// the [`FieldSet`].
    #[error("unknown field '{0}'")]
    UnknownField(String),

    /// The same descriptor appeared twice in a batch update.  Two references
    /// resolve to the "same" field when they point to the same static
    /// descriptor (pointer equality), regardless of whether they arrived as
    /// `Name`, `Field`, or `HalfEdge` variants.
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
}

// ── FieldSet<E> ───────────────────────────────────────────────────────────────

/// Static registry of all [`FieldDescriptor`]s and [`HalfEdgeDescriptor`]s for
/// an entity type.
///
/// # Construction
///
/// ```ignore
/// static FIELD_SET: LazyLock<FieldSet<MyEntityType>> =
///     LazyLock::new(FieldSet::from_inventory);
/// ```
///
/// # Lookup
///
/// All lookups accept both canonical names and aliases (case-sensitive matching
/// on the stored key; aliases are lowercased by [`crate::field::NamedField::aliases`]).
pub struct FieldSet<E: EntityType> {
    /// Plain (non-edge) descriptors in declaration order.
    fields: Vec<&'static FieldDescriptor<E>>,
    /// Half-edge descriptors in declaration order.
    edges: Vec<&'static HalfEdgeDescriptor<E>>,
    /// Maps every canonical name **and** alias → index discriminant.
    name_map: HashMap<String, FieldIndex>,
    /// Indices into `fields` where `required == true`.
    required: Vec<usize>,
    /// Indices into `fields` that have a `read_fn`.
    readable: Vec<usize>,
    /// Indices into `fields` that have a `write_fn`.
    writable: Vec<usize>,
}

impl<E: EntityType> FieldSet<E> {
    /// Build a `FieldSet` from all descriptors submitted via the global
    /// [`crate::field::CollectedField`] and [`crate::field::CollectedHalfEdge`]
    /// registries.  Both vecs are sorted by `order` (ascending) before the
    /// lookup map is built.
    #[must_use]
    pub fn from_inventory() -> Self {
        use crate::field::{all_fields, all_half_edges};
        use std::any::Any;

        let mut field_descs: Vec<&'static FieldDescriptor<E>> = all_fields()
            .filter(|cf| cf.0.entity_type_name() == E::TYPE_NAME)
            .filter_map(|cf| (cf.0 as &dyn Any).downcast_ref::<FieldDescriptor<E>>())
            .collect();
        field_descs.sort_by_key(|d| d.order());

        let mut edge_descs: Vec<&'static HalfEdgeDescriptor<E>> = all_half_edges()
            .filter(|ce| ce.0.entity_type_name() == E::TYPE_NAME)
            .filter_map(|ce| (ce.0 as &dyn Any).downcast_ref::<HalfEdgeDescriptor<E>>())
            .collect();
        edge_descs.sort_by_key(|d| d.order());

        Self::from_slices(&field_descs, &edge_descs)
    }

    /// Build a `FieldSet` from explicit slices.
    ///
    /// Prefer [`FieldSet::from_inventory`] for production entity types.
    /// This constructor is kept for tests that use mock entities without
    /// an inventory collection.
    #[must_use]
    pub fn new(descriptors: &[&'static FieldDescriptor<E>]) -> Self {
        Self::from_slices(descriptors, &[])
    }

    fn from_slices(
        field_descs: &[&'static FieldDescriptor<E>],
        edge_descs: &[&'static HalfEdgeDescriptor<E>],
    ) -> Self {
        let fields: Vec<&'static FieldDescriptor<E>> = field_descs.to_vec();
        let edges: Vec<&'static HalfEdgeDescriptor<E>> = edge_descs.to_vec();
        let mut name_map: HashMap<String, FieldIndex> = HashMap::new();
        let mut required = Vec::new();
        let mut readable = Vec::new();
        let mut writable = Vec::new();

        for (idx, desc) in fields.iter().enumerate() {
            name_map.insert(desc.name().to_string(), FieldIndex::Field(idx));
            for alias in desc.aliases() {
                name_map.insert(alias.to_string(), FieldIndex::Field(idx));
            }
            if desc.required {
                required.push(idx);
            }
            if desc.cb.read_fn.is_some() {
                readable.push(idx);
            }
            if desc.cb.write_fn.is_some() {
                writable.push(idx);
            }
        }

        for (idx, desc) in edges.iter().enumerate() {
            name_map.insert(desc.name().to_string(), FieldIndex::HalfEdge(idx));
            for alias in desc.aliases() {
                name_map.insert(alias.to_string(), FieldIndex::HalfEdge(idx));
            }
        }

        Self {
            fields,
            edges,
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
    pub fn get_by_name(&self, name: &str) -> Option<ResolvedRef<E>> {
        self.name_map.get(name).map(|&idx| self.resolve_index(idx))
    }

    fn resolve_index(&self, idx: FieldIndex) -> ResolvedRef<E> {
        match idx {
            FieldIndex::Field(i) => ResolvedRef::Field(self.fields[i]),
            FieldIndex::HalfEdge(i) => ResolvedRef::HalfEdge(self.edges[i]),
        }
    }

    // ── Iterators ─────────────────────────────────────────────────────────────

    /// Iterate plain (non-edge) descriptors in declaration order.
    pub fn fields(&self) -> impl Iterator<Item = &'static FieldDescriptor<E>> + '_ {
        self.fields.iter().copied()
    }

    /// Iterate half-edge descriptors in declaration order.
    pub fn half_edges(&self) -> impl Iterator<Item = &'static HalfEdgeDescriptor<E>> + '_ {
        self.edges.iter().copied()
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

    /// Returns `(name, CrdtFieldType)` pairs for every plain field whose
    /// `crdt_type` has automerge backing (`Scalar`, `Text`, or `List`).
    ///
    /// `Derived` fields (including all edge fields) are excluded — edge
    /// list storage is managed by the `edge_crdt` layer.
    pub fn crdt_fields(&self) -> impl Iterator<Item = (&'static str, CrdtFieldType)> + '_ {
        self.fields.iter().filter_map(|d| {
            if matches!(
                d.crdt_type(),
                CrdtFieldType::Scalar | CrdtFieldType::Text | CrdtFieldType::List
            ) {
                Some((d.name(), d.crdt_type()))
            } else {
                None
            }
        })
    }

    // ── Dispatch helpers ──────────────────────────────────────────────────────

    /// Read a field or edge by name (or alias) from the schedule.
    ///
    /// Returns `Err(FieldError::NotFound)` if no field or edge matches `name`.
    pub fn read_field_value(
        &self,
        name: &str,
        id: crate::entity::EntityId<E>,
        schedule: &Schedule,
    ) -> Result<Option<FieldValue>, FieldError> {
        let resolved = self
            .get_by_name(name)
            .ok_or(FieldError::NotFound { name: "field" })?;
        resolved.read(id, schedule)
    }

    /// Write a field or edge value by name (or alias) into the schedule.
    ///
    /// Supports auto-resolution of `add_` and `remove_` prefixes:
    /// - `add_foo` resolves to field `foo` with `FieldOp::Add`
    /// - `remove_foo` resolves to field `foo` with `FieldOp::Remove`
    ///
    /// Returns `Err(FieldError::NotFound)` if no field or edge matches `name`.
    pub fn write_field_value(
        &self,
        name: &'static str,
        id: crate::entity::EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        let update = FieldUpdate {
            op: FieldOp::Set,
            field: FieldRef::Name(name),
            value,
        };
        let (resolved, op) = self
            .resolve(&update.field, update.op)
            .map_err(|_| FieldError::NotFound { name: "field" })?;

        match op {
            FieldOp::Set => resolved.write(id, schedule, update.value),
            FieldOp::Add => resolved.add(id, schedule, update.value),
            FieldOp::Remove => resolved.remove(id, schedule, update.value),
        }
    }

    // ── Batch writes ──────────────────────────────────────────────────────────

    /// Apply a batch of field operations atomically (from the caller's point of
    /// view).
    ///
    /// # Resolution
    ///
    /// Each [`FieldUpdate::field`] ([`FieldRef`]) is resolved to a
    /// [`ResolvedRef<E>`]:
    /// - [`FieldRef::Field`] and [`FieldRef::HalfEdge`] are used directly (zero-cost).
    /// - [`FieldRef::Name`] is looked up in the name/alias map; an unknown
    ///   name returns [`FieldSetError::UnknownField`].
    ///
    /// # De-duplication
    ///
    /// Two Set operations on the same field, or a Set operation after any
    /// prior operation (Set, Add, or Remove), will cause the batch to abort
    /// with [`FieldSetError::DuplicateField`] before any writes occur.
    /// Multiple Add and/or Remove operations on the same field are allowed.
    ///
    /// # Write phase
    ///
    /// Operations are applied in the order supplied by the caller, dispatched
    /// by [`FieldUpdate::op`]:
    /// - [`FieldOp::Set`] → calls [`WritableField::write`]
    /// - [`FieldOp::Add`] → calls [`AddableField::add`]
    /// - [`FieldOp::Remove`] → calls [`RemovableField::remove`]
    ///
    /// The first failure aborts the batch and returns
    /// [`FieldSetError::WriteError`].  **No rollback** is performed; prior
    /// writes remain applied.
    pub fn write_multiple(
        &self,
        id: crate::entity::EntityId<E>,
        schedule: &mut Schedule,
        updates: &[FieldUpdate<E>],
    ) -> Result<(), FieldSetError> {
        let mut resolved: Vec<(ResolvedRef<E>, FieldOp, &FieldValue)> =
            Vec::with_capacity(updates.len());
        for update in updates {
            let (desc, resolved_op) = self.resolve(&update.field, update.op)?;
            if resolved
                .iter()
                .any(|(prev, _, _)| prev.as_ptr() == desc.as_ptr())
                && resolved_op == FieldOp::Set
            {
                return Err(FieldSetError::DuplicateField(desc.name()));
            }
            resolved.push((desc, resolved_op, &update.value));
        }

        for (desc, op, value) in &resolved {
            let result = match op {
                FieldOp::Set => desc.write(id, schedule, (*value).clone()),
                FieldOp::Add => desc.add(id, schedule, (*value).clone()),
                FieldOp::Remove => desc.remove(id, schedule, (*value).clone()),
            };
            result.map_err(|error| FieldSetError::WriteError {
                field: desc.name(),
                error: Box::new(error),
            })?;
        }

        Ok(())
    }

    /// Ergonomic wrapper over [`FieldSet::write_multiple`] that accepts any
    /// [`IntoFieldValue`]-typed value.
    pub fn write_many<I, V>(
        &self,
        id: crate::entity::EntityId<E>,
        schedule: &mut Schedule,
        updates: I,
    ) -> Result<(), FieldSetError>
    where
        I: IntoIterator<Item = (FieldOp, FieldRef<E>, V)>,
        V: IntoFieldValue,
    {
        let batch: Vec<FieldUpdate<E>> = updates
            .into_iter()
            .map(|(op, field, value)| FieldUpdate {
                op,
                field,
                value: value.into_field_value(),
            })
            .collect();
        self.write_multiple(id, schedule, &batch)
    }

    fn resolve(
        &self,
        field_ref: &FieldRef<E>,
        op: FieldOp,
    ) -> Result<(ResolvedRef<E>, FieldOp), FieldSetError> {
        match field_ref {
            FieldRef::Field(d) => Ok((ResolvedRef::Field(*d), op)),
            FieldRef::HalfEdge(d) => Ok((ResolvedRef::HalfEdge(*d), op)),
            FieldRef::Name(name) => {
                if let Some(resolved) = self.get_by_name(name) {
                    return Ok((resolved, op));
                }

                if op == FieldOp::Set || op == FieldOp::Add {
                    if let Some(stripped) = name.strip_prefix("add_") {
                        if let Some(resolved) = self.get_by_name(stripped) {
                            return Ok((resolved, FieldOp::Add));
                        }
                    }
                }

                if op == FieldOp::Set || op == FieldOp::Remove {
                    if let Some(stripped) = name.strip_prefix("remove_") {
                        if let Some(resolved) = self.get_by_name(stripped) {
                            return Ok((resolved, FieldOp::Remove));
                        }
                    }
                }

                Err(FieldSetError::UnknownField((*name).to_string()))
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::CrdtFieldType;
    use crate::edge::EdgeKind;
    use crate::entity::{EntityId, EntityType};
    use crate::field::{CommonFieldData, FieldCallbacks, ReadFn, WriteFn};
    use crate::field_value;
    use crate::value::{FieldCardinality, FieldType, FieldTypeItem};
    use crate::value::{FieldError, ValidationError};
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
            static FS: std::sync::OnceLock<FieldSet<MockEntity>> = std::sync::OnceLock::new();
            FS.get_or_init(|| FieldSet::new(&[&LABEL_FIELD, &COUNT_FIELD, &DERIVED_FIELD]))
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
        data: CommonFieldData {
            name: "label",
            display: "Label",
            description: "A text label.",
            aliases: &["tag", "name"],
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
            crdt_type: CrdtFieldType::Scalar,
            example: "Hello World",
            order: 0,
        },
        required: true,
        edge_kind: EdgeKind::NonEdge,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d: &MockData| {
                Some(field_value!(d.label.clone()))
            })),
            write_fn: Some(WriteFn::Bare(|d: &mut MockData, v| {
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
            crdt_type: CrdtFieldType::Scalar,
            example: "7",
            order: 100,
        },
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d: &MockData| Some(field_value!(d.count)))),
            write_fn: Some(WriteFn::Bare(|d: &mut MockData, v| {
                d.count = v.into_integer()?;
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    };

    static DERIVED_FIELD: FieldDescriptor<MockEntity> = FieldDescriptor {
        data: CommonFieldData {
            name: "derived",
            display: "Derived",
            description: "Read-only derived value.",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
            crdt_type: CrdtFieldType::Derived,
            example: "42",
            order: 200,
        },
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|_: &MockData| Some(field_value!(42)))),
            write_fn: None,
            add_fn: None,
            remove_fn: None,
        },
    };

    fn make_field_set() -> FieldSet<MockEntity> {
        FieldSet::new(&[&LABEL_FIELD, &COUNT_FIELD, &DERIVED_FIELD])
    }

    fn make_id() -> EntityId<MockEntity> {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        unsafe { EntityId::new_unchecked(non_nil_uuid) }
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
        assert_eq!(
            fs.get_by_name("tag").unwrap().name(),
            fs.get_by_name("label").unwrap().name()
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
        let names: Vec<_> = fs.required_fields().map(|d| d.name()).collect();
        assert_eq!(names, vec!["label"]);
    }

    #[test]
    fn test_readable_fields() {
        let fs = make_field_set();
        let names: Vec<_> = fs.readable_fields().map(|d| d.name()).collect();
        assert!(names.contains(&"label"));
        assert!(names.contains(&"count"));
        assert!(names.contains(&"derived"));
    }

    #[test]
    fn test_writable_fields() {
        let fs = make_field_set();
        let names: Vec<_> = fs.writable_fields().map(|d| d.name()).collect();
        assert!(names.contains(&"label"));
        assert!(names.contains(&"count"));
        assert!(!names.contains(&"derived"));
    }

    #[test]
    fn test_fields_order() {
        let fs = make_field_set();
        let names: Vec<_> = fs.fields().map(|d| d.name()).collect();
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
            assert!(
                matches!(
                    ct,
                    CrdtFieldType::Scalar | CrdtFieldType::Text | CrdtFieldType::List
                ),
                "field {name} should only be Scalar, Text, or List; got {ct:?}"
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

    // ── write_multiple / write_many ────────────────────────────────────────

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
                FieldUpdate::set("label", "hi"),
                FieldUpdate::set("count", 7_i64),
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
                FieldUpdate::set(&LABEL_FIELD, "hi"),
                FieldUpdate::set(&COUNT_FIELD, 9_i64),
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
                FieldUpdate::set("label", "hi"),
                FieldUpdate::set(&COUNT_FIELD, 11_i64),
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
            .write_multiple(id, &mut sched, &[FieldUpdate::set("nofield", "x")])
            .unwrap_err();
        assert!(matches!(err, FieldSetError::UnknownField(ref s) if s == "nofield"));
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
                    FieldUpdate::set(&LABEL_FIELD, "a"),
                    FieldUpdate::set(&LABEL_FIELD, "b"),
                ],
            )
            .unwrap_err();
        assert!(matches!(err, FieldSetError::DuplicateField("label")));
        let d = sched.get_internal::<MockEntity>(id).unwrap();
        assert_eq!(d.label, "start");
    }

    #[test]
    fn test_write_multiple_duplicate_name_and_descriptor() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[
                    FieldUpdate::set("label", "a"),
                    FieldUpdate::set(&LABEL_FIELD, "b"),
                ],
            )
            .unwrap_err();
        assert!(matches!(err, FieldSetError::DuplicateField("label")));
    }

    #[test]
    fn test_write_multiple_duplicate_name_via_alias() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[FieldUpdate::set("label", "a"), FieldUpdate::set("tag", "b")],
            )
            .unwrap_err();
        assert!(matches!(err, FieldSetError::DuplicateField("label")));
    }

    #[test]
    fn test_write_multiple_write_error_short_circuits() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        let err = fs
            .write_multiple(
                id,
                &mut sched,
                &[
                    FieldUpdate::set("label", "applied"),
                    FieldUpdate::set("count", "not an integer"),
                ],
            )
            .unwrap_err();
        match err {
            FieldSetError::WriteError { field, .. } => assert_eq!(field, "count"),
            other => panic!("expected WriteError, got {other:?}"),
        }
        let d = sched.get_internal::<MockEntity>(id).unwrap();
        assert_eq!(d.label, "applied");
    }

    // ── write_many ────────────────────────────────────────────────────────────

    #[test]
    fn test_write_many_typed_values() {
        let fs = make_field_set();
        let id = make_id();
        let mut sched = make_schedule_with(id, initial_data());
        fs.write_many(
            id,
            &mut sched,
            [(FieldOp::Set, FieldRef::Field(&LABEL_FIELD), "typed-str")],
        )
        .unwrap();
        fs.write_many(
            id,
            &mut sched,
            [(FieldOp::Set, FieldRef::Field(&COUNT_FIELD), 13_i64)],
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
            .write_many(
                id,
                &mut sched,
                [(FieldOp::Set, FieldRef::Name("nofield"), "x")],
            )
            .unwrap_err();
        assert!(matches!(err, FieldSetError::UnknownField(_)));
    }
}
