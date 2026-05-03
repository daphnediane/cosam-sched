/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field trait hierarchy: [`NamedField`], [`ReadableField<E>`], [`WritableField<E>`],
//! [`AddableField<E>`], and [`RemovableField<E>`]
//!
//! These traits define the core field API used throughout the entity/field system.

use crate::crdt::CrdtFieldType;
use crate::edge::HalfEdge;
use crate::entity::{EntityId, EntityType};
use crate::schedule::Schedule;
use crate::value::{FieldError, FieldValue};

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

    /// Compact `"entity_type:field_name"` key for serialization and registry lookup.
    #[must_use]
    fn field_key(&self) -> String {
        format!("{}:{}", self.entity_type_name(), self.name())
    }

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

// ── AddableField<E> ───────────────────────────────────────────────────────────

/// Field that can accept adding items to a list value.
///
/// Returns `Err(FieldError::ReadOnly)` for fields that don't support add operations.
pub trait AddableField<E: EntityType>: NamedField {
    fn add(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError>;
}

// ── RemovableField<E> ─────────────────────────────────────────────────────────

/// Field that can accept removing items from a list value.
///
/// Returns `Err(FieldError::ReadOnly)` for fields that don't support remove operations.
pub trait RemovableField<E: EntityType>: NamedField {
    fn remove(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError>;
}
