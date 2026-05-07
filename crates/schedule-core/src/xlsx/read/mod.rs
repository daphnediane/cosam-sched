/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! XLSX import: shared utilities and the top-level [`import_xlsx`] entry point.

pub mod headers;
mod panel_types;
mod people;
mod rooms;
mod schedule;

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use umya_spreadsheet::structs::Worksheet;
use umya_spreadsheet::Spreadsheet;

use crate::entity::{EntityType, EntityUuid};
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::sidecar::SidecarFormulaField;
use crate::tables::presenter::{self, PresenterEntityType};
use crate::xlsx::columns::{FieldDef, FormulaColumnDef};

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
    /// Preferred sheet/table name for the People/Presenters sheet (default: `"People"`).
    /// Set to an empty string to skip People sheet processing.
    pub people_table: String,
}

impl Default for XlsxImportOptions {
    fn default() -> Self {
        Self {
            schedule_table: "Schedule".to_string(),
            rooms_table: "Rooms".to_string(),
            panel_types_table: "PanelTypes".to_string(),
            people_table: "People".to_string(),
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Import an XLSX spreadsheet and return a populated [`Schedule`].
///
/// Read order:
/// 1. PanelTypes — so panel-type lookups work during schedule import.
/// 2. Rooms — so room lookups work during schedule import.
/// 3. People — establishes presenter rank/flags before the Schedule sheet
///    creates presenter entities from column headers.
/// 4. Schedule — panels, timing, rooms, panel type, and presenter edges.
///
/// The returned `Schedule` is a clean slate — all entities and edges are
/// freshly created.  No existing CRDT state is preserved or merged.
/// See IDEA-080 for future merge-import support.
pub fn import_xlsx(path: &Path, options: &XlsxImportOptions) -> Result<Schedule> {
    let book = umya_spreadsheet::reader::xlsx::read(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;

    let mut schedule = Schedule::new();
    schedule.metadata.modified_at = resolve_source_modified(&book, path);

    let file_path = path.to_str().map(str::to_owned);
    let import_time = chrono::Utc::now();

    let panel_type_lookup = panel_types::read_panel_types_into(
        &book,
        &options.panel_types_table,
        &mut schedule,
        file_path.as_deref(),
        import_time,
    )?;
    let room_lookup = rooms::read_rooms_into(
        &book,
        &options.rooms_table,
        &mut schedule,
        file_path.as_deref(),
        import_time,
    )?;

    if !options.people_table.is_empty() {
        people::read_people_into(
            &book,
            &options.people_table,
            &mut schedule,
            file_path.as_deref(),
            import_time,
        )?;
    }

    schedule::read_schedule_into(
        &book,
        &options.schedule_table,
        &mut schedule,
        &room_lookup,
        &panel_type_lookup,
        file_path.as_deref(),
        import_time,
    )?;

    normalize_presenter_sort_indices(&mut schedule);

    Ok(schedule)
}

// ── Presenter sort normalization ──────────────────────────────────────────────

/// After all sheets are imported, assign monotonically increasing `sort_index`
/// values (multiples of 100) to each presenter based on their sidecar
/// `xlsx_sort_key` (column, row).
///
/// Sort order: People-sheet entries (col=0) first in row order, then
/// schedule-sheet entries by (col, row). Presenters with no sidecar key are
/// appended last. Gaps of 100 allow future manual insertions.
fn normalize_presenter_sort_indices(schedule: &mut Schedule) {
    // Collect (uuid, sort_key) for all presenters.
    let mut keyed: Vec<(uuid::NonNilUuid, Option<(u32, u32)>)> = schedule
        .iter_entities::<PresenterEntityType>()
        .map(|(id, _)| {
            let uuid = id.entity_uuid();
            let key = schedule.sidecar().get(uuid).and_then(|e| e.xlsx_sort_key);
            (uuid, key)
        })
        .collect();

    // Sort: known keys first (People col=0 before schedule cols), None last.
    keyed.sort_by(|(_, a), (_, b)| match (a, b) {
        (Some(ka), Some(kb)) => ka.cmp(kb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    // Assign sort_index = (rank + 1) * 100.
    // Collect (uuid, idx) pairs first, then apply in a separate pass to avoid
    // borrowing `schedule` while iterating over it.
    let assignments: Vec<(uuid::NonNilUuid, i64)> = keyed
        .iter()
        .enumerate()
        .map(|(rank, (uuid, _))| (*uuid, (rank as i64 + 1) * 100))
        .collect();

    for (uuid, idx) in assignments {
        // SAFETY: uuid came from iter_entities, so the entity exists.
        let id = unsafe { crate::entity::EntityId::<PresenterEntityType>::new_unchecked(uuid) };
        let update = FieldUpdate::set(&presenter::FIELD_SORT_INDEX, idx);
        let _ = PresenterEntityType::field_set().write_multiple(id, schedule, &[update]);
    }
}

// ── Modified-time resolution ──────────────────────────────────────────────────

/// Resolve the best "last modified" timestamp for the imported spreadsheet.
///
/// Precedence:
/// 1. `dcterms:modified` from `docProps/core.xml` inside the XLSX, if the
///    value parses as RFC 3339 and is after 2010 (earlier values are a known
///    Google Sheets export bug where it writes `2006-09-16`).
/// 2. File-system mtime of `path`.
/// 3. `None` if neither is available.
fn resolve_source_modified(book: &Spreadsheet, path: &Path) -> Option<DateTime<Utc>> {
    // Cutoff: reject xlsx-internal timestamps from 2010 or earlier as suspect.
    let cutoff: DateTime<Utc> = "2010-01-01T00:00:00Z".parse().ok()?;

    let props_raw = book.get_properties().get_modified();
    if !props_raw.is_empty() {
        if let Ok(ts) = DateTime::parse_from_rfc3339(props_raw) {
            let ts_utc = ts.with_timezone(&Utc);
            if ts_utc > cutoff {
                return Some(ts_utc);
            }
        }
    }

    // Fall back to file-system mtime.
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from)
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

// ── Extra-column routing ──────────────────────────────────────────────────────

/// Route all columns not in `known_field_keys` for a given row to either the
/// CRDT `__extra` map or the sidecar `formula_extras`, following the priority:
///
/// 1. Explicit ignore (e.g. `OLD_UNIQ_ID`) — skipped before this call.
/// 2. [`FormulaColumnDef`] list → sidecar `formula_extras`.
/// 3. [`crate::extra_field::ExtraFieldDescriptor`] registry → CRDT `__extra`.
/// 4. Unknown: formula cell → sidecar; plain value → CRDT `__extra`.
///
/// `known_field_keys` should be the union of all canonical keys and aliases
/// from the sheet's `FieldDef::ALL` slice plus any explicitly-ignored columns.
#[allow(clippy::too_many_arguments)]
pub(super) fn route_extra_columns(
    ws: &Worksheet,
    row: u32,
    range: &DataRange,
    raw_headers: &[String],
    canonical_headers: &[Option<String>],
    known_field_keys: &std::collections::HashSet<String>,
    formula_columns: &[FormulaColumnDef],
    entity_uuid: uuid::NonNilUuid,
    entity_type_name: &str,
    schedule: &mut Schedule,
) {
    for (i, col) in (range.start_col..=range.end_col).enumerate() {
        let raw = &raw_headers[i];
        if raw.is_empty() {
            continue;
        }
        let canonical = match &canonical_headers[i] {
            Some(c) => c.as_str(),
            None => continue,
        };
        // Skip columns already handled by the field system.
        if known_field_keys.contains(canonical) {
            continue;
        }

        // --- Step 2: FormulaColumnDef list → sidecar formula extras ---
        let is_formula_col = formula_columns
            .iter()
            .any(|fd| fd.keys().any(|k| k == canonical));
        if is_formula_col {
            let formula_str = ws
                .get_cell((col, row))
                .map(|c| c.get_formula().to_string())
                .filter(|f| !f.is_empty());
            let display_value = get_cell_str(ws, col, row).unwrap_or_default();
            if !display_value.is_empty() || formula_str.is_some() {
                schedule.sidecar_mut().set_formula_extra(
                    entity_uuid,
                    raw.clone(),
                    SidecarFormulaField {
                        formula: formula_str,
                        display_value,
                    },
                );
            }
            continue;
        }

        // --- Step 3: ExtraFieldDescriptor registry → CRDT __extra ---
        if crate::extra_field::find_extra_descriptor(raw, entity_type_name).is_some() {
            if let Some(value) = get_cell_str(ws, col, row) {
                let _ = schedule.write_extra_field(entity_type_name, entity_uuid, raw, &value);
            }
            continue;
        }

        // --- Step 4: Unknown column: detect formula vs. plain value ---
        let formula_str = ws
            .get_cell((col, row))
            .map(|c| c.get_formula().to_string())
            .filter(|f| !f.is_empty());
        let display_value = get_cell_str(ws, col, row).unwrap_or_default();

        if let Some(formula) = formula_str {
            // Formula cell → sidecar (preserve formula for update_xlsx round-trip)
            if !display_value.is_empty() || !formula.is_empty() {
                schedule.sidecar_mut().set_formula_extra(
                    entity_uuid,
                    raw.clone(),
                    SidecarFormulaField {
                        formula: Some(formula),
                        display_value,
                    },
                );
            }
        } else if !display_value.is_empty() {
            // Plain value → CRDT __extra (shared, merged between users)
            let _ = schedule.write_extra_field(entity_type_name, entity_uuid, raw, &display_value);
        }
    }
}

/// Build the set of canonical keys that belong to the field system for a sheet.
///
/// Pass the sheet's `FieldDef::ALL` slice and any additional explicit-ignore
/// columns (e.g. `&[sc::OLD_UNIQ_ID]`).
pub(super) fn known_field_key_set(
    all_fields: &[FieldDef],
    ignore: &[FieldDef],
) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    for fd in all_fields.iter().chain(ignore.iter()) {
        for key in fd.keys() {
            set.insert(key.to_string());
        }
    }
    set
}
