/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! XLSX import: shared utilities and the top-level [`import_xlsx`] entry point.

pub mod headers;
mod panel_types;
mod rooms;
mod schedule;

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use umya_spreadsheet::structs::Worksheet;
use umya_spreadsheet::Spreadsheet;

use crate::schedule::Schedule;
use crate::xlsx::columns::FieldDef;

pub use headers::canonical_header;
pub(crate) use headers::{parse_presenter_header, PresenterColumn, PresenterHeader};

// ── Import options ────────────────────────────────────────────────────────────

/// Options controlling which sheets are read during XLSX import.
pub struct XlsxImportOptions {
    /// Preferred sheet/table name for panel data (default: `"Schedule"`).
    pub schedule_table: String,
    /// Preferred sheet/table name for rooms (default: `"Rooms"`).
    pub rooms_table: String,
    /// Preferred sheet/table name for panel types (default: `"PanelTypes"`).
    pub panel_types_table: String,
}

impl Default for XlsxImportOptions {
    fn default() -> Self {
        Self {
            schedule_table: "Schedule".to_string(),
            rooms_table: "Rooms".to_string(),
            panel_types_table: "PanelTypes".to_string(),
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Import an XLSX spreadsheet and return a populated [`Schedule`].
///
/// Reads the PanelTypes sheet first (so panel-type lookups work during
/// schedule import), then the Rooms sheet, then the Schedule sheet.
///
/// The returned `Schedule` is a clean slate — all entities and edges are
/// freshly created.  No existing CRDT state is preserved or merged.
/// See IDEA-079 for future merge-import support.
pub fn import_xlsx(path: &Path, options: &XlsxImportOptions) -> Result<Schedule> {
    let book = umya_spreadsheet::reader::xlsx::read(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;

    let mut schedule = Schedule::new();

    let panel_type_lookup =
        panel_types::read_panel_types_into(&book, &options.panel_types_table, &mut schedule)?;
    let room_lookup = rooms::read_rooms_into(&book, &options.rooms_table, &mut schedule)?;

    schedule::read_schedule_into(
        &book,
        &options.schedule_table,
        &mut schedule,
        &room_lookup,
        &panel_type_lookup,
    )?;

    Ok(schedule)
}

// ── Shared data-range utilities ───────────────────────────────────────────────

/// A contiguous data range in a worksheet (all coordinates are 1-based umya values).
/// `header_row` holds the column headers; data rows start at `header_row + 1`.
pub(super) struct DataRange {
    pub(super) sheet_name: String,
    pub(super) start_col: u32,
    pub(super) header_row: u32,
    pub(super) end_col: u32,
    pub(super) end_row: u32,
}

impl DataRange {
    pub(super) fn has_data(&self) -> bool {
        self.end_row > self.header_row && self.end_col >= self.start_col
    }
}

/// Find a named table or sheet by name.
///
/// Search order:
/// 1. Named table matching `primary_name` (case-insensitive) on any sheet.
/// 2. Sheet whose name matches `primary_name` (case-insensitive).
/// 3. Sheets matching each entry in `fallback_sheet_names` in order.
pub(super) fn find_data_range(
    book: &Spreadsheet,
    primary_name: &str,
    fallback_sheet_names: &[&str],
) -> Option<DataRange> {
    let primary_lower = primary_name.to_lowercase();

    for sheet in book.get_sheet_collection() {
        for table in sheet.get_tables() {
            if table.get_name().to_lowercase() == primary_lower {
                let (start, end) = table.get_area();
                return Some(DataRange {
                    sheet_name: sheet.get_name().to_string(),
                    start_col: *start.get_col_num(),
                    header_row: *start.get_row_num(),
                    end_col: *end.get_col_num(),
                    end_row: *end.get_row_num(),
                });
            }
        }
    }

    let mut all_names: Vec<&str> = vec![primary_name];
    all_names.extend_from_slice(fallback_sheet_names);
    for name in all_names {
        let lower = name.to_lowercase();
        if let Some(sheet) = book
            .get_sheet_collection()
            .iter()
            .find(|s| s.get_name().to_lowercase() == lower)
        {
            let end_col = sheet.get_highest_column();
            let end_row = sheet.get_highest_row();
            if end_row >= 2 && end_col >= 1 {
                return Some(DataRange {
                    sheet_name: sheet.get_name().to_string(),
                    start_col: 1,
                    header_row: 1,
                    end_col,
                    end_row,
                });
            }
        }
    }

    None
}

/// Return the trimmed string value of a cell, or `None` if empty.
pub(super) fn get_cell_str(ws: &Worksheet, col: u32, row: u32) -> Option<String> {
    let value = ws.get_value((col, row));
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Return the numeric value of a cell if it has one.
pub(super) fn get_cell_number(ws: &Worksheet, col: u32, row: u32) -> Option<f64> {
    ws.get_value_number((col, row))
}

/// Build header maps for a data range.
///
/// Returns `(raw_headers, canonical_headers, col_map)`:
/// - `raw_headers`: raw strings in column order (first occurrence wins in `col_map`).
/// - `canonical_headers`: `canonical_header()` of each raw string, `None` for blanks.
/// - `col_map`: canonical key → 1-based column index.
pub(super) fn build_column_map(
    ws: &Worksheet,
    range: &DataRange,
) -> (Vec<String>, Vec<Option<String>>, HashMap<String, u32>) {
    let mut raw_headers = Vec::new();
    let mut canonical_headers = Vec::new();
    let mut col_map: HashMap<String, u32> = HashMap::new();

    for col in range.start_col..=range.end_col {
        let raw = ws.get_value((col, range.header_row)).trim().to_string();
        let canon = canonical_header(&raw);
        if let Some(ref key) = canon {
            col_map.entry(key.clone()).or_insert(col);
        }
        raw_headers.push(raw);
        canonical_headers.push(canon);
    }

    (raw_headers, canonical_headers, col_map)
}

/// Convert one data row to a map keyed by both raw header and canonical header.
pub(super) fn row_to_map(
    ws: &Worksheet,
    row: u32,
    range: &DataRange,
    raw_headers: &[String],
    canonical_headers: &[Option<String>],
) -> HashMap<String, String> {
    let mut data = HashMap::new();
    for (i, col) in (range.start_col..=range.end_col).enumerate() {
        if let Some(value) = get_cell_str(ws, col, row) {
            if !raw_headers[i].is_empty() {
                data.insert(raw_headers[i].clone(), value.clone());
            }
            if let Some(ref key) = canonical_headers[i] {
                data.entry(key.clone()).or_insert(value);
            }
        }
    }
    data
}

/// Look up the first non-`None` key from a [`FieldDef`]'s key set in `row_data`.
pub(super) fn get_field_def<'a>(
    row_data: &'a HashMap<String, String>,
    field: &FieldDef,
) -> Option<&'a String> {
    for key in field.keys() {
        if let Some(val) = row_data.get(key) {
            return Some(val);
        }
    }
    None
}

/// Return `true` for any non-blank, non-falsy string value.
pub(super) fn is_truthy(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    !lower.is_empty() && lower != "0" && lower != "no" && lower != "false"
}
