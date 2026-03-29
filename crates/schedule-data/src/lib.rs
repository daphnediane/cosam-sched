/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Generic Schedule Data System
//!
//! This crate provides a type-safe, field-based schedule data system with:
//! - Generic entity types with compile-time validation
//! - Field-based querying with type-aware matching
//! - Edge-based relationship system
//! - TimeRange state machine for safe time handling
//! - Comprehensive validation and conflict detection

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
