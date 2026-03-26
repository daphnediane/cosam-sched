/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use anyhow::Result;
use chrono::{Local, Utc};
use umya_spreadsheet::structs::Worksheet;

use crate::data::panel::{ExtraFields, ExtraValue};
use crate::data::panel_type::PanelType;
use crate::data::presenter::Presenter;
use crate::data::room::Room;
use crate::data::schedule::Schedule;
use crate::data::source_info::{ChangeState, SourceInfo};
use crate::data::time;
use crate::file::ScheduleFile;
use crate::xlsx::columns::{FieldDef, panel_types as pt, people, room_map, schedule as sc};
use crate::xlsx::read::{
    PresenterColumn, PresenterHeader, canonical_header, parse_presenter_header,
};

use super::common::{FlatSession, flatten_panel_sessions, update_table_areas};

/// Update an existing XLSX file in place, preserving formatting, formulas,
/// and extra columns. Only modifies rows that have changed.
pub fn update_xlsx(sf: &ScheduleFile, path: &Path) -> Result<()> {
    let schedule = &sf.schedule;
    // Check for an Office lock file (~$filename) which indicates the file is open.
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let lock_name = format!("~${}", file_name);
        let lock_path = path.with_file_name(lock_name);
        if lock_path.exists() {
            return Err(anyhow::anyhow!(
                "File '{}' appears to be open in another application (lock file found). \
                 Close the file and try again.",
                path.display()
            ));
        }
    }

    // Record mtime before reading so we can detect external modifications later.
    let mtime_before: Option<SystemTime> = fs::metadata(path).ok().and_then(|m| m.modified().ok());

    let mut book = umya_spreadsheet::reader::xlsx::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read XLSX {}: {e}", path.display()))?;

    {
        let properties = book.get_properties_mut();
        if let Some(ref modified) = schedule.meta.modified {
            properties.set_modified(modified);
        } else {
            let now = time::format_storage_ts(Utc::now());
            properties.set_modified(&now);
        }
        if let Some(ref modified_by) = schedule.meta.last_modified_by {
            properties.set_last_modified_by(modified_by);
        }
    }

    if schedule.imported_sheets.has_room_map {
        if let Some(sheet_name) = find_sheet_name(schedule.rooms.iter().map(|r| &r.source)) {
            update_rooms_sheet(&mut book, &sheet_name, schedule)?;
        }
    }

    if schedule.imported_sheets.has_panel_types {
        if let Some(sheet_name) =
            find_sheet_name(schedule.panel_types.values().map(|pt| &pt.source))
        {
            update_panel_types_sheet(&mut book, &sheet_name, schedule)?;
        }
    }

    if schedule.imported_sheets.has_presenters {
        for sheet_name in &["People", "Presenters"] {
            if book.get_sheet_by_name(sheet_name).is_some() {
                update_people_sheet(&mut book, sheet_name, schedule)?;
                break;
            }
        }
    }

    if schedule.imported_sheets.has_schedule {
        if let Some(sheet_name) = find_sheet_name(
            schedule
                .panel_sets
                .values()
                .flat_map(|ps| ps.panels.iter())
                .map(|p| &p.source),
        ) {
            update_schedule_sheet(&mut book, &sheet_name, schedule)?;
        }
    }

    // Add Grid sheet (create if it doesn't exist)
    if book.get_sheet_by_name("Grid").is_none() {
        book.new_sheet("Grid").map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let ws = book
            .get_sheet_by_name_mut("Grid")
            .ok_or_else(|| anyhow::anyhow!("Sheet 'Grid' not found"))?;
        super::grid::write_grid_sheet(ws, schedule)?;
    }

    // Update Timestamp sheet if present (record processing time in A2).
    if let Some(ws) = book.get_sheet_by_name_mut("Timestamp") {
        let now = Local::now().format(time::LOCAL_TS_FMT).to_string();
        ws.get_cell_mut((1, 2)).set_value(&now);
    }

    // Verify the file hasn't been modified externally while we were processing it.
    if let Some(before) = mtime_before {
        if let Ok(meta) = fs::metadata(path) {
            if let Ok(mtime_now) = meta.modified() {
                if mtime_now != before {
                    return Err(anyhow::anyhow!(
                        "File '{}' was modified externally while being processed. \
                         Aborting to prevent data loss.",
                        path.display()
                    ));
                }
            }
        }
    }

    // Write atomically: write to a temp file then rename into place.
    let tmp_path = path.with_extension("xlsx.tmp");
    let write_result = umya_spreadsheet::writer::xlsx::write(&book, &tmp_path)
        .map_err(|e| anyhow::anyhow!("Failed to write XLSX {}: {e}", tmp_path.display()));

    match write_result {
        Ok(()) => {
            fs::rename(&tmp_path, path).map_err(|e| {
                let _ = fs::remove_file(&tmp_path);
                anyhow::anyhow!("Failed to replace '{}' with temp file: {e}", path.display())
            })?;
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            return Err(e);
        }
    }

    Ok(())
}

/// After a successful save, remove Deleted items and reset all change states.
pub fn post_save_cleanup(sf: &mut ScheduleFile) {
    let schedule = &mut sf.schedule;
    // Remove deleted panel sets and their panels
    schedule.panel_sets.retain(|_, ps| {
        !ps.panels
            .iter()
            .all(|p| p.change_state == ChangeState::Deleted)
    });
    for ps in schedule.panel_sets.values_mut() {
        ps.panels.retain(|p| p.change_state != ChangeState::Deleted);
        for panel in &mut ps.panels {
            panel.change_state = ChangeState::Unchanged;
        }
    }

    schedule
        .rooms
        .retain(|r| r.change_state != ChangeState::Deleted);
    schedule
        .panel_types
        .retain(|_, pt| pt.change_state != ChangeState::Deleted);
    schedule
        .presenters
        .retain(|p| p.change_state != ChangeState::Deleted);
    schedule
        .timeline
        .retain(|t| t.change_state != ChangeState::Deleted);

    // Reset change states to Unchanged
    for ps in schedule.panel_sets.values_mut() {
        for panel in &mut ps.panels {
            panel.change_state = ChangeState::Unchanged;
        }
    }
    for room in &mut schedule.rooms {
        room.change_state = ChangeState::Unchanged;
    }
    for (_, pt) in &mut schedule.panel_types {
        pt.change_state = ChangeState::Unchanged;
    }
    for presenter in &mut schedule.presenters {
        presenter.change_state = ChangeState::Unchanged;
    }
    for timeline in &mut schedule.timeline {
        timeline.change_state = ChangeState::Unchanged;
    }
}

fn find_sheet_name<'a, I>(sources: I) -> Option<String>
where
    I: Iterator<Item = &'a Option<SourceInfo>>,
{
    sources
        .filter_map(|s| s.as_ref())
        .filter_map(|s| s.sheet_name.clone())
        .next()
}

fn build_header_map(worksheet: &Worksheet) -> HashMap<String, u32> {
    let max_col = worksheet.get_highest_column();
    let mut map = HashMap::new();
    for col in 1..=max_col {
        let value = worksheet.get_value((col, 1));
        if let Some(key) = canonical_header(&value) {
            map.entry(key).or_insert(col);
        }
    }
    map
}

fn build_raw_header_map(worksheet: &Worksheet) -> HashMap<String, u32> {
    let max_col = worksheet.get_highest_column();
    let mut map = HashMap::new();
    for col in 1..=max_col {
        let value = worksheet.get_value((col, 1)).trim().to_string();
        if !value.is_empty() {
            map.entry(value).or_insert(col);
        }
    }
    map
}

/// Append new column header cells for keys not yet present in the worksheet.
/// Returns a map of raw_key → new column index.
pub(super) fn add_new_metadata_columns<'a>(
    worksheet: &mut Worksheet,
    new_keys: impl Iterator<Item = &'a str>,
) -> HashMap<String, u32> {
    let mut next_col = worksheet.get_highest_column() + 1;
    let mut result = HashMap::new();
    for key in new_keys {
        worksheet.get_cell_mut((next_col, 1)).set_value(key);
        result.insert(key.to_string(), next_col);
        next_col += 1;
    }
    result
}

/// Write ExtraFields metadata into row cells using the raw header map.
fn write_metadata_to_row(
    worksheet: &mut Worksheet,
    raw_map: &HashMap<String, u32>,
    row: u32,
    metadata: &Option<ExtraFields>,
) {
    let Some(meta) = metadata else { return };
    for (key, value) in meta.iter() {
        if let Some(&col) = raw_map.get(key.as_str()) {
            match value {
                ExtraValue::Formula(fv) => {
                    worksheet.get_cell_mut((col, row)).set_formula(&fv.formula);
                }
                ExtraValue::String(s) => {
                    worksheet.get_cell_mut((col, row)).set_value(s.as_str());
                }
            }
        }
    }
}

/// Holds all column lookup maps built from a worksheet's header row.
struct HeaderMaps {
    canonical: HashMap<String, u32>,
    raw: HashMap<String, u32>,
    presenter_cols: Vec<PresenterColumn>,
}

fn build_extended_header_map(worksheet: &Worksheet) -> HeaderMaps {
    let max_col = worksheet.get_highest_column();
    let mut canonical = HashMap::new();
    let mut raw: HashMap<String, u32> = HashMap::new();
    let mut presenter_cols = Vec::new();

    for col in 1..=max_col {
        let value = worksheet.get_value((col, 1));
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        raw.entry(value.to_string()).or_insert(col);
        if let Some(key) = canonical_header(value) {
            canonical.entry(key).or_insert(col);
        }
        if let Some(pc) = parse_presenter_header(value, col) {
            presenter_cols.push(pc);
        }
    }

    HeaderMaps {
        canonical,
        raw,
        presenter_cols,
    }
}

/// Strip group/prefix markers from a presenter column header name to get the plain name.
/// e.g. "Alice=UNC Staff" → "Alice", "<Bob==Team" → "Bob"
fn decode_presenter_name_from_header(encoded: &str) -> String {
    let name = if let Some(eq_pos) = encoded.find('=') {
        &encoded[..eq_pos]
    } else {
        encoded
    };
    name.trim_start_matches('<').trim().to_string()
}

/// Write presenter column values for a session row.
/// Only writes to columns that already exist; never adds new columns.
fn write_presenter_columns(
    worksheet: &mut Worksheet,
    header_maps: &HeaderMaps,
    row: u32,
    session: &FlatSession,
    schedule: &Schedule,
) {
    let mut covered_by_named: std::collections::HashSet<String> = std::collections::HashSet::new();

    for pc in &header_maps.presenter_cols {
        match &pc.header {
            PresenterHeader::Named(encoded_name) => {
                let presenter_name = decode_presenter_name_from_header(encoded_name);
                let value = if session.credited_set.contains(&presenter_name) {
                    covered_by_named.insert(presenter_name);
                    "Yes"
                } else if session.all_presenters.contains(&presenter_name) {
                    covered_by_named.insert(presenter_name);
                    "*"
                } else {
                    ""
                };
                worksheet.get_cell_mut((pc.col, row)).set_value(value);
            }
            PresenterHeader::Other => {
                let rank = pc.rank.as_str();
                let others: Vec<String> = session
                    .all_presenters
                    .iter()
                    .filter(|name| !covered_by_named.contains(*name))
                    .filter(|name| {
                        schedule
                            .presenters
                            .iter()
                            .find(|p| &p.name == *name)
                            .map(|p| p.rank.as_str() == rank)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();
                let value = others.join(", ");
                worksheet.get_cell_mut((pc.col, row)).set_value(&value);
            }
        }
    }
}

fn set_cell_field(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    field: &FieldDef,
    value: &str,
) {
    for key in field.keys() {
        if let Some(&col) = header_map.get(key) {
            worksheet.get_cell_mut((col, row)).set_value(value);
            return;
        }
    }
}

fn set_cell_opt_field(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    field: &FieldDef,
    value: &Option<String>,
) {
    set_cell_field(
        worksheet,
        header_map,
        row,
        field,
        value.as_deref().unwrap_or(""),
    );
}

fn set_cell_bool_field(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    field: &FieldDef,
    value: bool,
) {
    set_cell_field(
        worksheet,
        header_map,
        row,
        field,
        if value { "Yes" } else { "" },
    );
}

// ── Rooms ──────────────────────────────────────────────────────────────────

fn write_room_to_row(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    raw_map: &HashMap<String, u32>,
    row: u32,
    room: &Room,
) {
    set_cell_field(
        worksheet,
        header_map,
        row,
        &room_map::ROOM_NAME,
        &room.short_name,
    );
    set_cell_field(
        worksheet,
        header_map,
        row,
        &room_map::LONG_NAME,
        &room.long_name,
    );
    set_cell_field(
        worksheet,
        header_map,
        row,
        &room_map::HOTEL_ROOM,
        &room.hotel_room,
    );
    set_cell_field(
        worksheet,
        header_map,
        row,
        &room_map::SORT_KEY,
        &room.sort_key.to_string(),
    );
    write_metadata_to_row(worksheet, raw_map, row, &room.metadata);
}

fn update_rooms_sheet(
    book: &mut umya_spreadsheet::Spreadsheet,
    sheet_name: &str,
    schedule: &Schedule,
) -> Result<()> {
    let worksheet = book
        .get_sheet_by_name(sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
    let header_map = build_header_map(worksheet);
    let mut raw_map = build_raw_header_map(worksheet);
    let highest_row = worksheet.get_highest_row();

    // Pre-scan: collect metadata keys from rooms that need writing but have no column yet.
    let mut new_keys: Vec<String> = Vec::new();
    for room in &schedule.rooms {
        if !matches!(
            room.change_state,
            ChangeState::Modified | ChangeState::Replaced | ChangeState::Added
        ) {
            continue;
        }
        if let Some(meta) = &room.metadata {
            for key in meta.keys() {
                if !raw_map.contains_key(key.as_str()) && !new_keys.contains(key) {
                    new_keys.push(key.clone());
                }
            }
        }
    }
    if !new_keys.is_empty() {
        let worksheet = book
            .get_sheet_by_name_mut(sheet_name)
            .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
        let added = add_new_metadata_columns(worksheet, new_keys.iter().map(String::as_str));
        raw_map.extend(added);
    }

    let mut rows_to_delete: Vec<u32> = Vec::new();
    let mut rows_to_append: Vec<&Room> = Vec::new();

    for room in &schedule.rooms {
        match room.change_state {
            ChangeState::Deleted => {
                if let Some(row_index) = room.source.as_ref().and_then(|s| s.row_index) {
                    rows_to_delete.push(row_index);
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                if let Some(row_index) = room.source.as_ref().and_then(|s| s.row_index) {
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_room_to_row(worksheet, &header_map, &raw_map, row_index, room);
                }
            }
            ChangeState::Added => {
                rows_to_append.push(room);
            }
            ChangeState::Unchanged | ChangeState::Converted => {}
        }
    }

    rows_to_delete.sort_unstable();
    rows_to_delete.reverse();
    for row in &rows_to_delete {
        book.remove_row(sheet_name, row, &1);
    }

    let mut next_row = highest_row + 1 - rows_to_delete.len() as u32;
    for room in rows_to_append {
        let worksheet = book
            .get_sheet_by_name_mut(sheet_name)
            .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
        write_room_to_row(worksheet, &header_map, &raw_map, next_row, room);
        next_row += 1;
    }

    let final_last_row = next_row - 1;
    if let Some(ws) = book.get_sheet_by_name_mut(sheet_name) {
        update_table_areas(ws, final_last_row);
    }

    Ok(())
}

// ── Panel Types ────────────────────────────────────────────────────────────

fn write_panel_type_to_row(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    raw_map: &HashMap<String, u32>,
    row: u32,
    panel_type: &PanelType,
) {
    set_cell_field(worksheet, header_map, row, &pt::PREFIX, &panel_type.prefix);
    set_cell_field(
        worksheet,
        header_map,
        row,
        &pt::PANEL_KIND,
        &panel_type.kind,
    );
    let color_opt = panel_type.color().map(|s| s.to_string());
    set_cell_opt_field(worksheet, header_map, row, &pt::COLOR, &color_opt);
    let bw_opt = panel_type.bw_color().map(|s| s.to_string());
    set_cell_opt_field(worksheet, header_map, row, &pt::BW_COLOR, &bw_opt);
    set_cell_bool_field(
        worksheet,
        header_map,
        row,
        &pt::IS_BREAK,
        panel_type.is_break,
    );
    set_cell_bool_field(worksheet, header_map, row, &pt::IS_CAFE, panel_type.is_cafe);
    set_cell_bool_field(
        worksheet,
        header_map,
        row,
        &pt::IS_WORKSHOP,
        panel_type.is_workshop,
    );
    set_cell_bool_field(
        worksheet,
        header_map,
        row,
        &pt::IS_ROOM_HOURS,
        panel_type.is_room_hours,
    );
    set_cell_field(
        worksheet,
        header_map,
        row,
        &pt::HIDDEN,
        if panel_type.is_hidden { "Yes" } else { "" },
    );
    write_metadata_to_row(worksheet, raw_map, row, &panel_type.metadata);
}

fn update_panel_types_sheet(
    book: &mut umya_spreadsheet::Spreadsheet,
    sheet_name: &str,
    schedule: &Schedule,
) -> Result<()> {
    let worksheet = book
        .get_sheet_by_name(sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
    let header_map = build_header_map(worksheet);
    let mut raw_map = build_raw_header_map(worksheet);
    let highest_row = worksheet.get_highest_row();

    let mut new_keys: Vec<String> = Vec::new();
    for (_prefix, panel_type) in &schedule.panel_types {
        if !matches!(
            panel_type.change_state,
            ChangeState::Modified | ChangeState::Replaced | ChangeState::Added
        ) {
            continue;
        }
        if let Some(meta) = &panel_type.metadata {
            for key in meta.keys() {
                if !raw_map.contains_key(key.as_str()) && !new_keys.contains(key) {
                    new_keys.push(key.clone());
                }
            }
        }
    }
    if !new_keys.is_empty() {
        let worksheet = book
            .get_sheet_by_name_mut(sheet_name)
            .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
        let added = add_new_metadata_columns(worksheet, new_keys.iter().map(String::as_str));
        raw_map.extend(added);
    }

    let mut rows_to_delete: Vec<u32> = Vec::new();
    let mut rows_to_append: Vec<&PanelType> = Vec::new();

    for (_prefix, panel_type) in &schedule.panel_types {
        match panel_type.change_state {
            ChangeState::Deleted => {
                if let Some(row_index) = panel_type.source.as_ref().and_then(|s| s.row_index) {
                    rows_to_delete.push(row_index);
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                if let Some(row_index) = panel_type.source.as_ref().and_then(|s| s.row_index) {
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_panel_type_to_row(
                        worksheet,
                        &header_map,
                        &raw_map,
                        row_index,
                        panel_type,
                    );
                }
            }
            ChangeState::Added => {
                rows_to_append.push(panel_type);
            }
            ChangeState::Unchanged | ChangeState::Converted => {}
        }
    }

    rows_to_delete.sort_unstable();
    rows_to_delete.reverse();
    for row in &rows_to_delete {
        book.remove_row(sheet_name, row, &1);
    }

    let mut next_row = highest_row + 1 - rows_to_delete.len() as u32;
    for panel_type in rows_to_append {
        let worksheet = book
            .get_sheet_by_name_mut(sheet_name)
            .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
        write_panel_type_to_row(worksheet, &header_map, &raw_map, next_row, panel_type);
        next_row += 1;
    }

    let final_last_row = next_row - 1;
    if let Some(ws) = book.get_sheet_by_name_mut(sheet_name) {
        update_table_areas(ws, final_last_row);
    }

    Ok(())
}

// ── People (Presenters) ────────────────────────────────────────────────────

fn write_presenter_to_row(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    presenter: &Presenter,
) {
    set_cell_field(worksheet, header_map, row, &people::NAME, &presenter.name);
    set_cell_field(
        worksheet,
        header_map,
        row,
        &people::CLASSIFICATION,
        presenter.rank.as_str(),
    );
    set_cell_bool_field(
        worksheet,
        header_map,
        row,
        &people::IS_GROUP,
        presenter.is_group(),
    );
    let members_str = presenter
        .members()
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if !members_str.is_empty() {
        set_cell_field(worksheet, header_map, row, &people::MEMBERS, &members_str);
    }
    let groups_str = presenter
        .groups()
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if !groups_str.is_empty() {
        set_cell_field(worksheet, header_map, row, &people::GROUPS, &groups_str);
    }
    set_cell_bool_field(
        worksheet,
        header_map,
        row,
        &people::ALWAYS_GROUPED,
        presenter.always_grouped(),
    );
}

fn update_people_sheet(
    book: &mut umya_spreadsheet::Spreadsheet,
    sheet_name: &str,
    schedule: &Schedule,
) -> Result<()> {
    let worksheet = book
        .get_sheet_by_name(sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
    let header_map = build_header_map(worksheet);
    let highest_row = worksheet.get_highest_row();

    // Build a name→row map to locate existing presenter rows.
    let name_col = people::NAME
        .keys()
        .find_map(|k| header_map.get(k).copied())
        .unwrap_or(1);
    let mut name_to_row: HashMap<String, u32> = HashMap::new();
    for row in 2..=highest_row {
        let name = worksheet.get_value((name_col, row)).trim().to_string();
        if !name.is_empty() {
            name_to_row.insert(name.to_lowercase(), row);
        }
    }

    let mut presenters_to_append: Vec<&Presenter> = Vec::new();

    for presenter in &schedule.presenters {
        match presenter.change_state {
            ChangeState::Deleted => {
                // Mark the name with * prefix; do not remove the row.
                let row_index = presenter
                    .source
                    .as_ref()
                    .and_then(|s| s.row_index)
                    .or_else(|| name_to_row.get(&presenter.name.to_lowercase()).copied());
                if let Some(row_index) = row_index {
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    let prefixed = format!("*{}", presenter.name);
                    set_cell_field(worksheet, &header_map, row_index, &people::NAME, &prefixed);
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                let row_index = presenter
                    .source
                    .as_ref()
                    .and_then(|s| s.row_index)
                    .or_else(|| name_to_row.get(&presenter.name.to_lowercase()).copied());
                if let Some(row_index) = row_index {
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_presenter_to_row(worksheet, &header_map, row_index, presenter);
                }
            }
            ChangeState::Added => {
                presenters_to_append.push(presenter);
            }
            ChangeState::Unchanged | ChangeState::Converted => {}
        }
    }

    let mut next_row = highest_row + 1;
    for presenter in presenters_to_append {
        let worksheet = book
            .get_sheet_by_name_mut(sheet_name)
            .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
        write_presenter_to_row(worksheet, &header_map, next_row, presenter);
        next_row += 1;
    }

    let final_last_row = next_row - 1;
    if let Some(ws) = book.get_sheet_by_name_mut(sheet_name) {
        update_table_areas(ws, final_last_row);
    }

    Ok(())
}

// ── Schedule (Panels) ──────────────────────────────────────────────────────

fn write_session_to_row(
    worksheet: &mut Worksheet,
    header_maps: &HeaderMaps,
    row: u32,
    session: &FlatSession,
    schedule: &Schedule,
) {
    let hm = &header_maps.canonical;

    set_cell_field(worksheet, hm, row, &sc::UNIQ_ID, &session.id);
    set_cell_field(worksheet, hm, row, &sc::NAME, &session.name);
    set_cell_opt_field(worksheet, hm, row, &sc::DESCRIPTION, &session.description);

    // Always write start_time (even empty) so unscheduled rows are cleared correctly.
    // Never write to Lstart or Lend — those are formula columns.
    let start_val = session.start_time.as_deref().unwrap_or("");
    set_cell_field(worksheet, hm, row, &sc::START_TIME, start_val);

    // End time: only write to explicit End Time column, not Lend (which is formula-based).
    let end_val = session.end_time.as_deref().unwrap_or("");
    set_cell_field(worksheet, hm, row, &sc::END_TIME, end_val);

    set_cell_field(
        worksheet,
        hm,
        row,
        &sc::DURATION,
        &session.duration.to_string(),
    );

    // Always write room (empty when unscheduled) to clear stale data.
    let room_name = session
        .room_id
        .and_then(|rid| schedule.room_by_id(rid))
        .map(|r| r.short_name.as_str())
        .unwrap_or("");
    set_cell_field(worksheet, hm, row, &sc::ROOM, room_name);

    let kind = session
        .panel_type
        .as_ref()
        .and_then(|pt_uid| schedule.panel_types.get(pt_uid))
        .map(|pt| pt.kind.as_str())
        .unwrap_or("");
    set_cell_field(worksheet, hm, row, &sc::KIND, kind);

    set_cell_opt_field(worksheet, hm, row, &sc::COST, &session.cost);
    set_cell_opt_field(worksheet, hm, row, &sc::CAPACITY, &session.capacity);
    set_cell_opt_field(worksheet, hm, row, &sc::DIFFICULTY, &session.difficulty);
    set_cell_opt_field(worksheet, hm, row, &sc::NOTE, &session.note);
    set_cell_opt_field(worksheet, hm, row, &sc::PREREQ, &session.prereq);
    set_cell_opt_field(worksheet, hm, row, &sc::TICKET_SALE, &session.ticket_url);
    set_cell_bool_field(worksheet, hm, row, &sc::FULL, session.is_full);
    set_cell_bool_field(
        worksheet,
        hm,
        row,
        &sc::HIDE_PANELIST,
        session.hide_panelist,
    );
    set_cell_opt_field(worksheet, hm, row, &sc::ALT_PANELIST, &session.alt_panelist);

    // Write presenter columns (existing columns only; no new columns added).
    write_presenter_columns(worksheet, header_maps, row, session, schedule);

    // Write metadata back to non-standard columns.
    for (raw_header, value) in &session.metadata {
        if let Some(&col) = header_maps.raw.get(raw_header.as_str()) {
            match value {
                ExtraValue::Formula(fv) => {
                    worksheet.get_cell_mut((col, row)).set_formula(&fv.formula);
                }
                ExtraValue::String(s) => {
                    worksheet.get_cell_mut((col, row)).set_value(s);
                }
            }
        }
    }
}

fn update_schedule_sheet(
    book: &mut umya_spreadsheet::Spreadsheet,
    sheet_name: &str,
    schedule: &Schedule,
) -> Result<()> {
    let worksheet = book
        .get_sheet_by_name(sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
    let mut header_maps = build_extended_header_map(worksheet);
    let highest_row = worksheet.get_highest_row();

    let sessions = flatten_panel_sessions(schedule, true);

    // Pre-scan: find metadata keys not yet present as columns and add them.
    let mut new_keys: Vec<String> = Vec::new();
    for session in &sessions {
        for key in session.metadata.keys() {
            if !header_maps.raw.contains_key(key.as_str()) && !new_keys.contains(key) {
                new_keys.push(key.clone());
            }
        }
    }
    if !new_keys.is_empty() {
        let worksheet = book
            .get_sheet_by_name_mut(sheet_name)
            .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
        let added = add_new_metadata_columns(worksheet, new_keys.iter().map(String::as_str));
        header_maps.raw.extend(added);
    }

    let mut sessions_to_append: Vec<usize> = Vec::new();

    for (idx, session) in sessions.iter().enumerate() {
        match session.change_state {
            ChangeState::Deleted => {
                // Mark deleted rows in place: prefix Uniq ID with * and record Old Uniq Id.
                // Rows are never removed — preserves formatting and avoids row-shift corruption.
                if let Some(row_index) = session.source.as_ref().and_then(|s| s.row_index) {
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    let prefixed_id = format!("*{}", session.id);
                    set_cell_field(
                        worksheet,
                        &header_maps.canonical,
                        row_index,
                        &sc::UNIQ_ID,
                        &prefixed_id,
                    );
                    // Write original ID to Old Uniq Id if that column exists and cell is empty
                    let old_id_col = sc::OLD_UNIQ_ID
                        .keys()
                        .find_map(|k| header_maps.canonical.get(k))
                        .copied();
                    if let Some(col) = old_id_col {
                        let current = worksheet.get_value((col, row_index));
                        if current.trim().is_empty() {
                            worksheet
                                .get_cell_mut((col, row_index))
                                .set_value(&session.id);
                        }
                    }
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                if let Some(row_index) = session.source.as_ref().and_then(|s| s.row_index) {
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_session_to_row(worksheet, &header_maps, row_index, session, schedule);
                }
            }
            ChangeState::Added => {
                sessions_to_append.push(idx);
            }
            ChangeState::Unchanged | ChangeState::Converted => {}
        }
    }

    // Append new sessions after existing rows (never delete rows).
    let mut next_row = highest_row + 1;
    for idx in sessions_to_append {
        let session = &sessions[idx];
        let worksheet = book
            .get_sheet_by_name_mut(sheet_name)
            .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
        write_session_to_row(worksheet, &header_maps, next_row, session, schedule);
        next_row += 1;
    }

    let final_last_row = next_row - 1;
    if let Some(ws) = book.get_sheet_by_name_mut(sheet_name) {
        update_table_areas(ws, final_last_row);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::panel::Panel;
    use crate::data::panel_set::PanelSet;
    use crate::data::presenter::{Presenter, PresenterGroup, PresenterMember, PresenterRank};
    use crate::data::schedule::{Meta, Schedule};
    use crate::data::source_info::ImportedSheetPresence;
    use crate::data::timeline::TimelineEntry;

    fn make_schedule_with_change_states() -> Schedule {
        Schedule {
            conflicts: Vec::new(),
            meta: Meta {
                title: "Test".to_string(),
                generated: "2026-01-01".to_string(),
                version: Some(8),
                variant: None,
                generator: None,
                start_time: None,
                end_time: None,
                next_presenter_id: None,
                creator: None,
                last_modified_by: None,
                modified: None,
            },
            panel_sets: indexmap::IndexMap::new(),
            timeline: vec![
                TimelineEntry {
                    id: "TL01".to_string(),
                    start_time: Some(
                        chrono::NaiveDateTime::parse_from_str(
                            "2026-06-26T09:00:00",
                            "%Y-%m-%dT%H:%M:%S",
                        )
                        .unwrap(),
                    ),
                    description: "Opening".to_string(),
                    panel_type: None,
                    note: None,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Unchanged,
                },
                TimelineEntry {
                    id: "TL02".to_string(),
                    start_time: Some(
                        chrono::NaiveDateTime::parse_from_str(
                            "2026-06-26T10:00:00",
                            "%Y-%m-%dT%H:%M:%S",
                        )
                        .unwrap(),
                    ),
                    description: "Deleted entry".to_string(),
                    panel_type: None,
                    note: None,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Deleted,
                },
            ],
            rooms: vec![
                crate::data::room::Room {
                    uid: 1,
                    short_name: "Main".to_string(),
                    long_name: "Main Hall".to_string(),
                    hotel_room: "Ballroom A".to_string(),
                    sort_key: 1,
                    is_break: false,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Unchanged,
                },
                crate::data::room::Room {
                    uid: 2,
                    short_name: "Old".to_string(),
                    long_name: "Old Room".to_string(),
                    hotel_room: "".to_string(),
                    sort_key: 2,
                    is_break: false,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Deleted,
                },
            ],
            panel_types: {
                let mut pt_map = indexmap::IndexMap::new();
                pt_map.insert(
                    "GP".to_string(),
                    crate::data::panel_type::PanelType {
                        prefix: "GP".to_string(),
                        kind: "General Panel".to_string(),
                        colors: indexmap::IndexMap::new(),
                        is_break: false,
                        is_cafe: false,
                        is_workshop: false,
                        is_hidden: false,
                        is_room_hours: false,
                        is_timeline: false,
                        is_private: false,
                        metadata: None,
                        source: None,
                        change_state: ChangeState::Modified,
                    },
                );
                pt_map
            },
            presenters: vec![
                Presenter {
                    id: None,
                    name: "Alice".to_string(),
                    rank: PresenterRank::from_str("guest"),
                    is_member: PresenterMember::NotMember,
                    is_grouped: PresenterGroup::NotGroup,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Converted,
                },
                Presenter {
                    id: None,
                    name: "Bob".to_string(),
                    rank: PresenterRank::from_str("staff"),
                    is_member: PresenterMember::NotMember,
                    is_grouped: PresenterGroup::NotGroup,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Deleted,
                },
            ],
            imported_sheets: ImportedSheetPresence {
                has_room_map: true,
                has_panel_types: true,
                has_presenters: false,
                has_schedule: true,
            },
        }
    }

    #[test]
    fn test_post_save_cleanup_removes_deleted() {
        let mut sf = crate::file::ScheduleFile::new(make_schedule_with_change_states());

        let initial_room_count = sf.schedule.rooms.len();
        let initial_presenter_count = sf.schedule.presenters.len();
        let initial_timeline_count = sf.schedule.timeline.len();

        post_save_cleanup(&mut sf);

        assert_eq!(
            sf.schedule.rooms.len(),
            initial_room_count - 1,
            "Deleted room should be removed"
        );
        assert_eq!(
            sf.schedule.presenters.len(),
            initial_presenter_count - 1,
            "Deleted presenter should be removed"
        );
        assert_eq!(
            sf.schedule.timeline.len(),
            initial_timeline_count - 1,
            "Deleted timeline entry should be removed"
        );

        assert!(!sf.schedule.rooms.iter().any(|r| r.short_name == "Old"));
        assert!(!sf.schedule.presenters.iter().any(|p| p.name == "Bob"));
    }

    #[test]
    fn test_post_save_cleanup_resets_change_states() {
        let mut sf = crate::file::ScheduleFile::new(make_schedule_with_change_states());
        post_save_cleanup(&mut sf);

        for room in &sf.schedule.rooms {
            assert_eq!(room.change_state, ChangeState::Unchanged);
        }
        for presenter in &sf.schedule.presenters {
            assert_eq!(presenter.change_state, ChangeState::Unchanged);
        }
        for timeline in &sf.schedule.timeline {
            assert_eq!(timeline.change_state, ChangeState::Unchanged);
        }
    }

    #[test]
    fn test_post_save_cleanup_preserves_data() {
        let mut sf = crate::file::ScheduleFile::new(make_schedule_with_change_states());
        post_save_cleanup(&mut sf);

        assert!(sf.schedule.rooms.iter().any(|r| r.short_name == "Main"));
        assert!(sf.schedule.presenters.iter().any(|p| p.name == "Alice"));
        assert!(sf.schedule.timeline.iter().any(|t| t.id == "TL01"));
    }

    #[test]
    fn test_find_sheet_name_returns_first_source() {
        let sources = vec![
            None,
            Some(SourceInfo {
                file_path: Some("test.xlsx".to_string()),
                sheet_name: Some("MySheet".to_string()),
                row_index: Some(1),
            }),
            Some(SourceInfo {
                file_path: Some("test.xlsx".to_string()),
                sheet_name: Some("Other".to_string()),
                row_index: Some(2),
            }),
        ];
        let result = find_sheet_name(sources.iter());
        assert_eq!(result, Some("MySheet".to_string()));
    }

    #[test]
    fn test_find_sheet_name_returns_none_when_empty() {
        let sources: Vec<Option<SourceInfo>> = vec![None, None];
        let result = find_sheet_name(sources.iter());
        assert_eq!(result, None);
    }

    #[test]
    fn test_decode_presenter_name_plain() {
        assert_eq!(decode_presenter_name_from_header("Alice"), "Alice");
    }

    #[test]
    fn test_decode_presenter_name_with_group() {
        assert_eq!(
            decode_presenter_name_from_header("Alice=UNC Staff"),
            "Alice"
        );
    }

    #[test]
    fn test_decode_presenter_name_always_grouped_prefix() {
        assert_eq!(decode_presenter_name_from_header("<Bob=Team"), "Bob");
    }

    #[test]
    fn test_flatten_includes_deleted_sessions() {
        let mut schedule = make_schedule_with_change_states();
        let mut panel = Panel::new("GP-001", "GP-001");
        panel.name = "Test Panel".to_string();
        panel.change_state = ChangeState::Deleted;
        panel.source = Some(SourceInfo {
            file_path: Some("test.xlsx".to_string()),
            sheet_name: Some("Schedule".to_string()),
            row_index: Some(5),
        });
        let mut ps = PanelSet::new("GP-001");
        ps.panels.push(panel);
        schedule.panel_sets.insert("GP-001".to_string(), ps);

        let sessions = flatten_panel_sessions(&schedule, true);
        let deleted: Vec<_> = sessions
            .iter()
            .filter(|s| s.change_state == ChangeState::Deleted)
            .collect();
        assert!(!deleted.is_empty(), "Deleted sessions must be included");
        assert_eq!(deleted[0].id, "GP-001");
    }

    #[test]
    fn test_flatten_merges_presenter_credited_status() {
        let mut schedule = make_schedule_with_change_states();
        let mut panel = Panel::new("GP-002", "GP-002");
        panel.name = "Presenter Panel".to_string();
        panel.credited_presenters = vec!["Alice".to_string()];
        panel.uncredited_presenters = vec!["Bob".to_string()];
        panel.change_state = ChangeState::Modified;
        let mut ps = PanelSet::new("GP-002");
        ps.panels.push(panel);
        schedule.panel_sets.insert("GP-002".to_string(), ps);

        let sessions = flatten_panel_sessions(&schedule, true);
        let s = sessions.iter().find(|s| s.id == "GP-002").unwrap();
        assert!(s.all_presenters.contains(&"Alice".to_string()));
        assert!(s.all_presenters.contains(&"Bob".to_string()));
        assert!(s.credited_set.contains("Alice"), "Alice should be credited");
        assert!(!s.credited_set.contains("Bob"), "Bob should be uncredited");
    }
}
