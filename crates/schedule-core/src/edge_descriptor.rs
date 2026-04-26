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
//! Every edge has two [`crate::field::FieldDescriptorAny`] endpoints:
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

use crate::field::FieldDescriptorAny;
use crate::field_node_id::FieldId;
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
    pub owner_field: &'static dyn FieldDescriptorAny,

    /// Field on the inverse (non-owner) side.
    ///
    /// - `target_field.name()` — inverse field name (e.g. `"panels"`)
    /// - `target_field.entity_type_name()` — target entity type name
    pub target_field: &'static dyn FieldDescriptorAny,

    /// `true` for transitive (hierarchical) relationships whose reachability
    /// is computed by [`crate::edge_cache::TransitiveEdgeCache`].
    ///
    /// Replaces `is_homogeneous` as the flag that drives transitive-closure
    /// queries.  `is_homogeneous()` now derives from the entity type names.
    pub is_transitive: bool,

    /// Per-edge data fields carried by this relationship.
    ///
    /// Empty for pure membership edges.  Non-empty only for `EDGE_PANEL_PRESENTERS`
    /// (`credited` boolean) until FEATURE-065 splits it into separate edge fields.
    pub fields: &'static [EdgeFieldSpec],
}

impl EdgeDescriptor {
    /// [`crate::entity::EntityType::TYPE_NAME`] of the CRDT canonical owner.
    #[inline]
    pub fn owner_type(&self) -> &'static str {
        self.owner_field.entity_type_name()
    }

    /// [`crate::entity::EntityType::TYPE_NAME`] of the non-owner endpoint.
    #[inline]
    pub fn target_type(&self) -> &'static str {
        self.target_field.entity_type_name()
    }

    /// Name of the CRDT list field on the owner entity.
    #[inline]
    pub fn field_name(&self) -> &'static str {
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
}

impl fmt::Debug for EdgeDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EdgeDescriptor")
            .field("name", &self.name)
            .field("owner_field", &self.owner_field.name())
            .field("owner_type", &self.owner_field.entity_type_name())
            .field("target_field", &self.target_field.name())
            .field("target_type", &self.target_field.entity_type_name())
            .field("is_transitive", &self.is_transitive)
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

// ── Edge field resolution ─────────────────────────────────────────────────────

/// Resolved field IDs and transitive flag for an `(l_type, r_type)` edge pair.
///
/// Returned by [`resolve_edge_fields`] for use in [`crate::edge_map::RawEdgeMap`]
/// operations that need typed field addresses rather than entity type names.
#[derive(Debug, Clone, Copy)]
pub struct EdgeFieldResolution {
    /// [`FieldId`] of the field on the `l_type` entity for this relationship.
    pub l_field_id: FieldId,
    /// [`FieldId`] of the field on the `r_type` entity for this relationship.
    pub r_field_id: FieldId,
    /// `true` when the relationship supports transitive-closure queries
    /// (see [`EdgeDescriptor::is_transitive`]).
    pub is_transitive: bool,
}

/// Resolve field IDs and transitive flag for the edge between `l_type` and `r_type`.
///
/// Searches [`all_edge_descriptors`] for a descriptor whose endpoints match the
/// given pair in either direction.  Returns `None` if no matching edge exists.
///
/// When `l_type == r_type` (homogeneous edge), the `owner` side of the descriptor
/// becomes the L-field and the `target` side becomes the R-field (i.e. the first
/// matching descriptor in iteration order is used — there must be exactly one).
#[must_use]
pub fn resolve_edge_fields(l_type: &str, r_type: &str) -> Option<EdgeFieldResolution> {
    for desc in all_edge_descriptors() {
        if desc.owner_type() == l_type && desc.target_type() == r_type {
            return Some(EdgeFieldResolution {
                l_field_id: desc.owner_field.field_id(),
                r_field_id: desc.target_field.field_id(),
                is_transitive: desc.is_transitive,
            });
        }
        // For heterogeneous edges only: also match the reversed direction.
        // Homogeneous edges (same type on both sides) only match the first branch.
        if !desc.is_homogeneous() && desc.target_type() == l_type && desc.owner_type() == r_type {
            return Some(EdgeFieldResolution {
                l_field_id: desc.target_field.field_id(),
                r_field_id: desc.owner_field.field_id(),
                is_transitive: desc.is_transitive,
            });
        }
    }
    None
}
