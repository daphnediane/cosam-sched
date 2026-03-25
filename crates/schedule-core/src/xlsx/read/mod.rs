/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

pub mod headers;
mod panel_types;
mod people;
mod rooms;
mod schedule;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use umya_spreadsheet::Spreadsheet;
use umya_spreadsheet::structs::Worksheet;

use crate::data::panel::ExtraFields;
use crate::data::panel::ExtraValue;
use crate::data::schedule::{Meta, Schedule};
use crate::data::source_info::ImportedSheetPresence;
use crate::data::time;
use crate::xlsx::columns::FieldDef;

pub use headers::canonical_header;
pub(crate) use headers::{PresenterColumn, PresenterHeader, parse_presenter_header};

pub struct XlsxImportOptions {
    pub title: String,
    pub schedule_table: String,
    pub rooms_table: String,
    pub panel_types_table: String,
    pub use_modified_as_generated: bool,
}

impl Default for XlsxImportOptions {
    fn default() -> Self {
        Self {
            title: "Event Schedule".to_string(),
            schedule_table: "Schedule".to_string(),
            rooms_table: "RoomMap".to_string(),
            panel_types_table: "Prefix".to_string(),
            use_modified_as_generated: false,
        }
    }
}

pub fn import_xlsx(path: &Path, options: &XlsxImportOptions) -> Result<Schedule> {
    let book = umya_spreadsheet::reader::xlsx::read(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;

    let file_path_str = path.display().to_string();

    // Extract Excel metadata
    let properties = book.get_properties();
    let creator = properties.get_creator();
    let last_modified_by = properties.get_last_modified_by();
    let modified_raw = properties.get_modified();

    // Validate and sanitize the modified timestamp
    // Google Sheets exports often contain incorrect timestamps (e.g., 2006-09-16)
    let file_metadata = fs::metadata(path)
        .with_context(|| format!("Failed to get file metadata for {}", path.display()))?;
    let file_modified = file_metadata
        .modified()
        .with_context(|| format!("Failed to get file modified time for {}", path.display()))?;
    let file_modified_datetime: DateTime<Utc> = file_modified.into();

    let modified = if modified_raw.is_empty() {
        None
    } else {
        // Try to parse the timestamp and validate it's reasonable
        if let Ok(parsed) = DateTime::parse_from_rfc3339(modified_raw) {
            let timestamp = parsed.with_timezone(&Utc);

            // Treat any timestamp from 2009 or earlier as suspect (Google Sheets export bug)
            let cutoff_year = DateTime::parse_from_rfc3339("2010-01-01T00:00:00Z").unwrap();

            if timestamp > cutoff_year {
                Some(modified_raw.to_string())
            } else {
                // Timestamp is from 2009 or earlier, use file modified time instead
                Some(time::format_storage_ts(file_modified_datetime))
            }
        } else {
            // Failed to parse, use file modified time instead
            Some(time::format_storage_ts(file_modified_datetime))
        }
    };

    let rooms = rooms::read_rooms(&book, &options.rooms_table, &file_path_str)?;
    let panel_types =
        panel_types::read_panel_types(&book, &options.panel_types_table, &file_path_str)?;
    let presenter_ranks = people::read_presenter_ranks(&book, &file_path_str)?;

    let has_presenters = book.get_sheet_by_name("People").is_some()
        || book.get_sheet_by_name("Presenters").is_some();
    let imported_sheets = ImportedSheetPresence {
        has_room_map: !rooms.is_empty() && rooms.iter().any(|r| r.source.is_some()),
        has_panel_types: !panel_types.is_empty()
            && panel_types.values().any(|pt| pt.source.is_some()),
        has_presenters,
        has_schedule: true, // We'll assume schedule exists if we get here
    };

    let generated = if options.use_modified_as_generated && modified.is_some() {
        modified.clone().unwrap()
    } else {
        time::format_storage_ts(chrono::Utc::now())
    };

    let (panels, presenters, timeline_entries) = schedule::read_panels(
        &book,
        &options.schedule_table,
        &rooms,
        &panel_types,
        &file_path_str,
        &presenter_ranks,
    )?;

    let mut schedule = Schedule {
        conflicts: Vec::new(),
        meta: Meta {
            title: options.title.clone(),
            generated,
            version: Some(7),
            variant: Some("full".to_string()),
            generator: Some(format!("cosam-editor {}", env!("CARGO_PKG_VERSION"))),
            start_time: None,
            end_time: None,
            next_presenter_id: None,
            creator: if creator.is_empty() {
                None
            } else {
                Some(creator.to_string())
            },
            last_modified_by: if last_modified_by.is_empty() {
                None
            } else {
                Some(last_modified_by.to_string())
            },
            modified,
        },
        timeline: timeline_entries,
        panels,
        rooms,
        panel_types,
        presenters,
        imported_sheets,
    };

    crate::data::post_process::apply_schedule_parity(&mut schedule);
    Ok(schedule)
}

// ---------------------------------------------------------------------------
// Shared utilities used by rooms, panel_types, schedule submodules
// ---------------------------------------------------------------------------

/// Describes a contiguous data range in a worksheet (all coordinates are 1-based umya values).
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

/// Search order:
///   1. Named table matching `primary_name` (case-insensitive) on any sheet.
///   2. Sheet whose name matches `primary_name` (case-insensitive).
///   3. Sheets whose names match each entry in `fallback_sheet_names` in order.
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

pub(super) fn get_cell_str(ws: &Worksheet, col: u32, row: u32) -> Option<String> {
    let value = ws.get_value((col, row));
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(super) fn get_cell_number(ws: &Worksheet, col: u32, row: u32) -> Option<f64> {
    ws.get_value_number((col, row))
}

pub(super) fn build_column_map(
    ws: &Worksheet,
    range: &DataRange,
) -> (Vec<String>, Vec<Option<String>>, HashMap<String, u32>) {
    let mut raw_headers = Vec::new();
    let mut canonical_headers = Vec::new();
    let mut col_map: HashMap<String, u32> = HashMap::new();

    for col in range.start_col..=range.end_col {
        let raw = ws.get_value((col, range.header_row));
        let raw = raw.trim().to_string();
        let canon = canonical_header(&raw);
        if let Some(ref key) = canon {
            col_map.entry(key.clone()).or_insert(col);
        }
        raw_headers.push(raw);
        canonical_headers.push(canon);
    }

    (raw_headers, canonical_headers, col_map)
}

pub(super) fn get_field<'a>(
    row_data: &'a HashMap<String, String>,
    keys: &[&str],
) -> Option<&'a String> {
    for key in keys {
        if let Some(val) = row_data.get(*key) {
            return Some(val);
        }
    }
    None
}

/// Look up a value in `row_data` using all keys from a [`FieldDef`] (canonical + aliases).
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

/// Capture raw-header columns that are not covered by `known_fields` as ExtraFields metadata.
/// A column is "known" if its canonical key matches any key in any of the `known_fields`
/// entries (canonical or alias). Uses canonical_header normalization.
pub(super) fn collect_extra_metadata(
    row_data: &HashMap<String, String>,
    raw_headers: &[String],
    known_fields: &[FieldDef],
) -> Option<ExtraFields> {
    use std::collections::HashSet;
    let known_canonical: HashSet<String> = known_fields
        .iter()
        .flat_map(|f| f.keys())
        .filter_map(|k| canonical_header(k))
        .collect();
    let mut meta = ExtraFields::new();
    for raw in raw_headers {
        if raw.is_empty() {
            continue;
        }
        let is_known = canonical_header(raw)
            .map(|c| known_canonical.contains(&c))
            .unwrap_or(true);
        if !is_known {
            if let Some(value) = row_data.get(raw.as_str()) {
                if !value.is_empty() {
                    meta.insert(raw.clone(), ExtraValue::String(value.clone()));
                }
            }
        }
    }
    if meta.is_empty() { None } else { Some(meta) }
}

pub(super) fn is_truthy(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    !lower.is_empty() && lower != "0" && lower != "no" && lower != "false"
}
