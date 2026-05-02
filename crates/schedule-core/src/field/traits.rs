/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field trait hierarchy: [`NamedField`], [`ReadableField<E>`], [`WritableField<E>`],
//! [`VerifiableField<E>`], and [`TypedField<E>`].
//!
//! These traits define the core field API used throughout the entity/field system.

use crate::crdt::CrdtFieldType;
use crate::edge::HalfEdge;
use crate::entity::{EntityId, EntityType};
use crate::schedule::Schedule;
use crate::value::{FieldError, FieldValue, VerificationError};

// ── NamedField ────────────────────────────────────────────────────────────────

/// Metadata common to all field descriptors.
///
/// Provides naming and description information, and entity
/// type identification via [`Self::entity_type_name`].
///
/// Implemented by [`FieldDescriptor`] and exposed as a trait object for
/// type-erased field lookup.
///
/// [`FieldDescriptor`]: crate::field::FieldDescriptor
pub trait NamedField: 'static + Send + Sync + std::any::Any {
    /// Returns the common field data.
    fn common_data(&self) -> &super::CommonFieldData;

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
    fn field_type(&self) -> crate::value::FieldType {
        self.common_data().field_type
    }

    /// CRDT storage type annotation.
    fn crdt_type(&self) -> CrdtFieldType {
        self.common_data().crdt_type
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

    /// [`crate::entity::EntityType::TYPE_NAME`] for the entity this field belongs to.
    fn entity_type_name(&self) -> &'static str;

    /// Upcast `self` to `Option<&dyn HalfEdge>`.
    fn try_as_half_edge(&self) -> Option<&dyn HalfEdge>;
}

// ── ReadableField<E> ───────────────────────────────────────────────────────────

/// Field that can produce a [`FieldValue`] given an entity ID and schedule.
///
/// Returns `Err(FieldError::WriteOnly)` for write-only fields.
pub trait ReadableField<E: EntityType>: NamedField {
    fn read(&self, id: EntityId<E>, schedule: &Schedule) -> Result<Option<FieldValue>, FieldError>;
}

// ── WritableField<E> ───────────────────────────────────────────────────────────

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

// ── VerifiableField<E> ─────────────────────────────────────────────────────────

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
/// [`crate::edge::EdgeDescriptor<E>`] (edge) — can be stored in a single collection.
///
/// [`FieldDescriptor<E>`]: crate::field::FieldDescriptor
pub trait TypedField<E: EntityType>:
    ReadableField<E> + WritableField<E> + VerifiableField<E>
{
}

impl<E: EntityType, T: ReadableField<E> + WritableField<E> + VerifiableField<E>> TypedField<E>
    for T
{
}
