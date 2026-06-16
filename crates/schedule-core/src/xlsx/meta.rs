/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared constants and header matching for the XLSX metadata sheet.
//!
//! Schedule-level metadata that has no natural home in the entity sheets — the
//! timezone and the schedule-wide start/end window — lives in a small
//! row-oriented table: one header row naming the fields, one data row holding
//! the values. This mirrors the canonical Google-sheet `Timestamp` table so
//! that table is read directly on import, and our export writes the same shape
//! so it round-trips.
//!
//! Recognized columns are matched by a normalized header (case-insensitive,
//! spaces and underscores ignored). Unrecognized columns — notably the legacy
//! `Last Change Added` timestamp, which is no longer authoritative — are
//! ignored, leaving the table open to extra columns.

/// Sheet/table names searched on import and used on export (first = export name).
pub const TABLE_NAMES: &[&str] = &["Meta", "Timestamp"];

/// Header written for the timezone column.
pub const TIMEZONE_HEADER: &str = "Time Zone";
/// Header written for the schedule-window start column.
pub const START_TIME_HEADER: &str = "Start Time";
/// Header written for the schedule-window end column.
pub const END_TIME_HEADER: &str = "End Time";
/// Header for the computed earliest panel start (export-only, ignored on read).
pub const EARLIEST_PANEL_START_HEADER: &str = "Earliest Panel Start";
/// Header for the computed latest panel end (export-only, ignored on read).
pub const LATEST_PANEL_END_HEADER: &str = "Latest Panel End";

/// A metadata field recognizable from a column header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaField {
    Timezone,
    StartTime,
    EndTime,
}

/// Classify a column header into a [`MetaField`], or `None` if unrecognized
/// (e.g. `Last Change Added`).
#[must_use]
pub fn classify_header(header: &str) -> Option<MetaField> {
    let norm: String = header
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .flat_map(char::to_lowercase)
        .collect();
    match norm.as_str() {
        "timezone" | "tz" => Some(MetaField::Timezone),
        "starttime" | "start" => Some(MetaField::StartTime),
        "endtime" | "end" => Some(MetaField::EndTime),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_canonical_headers() {
        assert_eq!(classify_header("Time Zone"), Some(MetaField::Timezone));
        assert_eq!(classify_header("timezone"), Some(MetaField::Timezone));
        assert_eq!(classify_header("Start Time"), Some(MetaField::StartTime));
        assert_eq!(classify_header("End Time"), Some(MetaField::EndTime));
    }

    #[test]
    fn ignores_legacy_and_unknown_headers() {
        assert_eq!(classify_header("Last Change Added"), None);
        assert_eq!(classify_header("Latest Change Added"), None);
        assert_eq!(classify_header("Notes"), None);
        // Export-only computed columns must not be read back as the window.
        assert_eq!(classify_header(EARLIEST_PANEL_START_HEADER), None);
        assert_eq!(classify_header(LATEST_PANEL_END_HEADER), None);
    }
}
