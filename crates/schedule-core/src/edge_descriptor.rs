/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`EdgeDescriptor`] — first-class description of entity relationships.
//!
//! Each edge relationship is declared as a `const` on its canonical CRDT
//! owner entity type (the "panels-outward" rule from FEATURE-023), mirroring
//! how [`crate::field::FieldDescriptor`] is declared per field.
//!
//! [`ALL_EDGE_DESCRIPTORS`] is the single authoritative registry.  All code
//! that previously iterated [`crate::edge_crdt::OWNER_EDGE_FIELDS`] or
//! matched on [`crate::edge_crdt::canonical_owner`] now derives from this
//! slice instead.
//!
//! ## Adding a new edge relationship
//!
//! 1. Declare `pub const EDGE_<NAME>: EdgeDescriptor = EdgeDescriptor { … }` on the
//!    canonical owner entity type.
//! 2. Add `&OwnerType::EDGE_<NAME>` to [`ALL_EDGE_DESCRIPTORS`] here.
//!
//! That is the only change required.  The CRDT mirror, load path, and
//! `canonical_owner` lookup all derive from this registry automatically.

/// Default value for a per-edge field.
///
/// Used when no explicit value has been written for an edge's metadata entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeFieldDefault {
    /// A boolean default (e.g. `credited = true` means credited by default).
    Boolean(bool),
}

/// Specification for a single per-edge data field.
///
/// Declared in the `fields` slice of an [`EdgeDescriptor`] for relationships
/// that carry additional per-edge attributes beyond membership.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeFieldSpec {
    /// Name of this per-edge property (e.g. `"credited"`).
    pub name: &'static str,
    /// Value used when no explicit entry has been written for this edge.
    pub default: EdgeFieldDefault,
}

/// Describes one entity relationship: CRDT ownership, target type, and CRDT
/// field name on the owner.
///
/// Instantiate as a `pub const` on the canonical owner entity type and register
/// it in [`ALL_EDGE_DESCRIPTORS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeDescriptor {
    /// Unique human-readable name for this relationship
    /// (e.g. `"panel_presenters"`).
    pub name: &'static str,

    /// [`crate::entity::EntityType::TYPE_NAME`] of the CRDT canonical owner.
    ///
    /// Follows the panels-outward rule: `Panel` owns most relationships;
    /// `EventRoom` owns hotel-room relationships; the source `Presenter` owns
    /// the groups edge.
    pub owner_type: &'static str,

    /// [`crate::entity::EntityType::TYPE_NAME`] of the non-owner endpoint.
    pub target_type: &'static str,

    /// `true` when both sides share the same `TYPE_NAME` (homogeneous edge,
    /// e.g. `Presenter ↔ Presenter` groups).
    pub is_homogeneous: bool,

    /// Name of the CRDT list field on the owner entity (e.g. `"presenters"`).
    pub field_name: &'static str,

    /// Per-edge data fields carried by this relationship.
    ///
    /// Empty for pure membership edges. When non-empty, each entry describes
    /// one per-edge attribute stored in the parallel `{field_name}_meta` map
    /// in the CRDT document.
    pub fields: &'static [EdgeFieldSpec],
}

// ── Registry ──────────────────────────────────────────────────────────────────

use crate::event_room::EventRoomEntityType;
use crate::panel::PanelEntityType;
use crate::presenter::PresenterEntityType;

/// All recognized edge relationships, in canonical load order.
///
/// This is the single source of truth that replaces both
/// `canonical_owner()` and `OWNER_EDGE_FIELDS` in [`crate::edge_crdt`].
/// Add a new relationship here *and* declare its [`EdgeDescriptor`] const on
/// the canonical owner type — those are the only two edits required.
pub const ALL_EDGE_DESCRIPTORS: &[&EdgeDescriptor] = &[
    &PanelEntityType::EDGE_PRESENTERS,
    &PanelEntityType::EDGE_EVENT_ROOMS,
    &PanelEntityType::EDGE_PANEL_TYPE,
    &EventRoomEntityType::EDGE_HOTEL_ROOMS,
    &PresenterEntityType::EDGE_GROUPS,
];
