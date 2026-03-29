/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity system with type-safe field operations
//!
//! This module provides:
//! - Generic entity types with compile-time validation
//! - Field-based querying with type-aware matching
//! - Edge-based relationship system

pub mod edge;
pub mod macros;
pub mod panel;
pub mod panel_type;
pub mod presenter;
pub mod room;

// Re-export core entity types for convenience
pub use edge::*;
pub use panel::*;
pub use panel_type::*;
pub use presenter::*;
pub use room::*;

use std::fmt;
use std::hash::Hash;

// Import field types
use crate::field::field_set::FieldSet;
use crate::field::validation::ValidationError;

/// Generic entity identifier
pub type EntityId = String;

/// Core trait for all entity types
pub trait EntityType: 'static + Send + Sync + Sized {
    type Id: Copy + Eq + Hash + Send + Sync + fmt::Debug + fmt::Display;
    type Data: Clone + Send + Sync + fmt::Debug;

    const TYPE_NAME: &'static str;

    fn entity_id(data: &Self::Data) -> Self::Id;
    fn field_set() -> &'static FieldSet<Self>;
    fn validate(data: &Self::Data) -> Result<(), ValidationError>;
}

/// Entity state for soft delete and status tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityState {
    Active,
    Inactive,
    Deleted,
}

impl Default for EntityState {
    fn default() -> Self {
        Self::Active
    }
}
