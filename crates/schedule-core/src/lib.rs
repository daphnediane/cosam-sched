/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

// Allow proc-macro generated `::schedule_core::...` paths to resolve from
// within this crate itself.
extern crate self as schedule_core;

// Directory modules
pub mod crdt;
pub mod edge;
pub mod edit;
pub mod entity;
pub mod field;
pub mod query;
pub mod registry;
pub mod schedule;
pub mod tables;
pub mod value;

// Re-export edge types for external use
pub use edge::*;

pub use schedule_macro::{
    accessor_field_properties, callback_field_properties, define_field, edge_field_properties,
};

// Re-export macros from value/macros.rs
// Note: macros are #[macro_export] so they're available at crate root automatically

// Re-exports from entity
pub use entity::{DynamicEntityId, EntityId, EntityTyped, EntityUuid, RuntimeEntityId};
