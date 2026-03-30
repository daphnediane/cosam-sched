/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity module with field definitions

pub mod edge;
pub mod panel;
pub mod panel_type;
pub mod presenter;
pub mod room;

// Re-export entity types (explicit to avoid ambiguous glob re-exports
// from macro-generated field structs like NameField, IsBreakField, etc.)
pub use edge::{Edge, EdgeType};
pub use panel::Panel;
pub use panel_type::PanelType;
pub use presenter::Presenter;
pub use room::Room;

use std::fmt;

use crate::field::field_set::FieldSet;
use crate::field::validation::ValidationError;

/// Generic entity identifier — monotonic u64, never reused
pub type EntityId = u64;

/// Core trait for all entity types
pub trait EntityType: 'static + Send + Sync + Sized {
    type Data: Clone + Send + Sync + fmt::Debug;

    const TYPE_NAME: &'static str;

    fn field_set() -> &'static FieldSet<Self>;
    fn validate(data: &Self::Data) -> Result<(), ValidationError>;
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
