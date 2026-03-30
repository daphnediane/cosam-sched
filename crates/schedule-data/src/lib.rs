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

pub mod entity;
pub mod field;
pub mod query;
pub mod schedule;
pub mod time;

// Re-export core types for convenience
pub use entity::*;
pub use field::*;
pub use query::*;
pub use schedule::*;
pub use time::*;

/// Simple hash function for ID generation
pub fn simple_hash(s: &str) -> u64 {
    let mut hash = 0u64;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}
