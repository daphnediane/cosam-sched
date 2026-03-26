/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use chrono::NaiveDateTime;
use indexmap::IndexMap;
use regex::Regex;
use umya_spreadsheet::Spreadsheet;

use crate::data::event::EventConflict;
use crate::data::panel::{ExtraValue, FormulaValue, Panel};
use crate::data::panel_id::PanelId;
use crate::data::panel_set::PanelSet;
use crate::data::panel_type::PanelType;
use crate::data::presenter::{PresenterGroup, PresenterMember, PresenterRank};
use crate::data::room::Room;
use crate::data::source_info::{ChangeState, SourceInfo};
use crate::data::time;
use crate::data::timeline::TimelineEntry;

use crate::xlsx::columns::schedule as sc;

use super::find_data_range;
use super::headers::{PresenterColumn, PresenterHeader, parse_presenter_header};
use super::people::{PresenterInfo, parse_presenter_data};
use super::{
    build_column_map, canonical_header, get_cell_number, get_cell_str, get_field, get_field_def,
    is_truthy, row_to_map,
};

pub(super) fn read_panels(
    book: &Spreadsheet,
    preferred: &str,
    rooms: &[Room],
    panel_types: &IndexMap<String, PanelType>,
    file_path: &str,
    _presenter_ranks: &HashMap<String, String>,
) -> Result<(
    IndexMap<String, PanelSet>,
    HashMap<String, PresenterInfo>,
    Vec<TimelineEntry>,
)> {
    let first_sheet_name = book
        .get_sheet_collection()
        .first()
        .map(|s| s.get_name().to_string());
    let first_sheet_ref: &str = first_sheet_name.as_deref().unwrap_or("");
    let range = match find_data_range(book, preferred, &["Schedule", first_sheet_ref]) {
        Some(r) => {
            // Check if the table range is smaller than the actual data
            let ws = book.get_sheet_by_name(&r.sheet_name).unwrap();
            let actual_end_row = ws.get_highest_row();
            let actual_end_col = ws.get_highest_column();

            // If there's more data beyond the table, extend the range
            if actual_end_row > r.end_row {
                super::DataRange {
                    sheet_name: r.sheet_name,
                    start_col: r.start_col,
                    header_row: r.header_row,
                    end_col: actual_end_col.max(r.end_col),
                    end_row: actual_end_row,
                }
            } else {
                r
            }
        }
        None => return Ok((IndexMap::new(), HashMap::new(), Vec::new())),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok((IndexMap::new(), HashMap::new(), Vec::new())); // 3-tuple still matches
    }

    let (raw_headers, canonical_headers, col_map) = build_column_map(ws, &range);

    let ticket_field_keys: HashSet<String> = sc::TICKET_SALE
        .keys()
        .chain(sc::SIMPLE_TIX_EVENT.keys())
        .filter_map(|k| canonical_header(k))
        .collect();
    let ticket_cols: HashSet<u32> = canonical_headers
        .iter()
        .enumerate()
        .filter_map(|(i, canon)| {
            let key = canon.as_deref()?;
            if ticket_field_keys.contains(key) {
                Some(range.start_col + i as u32)
            } else {
                None
            }
        })
        .collect();

    let presenter_cols: Vec<PresenterColumn> = raw_headers
        .iter()
        .enumerate()
        .filter_map(|(i, h)| parse_presenter_header(h, range.start_col + i as u32))
        .collect();

    // Known canonical keys derived from FieldDef constants (ALL + Lstart/Lend + presenter fallback).
    let known_canonical_headers: HashSet<String> = sc::ALL
        .iter()
        .chain([sc::LSTART, sc::LEND].iter())
        .flat_map(|f| f.keys())
        .filter_map(|k| canonical_header(k))
        .collect();

    // Identify non-standard columns as metadata candidates:
    // columns not in the known set, not a presenter column, and not a ticket column.
    let presenter_col_indices: HashSet<u32> = presenter_cols.iter().map(|pc| pc.col).collect();
    let metadata_cols: Vec<(String, u32)> = raw_headers
        .iter()
        .enumerate()
        .filter_map(|(i, raw_h)| {
            if raw_h.is_empty() {
                return None;
            }
            let col = range.start_col + i as u32;
            if presenter_col_indices.contains(&col) || ticket_cols.contains(&col) {
                return None;
            }
            let is_known = canonical_headers
                .get(i)
                .and_then(|c| c.as_ref())
                .map(|c| known_canonical_headers.contains(c.as_str()))
                .unwrap_or(false);
            if is_known {
                None
            } else {
                Some((raw_h.clone(), col))
            }
        })
        .collect();

    let room_lookup: HashMap<String, &Room> = rooms
        .iter()
        .flat_map(|r| {
            let mut entries = vec![(r.short_name.to_lowercase(), r)];
            entries.push((r.long_name.to_lowercase(), r));
            if !r.hotel_room.is_empty() {
                entries.push((r.hotel_room.to_lowercase(), r));
            }
            entries
        })
        .collect();

    let type_lookup: HashMap<String, &PanelType> = panel_types
        .iter()
        .map(|(prefix, pt)| (prefix.to_lowercase(), pt))
        .collect();

    let mut presenter_map: HashMap<String, PresenterInfo> = HashMap::new();
    let mut panel_sets: IndexMap<String, PanelSet> = IndexMap::new();
    let mut timeline_entries: Vec<TimelineEntry> = Vec::new();

    let start_time_col = col_map.get(sc::START_TIME.canonical).copied();
    let end_time_col = col_map.get(sc::END_TIME.canonical).copied();
    let duration_col = col_map.get(sc::DURATION.canonical).copied();

    for row in (range.header_row + 1)..=range.end_row {
        let mut data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        for &col in &ticket_cols {
            if let Some(url) = extract_hyperlink_url(ws, col, row) {
                let header_idx = (col - range.start_col) as usize;
                if let Some(canon) = canonical_headers.get(header_idx).and_then(|c| c.as_ref()) {
                    data.insert(canon.clone(), url.clone());
                }
                if let Some(raw) = raw_headers.get(header_idx) {
                    if !raw.is_empty() {
                        data.insert(raw.clone(), url);
                    }
                }
            }
        }

        let raw_uniq_id = get_field_def(&data, &sc::UNIQ_ID).cloned();
        // A leading * means this row was soft-deleted by xlsx_update; strip it and mark deleted.
        let (uniq_id, is_deleted_row) = match raw_uniq_id {
            Some(ref s) if s.starts_with('*') => {
                (Some(s.trim_start_matches('*').to_string()), true)
            }
            other => (other, false),
        };
        let raw_name = match get_field_def(&data, &sc::NAME) {
            Some(n) => n.clone(),
            None => {
                continue;
            }
        };

        // Strip trailing part/session numbers from title
        let (name, title_part_num, title_session_num) = strip_title_suffix(&raw_name);

        let panel_id = match PanelId::parse(&uniq_id.as_deref().unwrap_or("")) {
            Some(pid) => pid,
            None => {
                // @TODO: Skip these records entirely, this was an old way to delete a panel from the schedule
                // Create a fake panel ID for rows without proper IDs
                // Use title-derived parts if available
                PanelId {
                    prefix: String::new(),
                    prefix_num: row,
                    part_num: title_part_num,
                    session_num: title_session_num,
                    suffix: None,
                }
            }
        };

        // @TODO: Check if panel id has already been used and if so try adding an alphabetical suffix starting with A until a unique id is found

        // Check for conflicts between title suffixes and Uniq ID parts
        let has_conflict = match (
            &panel_id.part_num,
            &panel_id.session_num,
            &title_part_num,
            &title_session_num,
        ) {
            (None, None, Some(_), Some(_)) => true, // ID has none, title has both
            (None, None, Some(_), None) => true,    // ID has none, title has part
            (None, None, None, Some(_)) => true,    // ID has none, title has session
            (Some(_id_part), None, None, Some(_)) => true, // ID has part, title has session
            (None, Some(_id_session), Some(_), None) => true, // ID has session, title has part
            (Some(id_part), Some(id_session), Some(title_part), Some(title_session)) => {
                id_part != title_part || id_session != title_session
            }
            (Some(id_part), None, Some(title_part), None) => id_part != title_part,
            (None, Some(id_session), None, Some(title_session)) => id_session != title_session,
            _ => false,
        };

        let start_time = parse_datetime_value(
            start_time_col.and_then(|c| get_cell_str(ws, c, row)),
            start_time_col.and_then(|c| get_cell_number(ws, c, row)),
        );

        // Keep start_time as None for unscheduled panels
        let start_time = start_time;

        let end_time_from_cell = parse_datetime_value(
            end_time_col.and_then(|c| get_cell_str(ws, c, row)),
            end_time_col.and_then(|c| get_cell_number(ws, c, row)),
        );
        let duration_minutes = parse_duration_value(
            duration_col.and_then(|c| get_cell_str(ws, c, row)),
            duration_col.and_then(|c| get_cell_number(ws, c, row)),
        );

        let (end_time, duration) = match (start_time, end_time_from_cell, duration_minutes) {
            (Some(st), Some(et), Some(_)) => {
                let diff = (et - st).num_minutes().max(0) as u32;
                (Some(et), Some(diff))
            }
            (Some(st), Some(et), None) => {
                let diff = (et - st).num_minutes().max(0) as u32;
                (Some(et), Some(diff))
            }
            (Some(st), None, Some(d)) => {
                let et = st + chrono::Duration::minutes(d as i64);
                (Some(et), Some(d))
            }
            (Some(st), None, None) => {
                // Panel has start time but no end time or duration
                // Default to 1 hour duration
                let et = st + chrono::Duration::hours(1);
                (Some(et), Some(60))
            }
            (None, Some(et), Some(d)) => {
                // Panel has end time and duration but no start time
                // Calculate start time from end time
                let _st = et - chrono::Duration::minutes(d as i64);
                (Some(et), Some(d))
            }
            (None, Some(et), None) => {
                // Panel has end time but no start time or duration
                // Default to 1 hour duration
                let _st = et - chrono::Duration::hours(1);
                (Some(et), Some(60))
            }
            (None, None, Some(d)) => {
                // Panel has duration but no start or end time
                // Keep as unscheduled (no times)
                (None, Some(d))
            }
            (None, None, None) => {
                // Panel is completely unscheduled
                (None, None)
            }
        };

        let room_name = get_field_def(&data, &sc::ROOM).cloned();
        let room_ids: Vec<u32> = if let Some(ref room_name) = room_name {
            room_name
                .split(',')
                .filter_map(|name| {
                    let trimmed = name.trim();
                    room_lookup.get(&trimmed.to_lowercase()).map(|r| r.uid)
                })
                .collect()
        } else {
            Vec::new()
        };

        let kind_raw = get_field_def(&data, &sc::KIND).cloned();
        let panel_type = if !panel_id.prefix.is_empty() {
            type_lookup.get(&panel_id.prefix.to_lowercase()).copied()
        } else {
            None
        };

        let panel_type = panel_type.or_else(|| {
            kind_raw.as_ref().and_then(|kr| {
                panel_types
                    .values()
                    .find(|pt| pt.kind.to_lowercase() == kr.to_lowercase())
            })
        });

        let cost_raw = get_field_def(&data, &sc::COST).cloned();
        let (cost, is_free, is_kids) = normalize_cost(cost_raw.as_ref());
        let is_full = get_field_def(&data, &sc::FULL)
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let mut credited_presenters: Vec<String> = Vec::new();
        let mut uncredited_presenters: Vec<String> = Vec::new();
        for pc in &presenter_cols {
            let cell_str = match get_cell_str(ws, pc.col, row) {
                Some(s) => s,
                None => continue,
            };

            let rank = pc.rank.as_str();

            // For Other columns, split by commas; for Named, each chunk is the whole cell
            let chunks: Vec<String> = match &pc.header {
                PresenterHeader::Other => split_presenter_names(&cell_str),
                PresenterHeader::Named(_) => vec![cell_str],
            };

            for chunk in chunks {
                let (uid, is_credited) =
                    match parse_presenter_data(&pc.header, rank, &chunk, &mut presenter_map) {
                        Some(r) => r,
                        None => continue,
                    };

                if is_credited {
                    if !credited_presenters.contains(&uid) {
                        credited_presenters.push(uid);
                    }
                } else if !uncredited_presenters.contains(&uid) {
                    uncredited_presenters.push(uid);
                }
            }
        }

        // Fallback: Presenter/Presenters column
        if credited_presenters.is_empty() && uncredited_presenters.is_empty() {
            if let Some(presenter_raw) =
                get_field(&data, &["Presenter", "Presenters", "Presenter_s", "Person"])
            {
                for part in split_presenter_names(presenter_raw) {
                    presenter_map
                        .entry(part.clone())
                        .or_insert_with(|| PresenterInfo {
                            rank: PresenterRank::from_str("fan_panelist"),
                            is_member: PresenterMember::NotMember,
                            is_grouped: PresenterGroup::NotGroup,
                        });
                    credited_presenters.push(part);
                }
            }
        }

        let panel_type_uid = panel_type.map(|pt| pt.prefix.clone()).or_else(|| {
            if !panel_id.prefix.is_empty() {
                Some(panel_id.prefix.clone())
            } else {
                None
            }
        });

        // Check if this is a timeline entry
        if let Some(pt) = panel_type {
            if pt.is_timeline {
                // Get the note field for timeline entries
                let note = get_field_def(&data, &sc::NOTE).cloned();

                // Create a TimelineEntry instead of a regular Panel
                // Timeline entries require a start time, so skip if none
                let Some(st) = start_time else {
                    continue;
                };

                let timeline_entry = TimelineEntry {
                    id: uniq_id.unwrap_or_else(|| format!("TL{}", row)).to_string(),
                    start_time: Some(st),
                    description: name.clone(),
                    panel_type: panel_type_uid.clone(),
                    note,
                    metadata: None,
                    source: Some(SourceInfo {
                        file_path: Some(file_path.to_string()),
                        sheet_name: Some(range.sheet_name.clone()),
                        row_index: Some(row as u32),
                    }),
                    change_state: ChangeState::Unchanged,
                };
                timeline_entries.push(timeline_entry);
                continue; // Skip regular panel processing for timeline entries
            }
        }

        // Get other fields
        let description = get_field_def(&data, &sc::DESCRIPTION).cloned();
        let note = get_field_def(&data, &sc::NOTE).cloned();
        let prereq = get_field_def(&data, &sc::PREREQ).cloned();
        let alt_panelist = get_field_def(&data, &sc::ALT_PANELIST).cloned();
        let capacity = get_field_def(&data, &sc::CAPACITY).cloned();
        let difficulty = get_field_def(&data, &sc::DIFFICULTY).cloned();
        let ticket_url = get_field_def(&data, &sc::TICKET_SALE).cloned();
        let simple_tix_event = get_field_def(&data, &sc::SIMPLE_TIX_EVENT).cloned();
        let hide_panelist = get_field_def(&data, &sc::HIDE_PANELIST)
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let seats_sold = get_field_def(&data, &sc::SEATS_SOLD).and_then(|s| s.parse::<u32>().ok());
        let pre_reg_max = get_field_def(&data, &sc::PRE_REG_MAX).cloned();
        let notes_non_printing = get_field_def(&data, &sc::NOTES_NON_PRINTING).cloned();
        let workshop_notes = get_field_def(&data, &sc::WORKSHOP_NOTES).cloned();
        let power_needs = get_field_def(&data, &sc::POWER_NEEDS).cloned();
        let sewing_machines = get_field_def(&data, &sc::SEWING_MACHINES)
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let av_notes = get_field_def(&data, &sc::AV_NOTES).cloned();
        let have_ticket_image = get_field_def(&data, &sc::HAVE_TICKET_IMAGE).map(|s| is_truthy(s));

        // Find or create the PanelSet for this base panel
        let base_id = panel_id.base_id();
        let ps = panel_sets
            .entry(base_id.clone())
            .or_insert_with(|| PanelSet::new(&base_id));

        // Build the flat panel's full ID from the Uniq_ID, or synthesize one
        let session_id = if let Some(ref id) = uniq_id {
            id.clone()
        } else {
            format!("{}-session-{}", base_id, ps.panels.len())
        };

        // Create the flat panel with all fields from this row
        let mut panel = Panel::new(&session_id, &base_id);
        panel.name = name.clone();
        panel.part_num = panel_id.part_num;
        panel.session_num = panel_id.session_num;
        panel.panel_type = panel_type_uid.clone();
        panel.room_ids = room_ids;

        // Set timing based on what we have using TimeRange constructors
        // Priority: start+duration > start+end > start only > duration only
        if let (Some(start), Some(duration_minutes)) = (start_time, duration) {
            // Use start time and duration (highest priority)
            let duration = chrono::Duration::minutes(duration_minutes as i64);
            if let Ok(timerange) = crate::data::time::TimeRange::new_scheduled(start, duration) {
                // Check for end time conflict if end_time was also specified
                if let Some(specified_end) = end_time {
                    let effective_end = timerange.effective_end_time();
                    if let Some(effective) = effective_end {
                        if effective != specified_end {
                            // TODO: Record conflict - specified end time differs from calculated
                            // This could be stored in a conflicts array or logged
                        }
                    }
                }
                panel.timing = timerange;
            } else {
                // Invalid duration, try to fall back to end_time if available
                if let Some(end) = end_time {
                    if let Ok(timerange) = crate::data::time::TimeRange::from_start_end(start, end)
                    {
                        panel.timing = timerange;
                    } else {
                        // Both duration and end_time invalid, fall back to start only
                        panel.timing = crate::data::time::TimeRange::UnspecifiedWithStart(start);
                    }
                } else {
                    // No end_time available, fall back to start only
                    panel.timing = crate::data::time::TimeRange::UnspecifiedWithStart(start);
                }
            }
        } else if let (Some(start), Some(end)) = (start_time, end_time) {
            // Use start and end time to create a complete TimeRange
            if let Ok(timerange) = crate::data::time::TimeRange::from_start_end(start, end) {
                panel.timing = timerange;
            } else {
                // Invalid range (end before start), fall back to start only
                panel.timing = crate::data::time::TimeRange::UnspecifiedWithStart(start);
            }
        } else if let Some(start) = start_time {
            // Only start time available
            panel.timing = crate::data::time::TimeRange::UnspecifiedWithStart(start);
        } else if let Some(duration_minutes) = duration {
            // Only duration available
            let duration = chrono::Duration::minutes(duration_minutes as i64);
            panel.timing = crate::data::time::TimeRange::UnspecifiedWithDuration(duration);
        }
        panel.cost = cost.clone();
        panel.is_free = is_free;
        panel.is_kids = is_kids;
        panel.is_full = is_full;
        panel.capacity = capacity;
        panel.seats_sold = seats_sold;
        panel.pre_reg_max = pre_reg_max;
        panel.ticket_url = ticket_url;
        panel.simple_tix_event = simple_tix_event;
        panel.have_ticket_image = have_ticket_image;
        panel.hide_panelist = hide_panelist;
        panel.difficulty = difficulty;
        panel.description = description;
        panel.note = note;
        panel.prereq = prereq;
        panel.alt_panelist = alt_panelist;
        panel.credited_presenters = credited_presenters;
        panel.uncredited_presenters = uncredited_presenters;
        panel.notes_non_printing = notes_non_printing;
        panel.workshop_notes = workshop_notes;
        panel.power_needs = power_needs;
        panel.sewing_machines = sewing_machines;
        panel.av_notes = av_notes;
        panel.source = Some(SourceInfo {
            file_path: Some(file_path.to_string()),
            sheet_name: Some(range.sheet_name.clone()),
            row_index: Some(row),
        });
        if is_deleted_row {
            panel.change_state = ChangeState::Deleted;
        }

        // Add conflict if detected
        if has_conflict {
            let conflict_details = format!(
                "Title suffix (Part:{}, Session:{}) doesn't match Uniq ID (Part:{}, Session:{})",
                title_part_num.unwrap_or(0),
                title_session_num.unwrap_or(0),
                panel_id.part_num.unwrap_or(0),
                panel_id.session_num.unwrap_or(0)
            );
            panel.conflicts.push(EventConflict {
                conflict_type: "title_id_mismatch".to_string(),
                details: Some(conflict_details),
                conflict_event_id: None,
            });
        }

        // Collect metadata from non-standard columns
        if !metadata_cols.is_empty() {
            let metadata: IndexMap<String, ExtraValue> = metadata_cols
                .iter()
                .filter_map(|(raw_h, col)| {
                    let cell = ws.get_cell((*col, row))?;
                    let formula = cell.get_formula().to_string();
                    let str_val = ws.get_value((*col, row)).trim().to_string();
                    if str_val.is_empty() && formula.is_empty() {
                        return None;
                    }
                    let value = if !formula.is_empty() {
                        ExtraValue::Formula(FormulaValue {
                            formula,
                            value: str_val,
                        })
                    } else {
                        ExtraValue::String(str_val)
                    };
                    Some((raw_h.clone(), value))
                })
                .collect();
            if !metadata.is_empty() {
                panel.metadata = metadata;
            }
        }

        ps.panels.push(panel);
    }

    Ok((panel_sets, presenter_map, timeline_entries))
}

fn extract_hyperlink_url(ws: &umya_spreadsheet::Worksheet, col: u32, row: u32) -> Option<String> {
    let cell = ws.get_cell((col, row))?;

    if let Some(hyperlink) = cell.get_hyperlink() {
        let url = hyperlink.get_url();
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }

    let formula = cell.get_formula();
    if !formula.is_empty() {
        if let Some(url) = parse_hyperlink_formula(formula) {
            return Some(url);
        }
    }

    None
}

/// Parse `HYPERLINK("url","text")` (without leading `=`) and return the URL.
fn parse_hyperlink_formula(formula: &str) -> Option<String> {
    let upper = formula.to_uppercase();
    if !upper.starts_with("HYPERLINK(") {
        return None;
    }
    let re = Regex::new(r#"(?i)^HYPERLINK\s*\(\s*"([^"]+)""#).ok()?;
    re.captures(formula).map(|caps| caps[1].to_string())
}

fn excel_serial_to_naive_datetime(serial: f64) -> Option<NaiveDateTime> {
    let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let days = serial.floor() as i64;
    let fraction = serial - serial.floor();
    let seconds_in_day = (fraction * 86400.0).round() as i64;

    let date = epoch + chrono::Duration::days(days);
    let time = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
        seconds_in_day.clamp(0, 86399) as u32,
        0,
    )?;
    Some(NaiveDateTime::new(date, time))
}

fn parse_datetime_value(str_val: Option<String>, num_val: Option<f64>) -> Option<NaiveDateTime> {
    if let Some(s) = str_val {
        if let Some(dt) = time::parse_datetime(&s) {
            return Some(dt);
        }
    }
    if let Some(f) = num_val {
        return excel_serial_to_naive_datetime(f);
    }
    None
}

fn parse_duration_value(str_val: Option<String>, num_val: Option<f64>) -> Option<u32> {
    if let Some(s) = str_val {
        if let Some(d) = time::parse_duration_str(&s) {
            return Some(d);
        }
    }
    if let Some(f) = num_val {
        if f > 0.0 && f < 1.0 {
            return Some((f * 24.0 * 60.0).round() as u32);
        }
        if f >= 1.0 {
            return Some(f as u32);
        }
    }
    None
}

fn normalize_cost(text: Option<&String>) -> (Option<String>, bool, bool) {
    let text = match text {
        Some(t) => t.trim(),
        None => return (None, true, false),
    };

    if text.is_empty() || text == "*" {
        return (None, true, false);
    }

    let lower = text.to_lowercase();
    if lower == "free" || lower == "n/a" || lower == "nothing" || lower == "$0" || lower == "$0.00"
    {
        return (None, true, false);
    }
    if lower == "kids" {
        return (None, true, true);
    }

    (Some(text.to_string()), false, false)
}

fn split_presenter_names(text: &str) -> Vec<String> {
    let re = Regex::new(r"\s*(?:,\s*(?:and\s+)?|\band\s+)").expect("valid regex");
    re.split(text)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Strip trailing part/session numbers from a panel title
///
/// Removes patterns like:
/// - " (Session #)"
/// - " (Part #)"  
/// - " (Part #, Session #)"
///
/// Returns the cleaned title and a tuple of (part_num, session_num) if found
fn strip_title_suffix(title: &str) -> (String, Option<u32>, Option<u32>) {
    let re = Regex::new(r"(?i)\s*\((?:Part\s+(\d+)(?:,\s*Session\s+(\d+))?|Session\s+(\d+))\)\s*$")
        .expect("valid regex");

    if let Some(caps) = re.captures(title) {
        let base_title = title[..caps.get(0).unwrap().start()].trim().to_string();

        let part_num = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let session_num = caps
            .get(2)
            .or_else(|| caps.get(3))
            .and_then(|m| m.as_str().parse().ok());

        (base_title, part_num, session_num)
    } else {
        (title.to_string(), None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_title_suffix() {
        // Test basic suffix removal
        let (title, part, session) = strip_title_suffix("My Panel (Part 1)");
        assert_eq!(title, "My Panel");
        assert_eq!(part, Some(1));
        assert_eq!(session, None);

        let (title, part, session) = strip_title_suffix("My Panel (Session 2)");
        assert_eq!(title, "My Panel");
        assert_eq!(part, None);
        assert_eq!(session, Some(2));

        let (title, part, session) = strip_title_suffix("My Panel (Part 3, Session 2)");
        assert_eq!(title, "My Panel");
        assert_eq!(part, Some(3));
        assert_eq!(session, Some(2));

        // Test no suffix
        let (title, part, session) = strip_title_suffix("My Panel");
        assert_eq!(title, "My Panel");
        assert_eq!(part, None);
        assert_eq!(session, None);

        // Test with extra spaces
        let (title, part, session) = strip_title_suffix("My Panel   (Part 1)   ");
        assert_eq!(title, "My Panel");
        assert_eq!(part, Some(1));
        assert_eq!(session, None);

        // Test case insensitive
        let (title, part, session) = strip_title_suffix("My Panel (part 1, session 2)");
        assert_eq!(title, "My Panel");
        assert_eq!(part, Some(1));
        assert_eq!(session, Some(2));
    }

    #[test]
    fn test_normalize_cost() {
        assert_eq!(normalize_cost(None), (None, true, false));
        assert_eq!(
            normalize_cost(Some(&"Free".to_string())),
            (None, true, false)
        );
        assert_eq!(
            normalize_cost(Some(&"Kids".to_string())),
            (None, true, true)
        );
        assert_eq!(
            normalize_cost(Some(&"$20.00".to_string())),
            (Some("$20.00".to_string()), false, false)
        );
    }

    #[test]
    fn test_split_presenter_names() {
        let names = split_presenter_names("Alice, Bob and Charlie");
        assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);

        let names = split_presenter_names("Alice, and Bob");
        assert_eq!(names, vec!["Alice", "Bob"]);

        let names = split_presenter_names("Single Name");
        assert_eq!(names, vec!["Single Name"]);
    }

    #[test]
    fn test_parse_hyperlink_formula() {
        let url = parse_hyperlink_formula(
            r#"HYPERLINK("https://www.simpletix.com/e/fw001-tickets-219590","purchase")"#,
        );
        assert_eq!(
            url.as_deref(),
            Some("https://www.simpletix.com/e/fw001-tickets-219590")
        );
        assert!(parse_hyperlink_formula("SUM(A1:A2)").is_none());
        assert!(parse_hyperlink_formula("").is_none());
    }
}
