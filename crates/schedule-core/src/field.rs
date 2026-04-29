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
use crate::field_node_id::FieldRef;
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

/// Metadata common to all field descriptors.
///
/// Provides naming and description information, plus type-erased identity
/// via [`Self::field_id`] and entity type identification via
/// [`Self::entity_type_name`].
///
/// Implemented by [`FieldDescriptor`] and exposed as a trait object for
/// type-erased field lookup.
pub trait NamedField: 'static + Send + Sync + std::any::Any {
    /// Returns the common field data.
    fn common_data(&self) -> &CommonFieldData;

    /// Canonical field name used in programmatic access (snake_case).
    fn name(&self) -> &'static str {
        self.common_data().name
    }

    /// Human-readable display name for UI presentation.
    fn display_name(&self) -> &'static str {
        self.common_data().display
    }

    /// Short description of the field's purpose.
    fn description(&self) -> &'static str {
        self.common_data().description
    }

    /// Alternative names accepted during lookup (e.g. singular/plural forms).
    fn aliases(&self) -> &'static [&'static str] {
        self.common_data().aliases
    }

    /// Logical field type (value type and cardinality).
    fn field_type(&self) -> FieldType {
        self.common_data().field_type
    }

    /// Example value for documentation and UI hints.
    fn example(&self) -> &'static str {
        self.common_data().example
    }

    /// Display/iteration order — lower values sort first.
    fn order(&self) -> u32 {
        self.common_data().order
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

    /// Type-erased identity — the address of the `'static` descriptor singleton.
    ///
    /// Only meaningful when called on a `'static` field descriptor (i.e. one of the
    /// statics declared in each entity module). Returns a [`FieldRef`] wrapper
    /// that can be used as a HashMap key.
    fn field_id(&self) -> FieldRef;

    /// [`crate::entity::EntityType::TYPE_NAME`] for the entity this field belongs to.
    fn entity_type_name(&self) -> &'static str;

    /// CRDT storage type annotation.
    ///
    /// Used to filter fields by role (e.g. `EdgeOwner { .. }` for edge-owning
    /// fields) without requiring a separate inventory.
    fn crdt_type(&self) -> crate::value::CrdtFieldType;
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

// ── TypedField<E> ─────────────────────────────────────────────────────────────

/// Entity-typed field: combines read, write, and verify capabilities.
///
/// A blanket implementation covers any type that implements all three of
/// [`ReadableField<E>`], [`WritableField<E>`], and [`VerifiableField<E>`].
///
/// This trait is used as `dyn TypedField<E>` in [`crate::field_set::FieldSet`]
/// so that all descriptor types — both [`FieldDescriptor<E>`] (non-edge) and
/// [`EdgeDescriptor<E>`] (edge) — can be stored in a single collection.
pub trait TypedField<E: EntityType>:
    ReadableField<E> + WritableField<E> + VerifiableField<E>
{
}

impl<E: EntityType, T: ReadableField<E> + WritableField<E> + VerifiableField<E>> TypedField<E>
    for T
{
}

// ── HalfEdge ─────────────────────────────────────────────────────────────────

/// Marker trait for any field that is one endpoint of a directed edge.
///
/// Only [`EdgeDescriptor<E>`] implements this trait. [`FieldDescriptor<E>`]
/// (scalar, text, list, derived fields) intentionally does **not** implement
/// it, ensuring that non-edge fields cannot be used as [`FieldNodeId`] endpoints.
///
/// [`FieldNodeId`]: crate::field_node_id::FieldNodeId
pub trait HalfEdge: NamedField {
    /// Returns the edge-kind (owner or target) and associated metadata.
    fn edge_kind(&self) -> &crate::value::EdgeKind;

    /// Upcast `self` to `&'static dyn NamedField`.
    ///
    /// Stable Rust does not support automatic trait-object upcasting, so this
    /// explicit method is required.  Implementors transmute `self as &dyn
    /// NamedField` to extend the lifetime to `'static`.
    ///
    /// # Safety (for implementors)
    ///
    /// This is safe only when `self` was originally a `'static` reference (i.e.
    /// one of the `static` field descriptor singletons declared in each entity
    /// module).
    fn as_named_field_static(&self) -> &'static dyn NamedField;
}

// ── TypedHalfEdge<E> ─────────────────────────────────────────────────────────

/// Entity-typed edge half-edge: combines [`HalfEdge`] with [`TypedField<E>`].
///
/// A blanket implementation covers any type that is both a [`HalfEdge`] and a
/// [`TypedField<E>`].  Used as `dyn TypedHalfEdge<E>` in [`FieldNodeId<E>`]
/// to ensure only edge fields can appear as field node ID endpoints.
///
/// [`FieldNodeId<E>`]: crate::field_node_id::FieldNodeId
pub trait TypedHalfEdge<E: EntityType>: HalfEdge + TypedField<E> {}

impl<E: EntityType, T: HalfEdge + TypedField<E>> TypedHalfEdge<E> for T {}

// ── EdgeDescriptor<E> ────────────────────────────────────────────────────────

/// Edge field descriptor — one `static` value per edge field on an entity type.
///
/// Replaces [`FieldDescriptor<E>`] for edge fields (owner and target sides).
/// The [`edge_kind`](Self::edge_kind) field distinguishes ownership and carries
/// the target/source field references and exclusivity information.
///
/// # Example
///
/// ```ignore
/// static FIELD_CREDITED_PRESENTERS: EdgeDescriptor<PanelEntityType> = EdgeDescriptor {
///     name: "credited_presenters",
///     display: "Credited Presenters",
///     description: "Presenters credited on this panel.",
///     aliases: &[],
///     field_type: FieldType(FieldCardinality::List, FieldTypeItem::EntityIdentifier("presenter")),
///     example: "",
///     order: 40,
///     edge_kind: EdgeKind::Owner {
///         target_field: &crate::presenter::FIELD_PANELS,
///         exclusive_with: Some(&FIELD_UNCREDITED_PRESENTERS),
///     },
///     read_fn: Some(ReadFn::Schedule(|sched, id| { … })),
///     write_fn: Some(WriteFn::Schedule(|sched, id, val| { … })),
///     verify_fn: None,
/// };
/// ```
pub struct EdgeDescriptor<E: EntityType> {
    /// Data shared by all field types
    pub(crate) data: CommonFieldData,
    /// Edge ownership and relationship metadata.
    pub edge_kind: crate::value::EdgeKind,
    /// Read implementation. `None` means write-only.
    pub read_fn: Option<ReadFn<E>>,
    /// Write implementation. `None` means read-only.
    pub write_fn: Option<WriteFn<E>>,
    /// Verification implementation. `None` means skip verification.
    pub verify_fn: Option<VerifyFn<E>>,
}

impl<E: EntityType> NamedField for EdgeDescriptor<E> {
    fn common_data(&self) -> &CommonFieldData {
        &self.data
    }

    fn field_id(&self) -> FieldRef {
        // SAFETY: self is a &'static EdgeDescriptor<E> (edge descriptors are static singletons).
        unsafe {
            let static_ref: &'static dyn NamedField = std::mem::transmute(self as &dyn NamedField);
            FieldRef(static_ref)
        }
    }

    fn entity_type_name(&self) -> &'static str {
        E::TYPE_NAME
    }

    fn crdt_type(&self) -> crate::value::CrdtFieldType {
        match &self.edge_kind {
            crate::value::EdgeKind::Owner { target_field, .. } => {
                crate::value::CrdtFieldType::EdgeOwner {
                    target_field: target_field.as_named_field_static(),
                }
            }
            crate::value::EdgeKind::Target { .. } => crate::value::CrdtFieldType::EdgeTarget,
        }
    }
}

impl<E: EntityType> HalfEdge for EdgeDescriptor<E> {
    fn edge_kind(&self) -> &crate::value::EdgeKind {
        &self.edge_kind
    }

    fn as_named_field_static(&self) -> &'static dyn NamedField {
        // SAFETY: EdgeDescriptor instances are 'static singletons.
        unsafe { std::mem::transmute(self as &dyn NamedField) }
    }
}

impl<E: EntityType> ReadableField<E> for EdgeDescriptor<E> {
    fn read(&self, id: EntityId<E>, schedule: &Schedule) -> Result<Option<FieldValue>, FieldError> {
        match &self.read_fn {
            None => Err(FieldError::WriteOnly {
                name: self.data.name,
            }),
            Some(ReadFn::Bare(f)) => Ok(schedule.get_internal::<E>(id).and_then(f)),
            Some(ReadFn::Schedule(f)) => Ok(f(schedule, id)),
        }
    }
}

impl<E: EntityType> WritableField<E> for EdgeDescriptor<E> {
    fn write(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match &self.write_fn {
            None => {
                return Err(FieldError::ReadOnly {
                    name: self.data.name,
                })
            }
            Some(WriteFn::Bare(f)) => {
                let data = schedule
                    .get_internal_mut::<E>(id)
                    .ok_or(FieldError::NotFound {
                        name: self.data.name,
                    })?;
                f(data, value)?;
            }
            Some(WriteFn::Schedule(f)) => f(schedule, id, value)?,
        }
        // Edge fields manage their own CRDT mirroring; no scalar mirror needed.
        Ok(())
    }
}

impl<E: EntityType> VerifiableField<E> for EdgeDescriptor<E> {
    fn verify(
        &self,
        id: EntityId<E>,
        schedule: &Schedule,
        attempted: &FieldValue,
    ) -> Result<(), VerificationError> {
        match &self.verify_fn {
            Some(VerifyFn::Bare(f)) => {
                let data =
                    schedule
                        .get_internal::<E>(id)
                        .ok_or(VerificationError::NotVerifiable {
                            field: self.data.name,
                        })?;
                f(data, attempted)
            }
            Some(VerifyFn::Schedule(f)) => f(schedule, id, attempted),
            Some(VerifyFn::ReRead) => {
                let actual = self
                    .read(id, schedule)
                    .map_err(|_| VerificationError::NotVerifiable {
                        field: self.data.name,
                    })?
                    .ok_or(VerificationError::NotVerifiable {
                        field: self.data.name,
                    })?;
                if actual == *attempted {
                    Ok(())
                } else {
                    Err(VerificationError::ValueChanged {
                        field: self.data.name,
                        requested: attempted.clone(),
                        actual,
                    })
                }
            }
            None => Ok(()),
        }
    }
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
///     field_type: FieldType(FieldCardinality::List, FieldTypeItem::EntityIdentifier("presenter")),
///     read_fn: None,
///     write_fn: Some(WriteFn::Schedule(|schedule, id, v| { todo!() })),
/// };
/// ```
pub struct FieldDescriptor<E: EntityType> {
    /// Data shared by all field types
    pub(crate) data: CommonFieldData,
    /// Whether the field is required (must be non-empty).
    pub required: bool,
    /// CRDT storage type annotation for Phase 4.
    pub crdt_type: CrdtFieldType,
    /// Read implementation. `None` means write-only.
    pub read_fn: Option<ReadFn<E>>,
    /// Write implementation. `None` means read-only.
    pub write_fn: Option<WriteFn<E>>,
    /// Verification implementation. `None` means use automatic read-back if `read_fn` is present.
    pub verify_fn: Option<VerifyFn<E>>,
}

impl<E: EntityType> NamedField for FieldDescriptor<E> {
    fn common_data(&self) -> &CommonFieldData {
        &self.data
    }

    fn field_id(&self) -> FieldRef {
        // SAFETY: self is a &'static FieldDescriptor<E> (field descriptors are static singletons),
        // so its address is stable for the life of the process. We extend the lifetime to 'static.
        unsafe {
            let static_ref: &'static dyn NamedField = std::mem::transmute(self as &dyn NamedField);
            FieldRef(static_ref)
        }
    }

    fn entity_type_name(&self) -> &'static str {
        E::TYPE_NAME
    }

    fn crdt_type(&self) -> crate::value::CrdtFieldType {
        self.crdt_type
    }
}

impl<E: EntityType> ReadableField<E> for FieldDescriptor<E> {
    fn read(&self, id: EntityId<E>, schedule: &Schedule) -> Result<Option<FieldValue>, FieldError> {
        match &self.read_fn {
            None => Err(FieldError::WriteOnly {
                name: self.data.name,
            }),
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
            None => {
                return Err(FieldError::ReadOnly {
                    name: self.data.name,
                })
            }
            Some(WriteFn::Bare(f)) => {
                let data = schedule
                    .get_internal_mut::<E>(id)
                    .ok_or(FieldError::NotFound {
                        name: self.data.name,
                    })?;
                f(data, value)?;
            }
            Some(WriteFn::Schedule(f)) => f(schedule, id, value)?,
        }

        // CRDT mirror: after the inner write succeeds, read the post-write
        // value back through the descriptor's own read_fn and push it into
        // the authoritative automerge document.
        if !schedule.mirror_enabled()
            || matches!(
                self.crdt_type,
                crate::value::CrdtFieldType::Derived
                    | crate::value::CrdtFieldType::EdgeOwner { .. }
                    | crate::value::CrdtFieldType::EdgeTarget
            )
        {
            return Ok(());
        }
        let value_opt = match self.read(id, schedule) {
            Ok(v) => v,
            // Write-only fields are not mirrored back — edge commands mirror
            // their target-list fields themselves in FEATURE-023.
            Err(FieldError::WriteOnly { .. }) => return Ok(()),
            Err(e) => return Err(e),
        };
        schedule.mirror_field_value::<E>(id, self.data.name, self.crdt_type, value_opt.as_ref())
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
                let data =
                    schedule
                        .get_internal::<E>(id)
                        .ok_or(VerificationError::NotVerifiable {
                            field: self.data.name,
                        })?;
                f(data, attempted)
            }
            Some(VerifyFn::Schedule(f)) => f(schedule, id, attempted),
            // Explicit opt-in to read-back verification
            Some(VerifyFn::ReRead) => {
                let actual = self
                    .read(id, schedule)
                    .map_err(|_| VerificationError::NotVerifiable {
                        field: self.data.name,
                    })?
                    .ok_or(VerificationError::NotVerifiable {
                        field: self.data.name,
                    })?;
                if actual == *attempted {
                    Ok(())
                } else {
                    Err(VerificationError::ValueChanged {
                        field: self.data.name,
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

// ── Global field registry ─────────────────────────────────────────────────────

/// Wrapper for globally registering a field descriptor via `inventory`.
///
/// All field descriptors (both macro-generated and hand-written) should submit
/// via `inventory::submit! { CollectedNamedField(&FIELD_NAME) }` to enable
/// `FieldId` round-trip conversions.
pub struct CollectedNamedField(pub &'static dyn NamedField);

inventory::collect!(CollectedNamedField);

/// Iterate over all field descriptors registered via `inventory::submit!`.
///
/// Enables `FieldId` to convert back to trait object references by address lookup.
pub fn all_named_fields() -> impl Iterator<Item = &'static CollectedNamedField> {
    inventory::iter::<CollectedNamedField>()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityId, EntityType};
    use crate::field_value;
    use crate::value::{CrdtFieldType, FieldError, ValidationError};
    use crate::value::{FieldCardinality, FieldType, FieldTypeItem};

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
        required: true,
        crdt_type: CrdtFieldType::Scalar,
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
        data: CommonFieldData {
            name: "count",
            display: "Count",
            description: "An integer count.",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
            example: "7",
            order: 100,
        },
        required: false,
        crdt_type: CrdtFieldType::Scalar,
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
        data: CommonFieldData {
            name: "readonly",
            display: "Read Only",
            description: "Always 42.",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::Integer),
            example: "42",
            order: 200,
        },
        required: false,
        crdt_type: CrdtFieldType::Derived,
        read_fn: Some(ReadFn::Bare(|_: &MockInternalData| Some(field_value!(42)))),
        write_fn: None,
        verify_fn: None,
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
        required: false,
        crdt_type: CrdtFieldType::Derived,
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
