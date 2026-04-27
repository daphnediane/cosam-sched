/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`EdgeDescriptor`] — first-class description of entity relationships.
//!
//! Each edge relationship is declared as a `pub(crate) static` in its canonical
//! CRDT owner entity module (the "panels-outward" rule from FEATURE-023), and
//! self-registers via `inventory::submit!` so no manual registry slice is needed.
//!
//! [`all_edge_descriptors()`] is the single authoritative iterator over all
//! registered relationships.
//!
//! ## Relationship between fields and edges
//!
//! Every edge has two [`crate::field::NamedField`] endpoints:
//!
//! - `owner_field`: the field on the CRDT-canonical owner entity (e.g.
//!   `Panel::FIELD_PRESENTERS`).  `owner_field.name()` is the CRDT list field
//!   name; `owner_field.entity_type_name()` is the owner entity type.
//! - `target_field`: the corresponding field on the non-owner endpoint (e.g.
//!   `Presenter::FIELD_PANELS`).  Its name is the inverse/lookup field name.
//!
//! ## Adding a new edge relationship
//!
//! 1. Declare `pub(crate) static EDGE_<NAME>: EdgeDescriptor = EdgeDescriptor { … }` in the
//!    canonical owner entity module.
//! 2. Add `inventory::submit! { CollectedEdge(&EDGE_<NAME>) }` immediately below.
//!
//! That is the only change required.  The CRDT mirror, load path, and
//! canonical-owner lookup all derive from `all_edge_descriptors()` automatically.

use crate::field::NamedField;
use std::fmt;

// ── Per-edge field metadata (kept until FEATURE-065 removes credited) ─────────

/// Default value for a per-edge field.
///
/// Used when no explicit value has been written for an edge's metadata entry.
/// Removed in FEATURE-065 when the `credited` split replaces per-edge metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeFieldDefault {
    /// A boolean default (e.g. `credited = true` means credited by default).
    Boolean(bool),
}

/// Specification for a single per-edge data field.
///
/// Declared in the `fields` slice of an [`EdgeDescriptor`] for relationships
/// that carry additional per-edge attributes beyond membership.
/// Removed in FEATURE-065.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeFieldSpec {
    /// Name of this per-edge property (e.g. `"credited"`).
    pub name: &'static str,
    /// Value used when no explicit entry has been written for this edge.
    pub default: EdgeFieldDefault,
}

// ── EdgeDescriptor ────────────────────────────────────────────────────────────

/// Describes one entity relationship: its two field endpoints and CRDT semantics.
///
/// Instantiate as a `pub(crate) static` in the canonical owner entity module and
/// register with `inventory::submit! { CollectedEdge(&EDGE_NAME) }`.
///
/// Field-derived accessors (`owner_type()`, `target_type()`, `field_name()`,
/// `is_homogeneous()`) provide backward-compatible access to information that is
/// now embedded in the field descriptors.
pub struct EdgeDescriptor {
    /// Unique human-readable name for this relationship (e.g. `"panel_presenters"`).
    pub name: &'static str,

    /// Field on the CRDT canonical owner side.
    ///
    /// - `owner_field.name()` — CRDT list field name (e.g. `"presenters"`)
    /// - `owner_field.entity_type_name()` — canonical owner entity type name
    pub owner_field: &'static dyn NamedField,

    /// Field on the inverse (non-owner) side.
    ///
    /// - `target_field.name()` — inverse field name (e.g. `"panels"`)
    /// - `target_field.entity_type_name()` — target entity type name
    pub target_field: &'static dyn NamedField,

    /// Per-edge data fields carried by this relationship.
    ///
    /// Empty for pure membership edges.  Non-empty only for `EDGE_PANEL_PRESENTERS`
    /// (`credited` boolean) until FEATURE-065 splits it into separate edge fields.
    pub fields: &'static [EdgeFieldSpec],
}

impl EdgeDescriptor {
    /// [`crate::entity::EntityType::TYPE_NAME`] of the CRDT canonical owner.
    #[inline]
    pub fn owning_type(&self) -> &'static str {
        self.owner_field.entity_type_name()
    }

    /// [`crate::entity::EntityType::TYPE_NAME`] of the non-owner endpoint.
    #[inline]
    pub fn target_type(&self) -> &'static str {
        self.target_field.entity_type_name()
    }

    /// Name of the CRDT list field on the owner entity.
    #[inline]
    pub fn owning_field(&self) -> &'static str {
        self.owner_field.name()
    }

    /// `true` when both sides share the same entity type name.
    ///
    /// Derived from the field descriptor entity type names.  Equivalent to
    /// the removed `is_homogeneous` struct field.
    #[inline]
    pub fn is_homogeneous(&self) -> bool {
        self.owner_field.entity_type_name() == self.target_field.entity_type_name()
    }

    /// Returns the far field given a near field.
    ///
    /// If `near` matches `owner_field`, returns `Some(target_field)`.
    /// If `near` matches `target_field`, returns `Some(owner_field)`.
    /// Otherwise returns `None`.
    ///
    /// Fields are compared by their name and entity type name.
    #[inline]
    pub fn far_field(&self, near: &'static dyn NamedField) -> Option<&'static dyn NamedField> {
        if near.name() == self.owner_field.name()
            && near.entity_type_name() == self.owner_field.entity_type_name()
        {
            Some(self.target_field)
        } else if near.name() == self.target_field.name()
            && near.entity_type_name() == self.target_field.entity_type_name()
        {
            Some(self.owner_field)
        } else {
            None
        }
    }
}

impl fmt::Debug for EdgeDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EdgeDescriptor")
            .field("name", &self.name)
            .field("owner_field", &self.owner_field.name())
            .field("owner_type", &self.owner_field.entity_type_name())
            .field("target_field", &self.target_field.name())
            .field("target_type", &self.target_field.entity_type_name())
            .finish()
    }
}

// ── Inventory ─────────────────────────────────────────────────────────────────

/// Inventory wrapper for [`EdgeDescriptor`] self-registration.
///
/// Each canonical owner entity module emits:
/// ```text
/// inventory::submit! { CollectedEdge(&EDGE_MY_RELATIONSHIP) }
/// ```
/// to register its edges without requiring a central list.
pub struct CollectedEdge(pub &'static EdgeDescriptor);

inventory::collect!(CollectedEdge);

/// Iterate over all registered [`EdgeDescriptor`]s.
///
/// Replaces the removed `ALL_EDGE_DESCRIPTORS` static slice.
pub fn all_edge_descriptors() -> impl Iterator<Item = &'static EdgeDescriptor> {
    inventory::iter::<CollectedEdge>().map(|ce| ce.0)
}
