/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Output formatting helpers.

pub use crate::args::OutputFormat;

/// Format a `FieldValue` as a display string.
///
/// Delegates to `FieldValue`'s `Display` impl.
pub fn format_field_value(v: &schedule_core::value::FieldValue) -> String {
    v.to_string()
}
