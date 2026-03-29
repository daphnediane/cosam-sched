/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Query interface for finding and updating entities

pub mod finder;
pub mod updater;

use crate::entity::EntityState;
use crate::field::FieldMatcher;

// Re-export query types
pub use finder::*;
pub use updater::*;

/// Field match condition for queries
#[derive(Debug, Clone)]
pub struct FieldMatch {
    pub field_name: String,
    pub matcher: FieldMatcher,
}

impl FieldMatch {
    pub fn new(field_name: impl Into<String>, matcher: FieldMatcher) -> Self {
        Self {
            field_name: field_name.into(),
            matcher,
        }
    }
}

/// Query options for filtering and sorting
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    pub state_filter: Option<EntityState>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub order_by: Option<String>,
    pub ascending: bool,
}

impl QueryOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_state(mut self, state: EntityState) -> Self {
        self.state_filter = Some(state);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_order_by(mut self, field: impl Into<String>, ascending: bool) -> Self {
        self.order_by = Some(field.into());
        self.ascending = ascending;
        self
    }
}
