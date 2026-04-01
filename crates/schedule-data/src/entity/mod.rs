/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity system with type-safe field operations.
//!
//! Each entity type (Panel, Presenter, Room, PanelType, Edge) is defined as a
//! plain Rust struct that derives [`EntityFields`](crate::EntityFields).  The
//! derive macro generates:
//!
//! - Per-field unit structs implementing [`NamedField`](crate::field::traits::NamedField),
//!   [`SimpleReadableField`](crate::field::traits::SimpleReadableField), and
//!   [`SimpleWritableField`](crate::field::traits::SimpleWritableField)
//! - A separate [`EntityType`] struct (e.g., `PanelEntityType`) with `impl EntityType for PanelEntityType`
//!   where `type Data = Panel`
//! - A static [`FieldSet`](crate::field::field_set::FieldSet) accessible via
//!   `PanelEntityType::field_set()`
//!
//! ## Identifiers
//!
//! [`EntityId`] is a monotonic `u64` used as the internal identifier for all
//! entity instances.  Per-entity typed wrappers (e.g. `PanelId(u64)`) exist
//! for type-safe public APIs but are not used in the generic entity system.
//!
//! ## Re-exports
//!
//! Entity types are re-exported explicitly (not via glob) to avoid name
//! collisions from macro-generated field structs like `NameField`,
//! `IsBreakField`, etc.

pub mod event_room;
pub mod hotel_room;
pub mod panel;
pub mod panel_type;
pub mod presenter;
pub mod presenter_rank;

// Re-export entity types (explicit to avoid ambiguous glob re-exports
// from macro-generated field structs like NameField, IsBreakField, etc.)
pub use event_room::EventRoom;
pub use hotel_room::HotelRoom;
pub use panel::Panel;
pub use panel_type::PanelType;
pub use presenter::Presenter;
pub use presenter_rank::PresenterRank;

// Re-export EntityType structs for clean import paths
pub use event_room::EventRoomEntityType;
pub use hotel_room::HotelRoomEntityType;
pub use panel::PanelEntityType;
pub use panel_type::PanelTypeEntityType;
pub use presenter::PresenterEntityType;

use std::fmt;

use crate::field::field_set::FieldSet;
use crate::field::validation::ValidationError;

/// Generic entity identifier — monotonic u64, never reused
pub type EntityId = u64;

/// Core trait for all entity types
pub trait EntityType: 'static + Send + Sync + fmt::Debug {
    type Data: Clone + Send + Sync + fmt::Debug;

    const TYPE_NAME: &'static str;

    fn field_set() -> &'static FieldSet<Self>
    where
        Self: Sized;
    fn validate(data: &Self::Data) -> Result<(), ValidationError>
    where
        Self: Sized;

    /// Get the type name for this entity type (for dyn compatibility)
    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }
}

/// Internal identifier for an entity instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternalId {
    pub type_name: &'static str,
    pub entity_id: EntityId,
}

impl InternalId {
    pub fn new<T: EntityType>(entity_id: EntityId) -> Self {
        Self {
            type_name: T::TYPE_NAME,
            entity_id,
        }
    }
}

/// Entity state for soft delete and status tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityState {
    Active,
    Inactive,
}

impl Default for EntityState {
    fn default() -> Self {
        Self::Active
    }
}
