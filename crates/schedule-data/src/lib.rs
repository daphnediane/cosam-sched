/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule data structures and field management
//!
//! This crate provides:
//! - Entity definitions for panels, presenters, rooms, etc.
//! - Field system with read/write/validate capabilities
//! - Schedule management and conflict detection

// Re-export the EntityFields proc macro
pub use schedule_macro::EntityFields;

pub mod edge;
pub mod entity;
pub mod field;
pub mod query;
pub mod schedule;
pub mod time;

// Re-export core types for convenience
pub use edge::*;
pub use entity::*;
pub use field::*;
pub use query::*;
pub use schedule::*;
pub use time::*;
