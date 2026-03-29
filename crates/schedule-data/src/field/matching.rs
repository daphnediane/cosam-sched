/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field matching conditions and operations

use super::FieldValue;

/// Field matching conditions
#[derive(Debug, Clone)]
pub enum FieldMatcher {
    Equals(FieldValue),
    NotEquals(FieldValue),
    Contains(String), // For string/text fields
    StartsWith(String),
    EndsWith(String),
    Range(FieldValue, FieldValue), // For numeric/date fields
    In(Vec<FieldValue>),
    NotIn(Vec<FieldValue>),
    IsNull,
    IsNotNull,
}

impl FieldMatcher {
    /// Create an equality matcher
    pub fn equals(value: impl Into<FieldValue>) -> Self {
        Self::Equals(value.into())
    }

    /// Create a not-equals matcher
    pub fn not_equals(value: impl Into<FieldValue>) -> Self {
        Self::NotEquals(value.into())
    }

    /// Create a contains matcher for strings
    pub fn contains(pattern: impl Into<String>) -> Self {
        Self::Contains(pattern.into())
    }

    /// Create a starts-with matcher for strings
    pub fn starts_with(prefix: impl Into<String>) -> Self {
        Self::StartsWith(prefix.into())
    }

    /// Create an ends-with matcher for strings
    pub fn ends_with(suffix: impl Into<String>) -> Self {
        Self::EndsWith(suffix.into())
    }

    /// Create a range matcher
    pub fn range(start: impl Into<FieldValue>, end: impl Into<FieldValue>) -> Self {
        Self::Range(start.into(), end.into())
    }

    /// Create an "in" matcher
    pub fn in_list(values: Vec<FieldValue>) -> Self {
        Self::In(values)
    }

    /// Create a "not in" matcher
    pub fn not_in_list(values: Vec<FieldValue>) -> Self {
        Self::NotIn(values)
    }

    /// Create a null matcher
    pub fn is_null() -> Self {
        Self::IsNull
    }

    /// Create a not-null matcher
    pub fn is_not_null() -> Self {
        Self::IsNotNull
    }
}
