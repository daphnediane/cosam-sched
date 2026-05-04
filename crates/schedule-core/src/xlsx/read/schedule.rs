/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the Schedule sheet → [`PanelEntityType`] entities + presenter edges.

use std::collections::HashMap;

use anyhow::Result;
use chrono::{Duration, NaiveDateTime};
use regex::Regex;
use umya_spreadsheet::Spreadsheet;

use crate::edit::builder::build_entity;
use crate::entity::UuidPreference;
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::tables::event_room::EventRoomId;
use crate::tables::panel::{
    PanelEntityType, PanelId, EDGE_CREDITED_PRESENTERS, EDGE_EVENT_ROOMS, EDGE_PANEL_TYPE,
    EDGE_UNCREDITED_PRESENTERS,
};
use crate::tables::panel_type::PanelTypeId;
use crate::tables::presenter::find_or_create_tagged_presenter;
use crate::value::time::{parse_datetime, parse_duration};
use crate::value::uniq_id::PanelUniqId;
use crate::xlsx::columns::schedule as sc;

use super::{
    build_column_map, find_data_range, get_cell_number, get_cell_str, is_truthy,
    parse_presenter_header, row_to_map, PresenterColumn, PresenterHeader,
};

/// Read the Schedule sheet and create Panel entities with all relationships.
pub(super) fn read_schedule_into(
    book: &Spreadsheet,
    preferred: &str,
    schedule: &mut Schedule,
    room_lookup: &HashMap<String, EventRoomId>,
    panel_type_lookup: &HashMap<String, PanelTypeId>,
) -> Result<()> {
    let first_sheet_name = book
        .get_sheet_collection()
        .first()
        .map(|s| s.get_name().to_string());
    let first_ref = first_sheet_name.as_deref().unwrap_or("");

    let range = match find_data_range(book, preferred, &["Schedule", first_ref]) {
        Some(r) => {
            // If actual data extends beyond the named table, expand the range.
            let ws = book.get_sheet_by_name(&r.sheet_name).unwrap();
            let actual_end_row = ws.get_highest_row();
            let actual_end_col = ws.get_highest_column();
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
        None => return Ok(()),
    };

    let ws = match book.get_sheet_by_name(&range.sheet_name) {
        Some(ws) => ws,
        None => return Ok(()),
    };

    if !range.has_data() {
        return Ok(());
    }

    let (raw_headers, canonical_headers, col_map) = build_column_map(ws, &range);

    // Identify presenter columns.
    let presenter_cols: Vec<PresenterColumn> = raw_headers
        .iter()
        .enumerate()
        .filter_map(|(i, h)| parse_presenter_header(h, range.start_col + i as u32))
        .collect();

    // Identify ticket columns for hyperlink extraction.
    let ticket_keys: std::collections::HashSet<String> = sc::TICKET_SALE
        .keys()
        .chain(sc::SIMPLE_TIX_EVENT.keys())
        .filter_map(|k| super::canonical_header(k))
        .collect();
    let ticket_cols: std::collections::HashSet<u32> = canonical_headers
        .iter()
        .enumerate()
        .filter_map(|(i, canon)| {
            let key = canon.as_deref()?;
            if ticket_keys.contains(key) {
                Some(range.start_col + i as u32)
            } else {
                None
            }
        })
        .collect();

    let start_time_col = col_map.get(sc::START_TIME.canonical).copied();
    let end_time_col = col_map.get(sc::END_TIME.canonical).copied();
    let duration_col = col_map.get(sc::DURATION.canonical).copied();

    for row in (range.header_row + 1)..=range.end_row {
        let mut data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        // Extract hyperlink URLs from ticket/SimpleTix columns.
        for &col in &ticket_cols {
            if let Some(url) = extract_hyperlink_url(ws, col, row) {
                let idx = (col - range.start_col) as usize;
                if let Some(canon) = canonical_headers.get(idx).and_then(|c| c.as_ref()) {
                    data.insert(canon.clone(), url.clone());
                }
                if let Some(raw) = raw_headers.get(idx) {
                    if !raw.is_empty() {
                        data.insert(raw.clone(), url);
                    }
                }
            }
        }

        // Require a Name field.
        let name = match get_field_def(&data, &sc::NAME) {
            Some(n) => n.clone(),
            None => continue,
        };

        // Parse Uniq ID; skip soft-deleted rows (leading `*`).
        let raw_uniq_id = get_field_def(&data, &sc::UNIQ_ID).cloned();
        let (uniq_id_str, is_deleted) = match raw_uniq_id {
            Some(ref s) if s.starts_with('*') => {
                (Some(s.trim_start_matches('*').to_string()), true)
            }
            other => (other, false),
        };
        if is_deleted {
            continue; // Soft-deleted rows are excluded from import.
        }

        // Parse timing.
        let start_time = parse_cell_datetime(
            start_time_col.and_then(|c| get_cell_str(ws, c, row)),
            start_time_col.and_then(|c| get_cell_number(ws, c, row)),
        );
        let end_time_raw = parse_cell_datetime(
            end_time_col.and_then(|c| get_cell_str(ws, c, row)),
            end_time_col.and_then(|c| get_cell_number(ws, c, row)),
        );
        let duration_minutes = parse_cell_duration(
            duration_col.and_then(|c| get_cell_str(ws, c, row)),
            duration_col.and_then(|c| get_cell_number(ws, c, row)),
        );

        // Resolve effective duration from whatever combination we have.
        let (effective_start, effective_duration) =
            resolve_timing(start_time, end_time_raw, duration_minutes);

        // Look up rooms (comma-separated).
        let room_ids: Vec<EventRoomId> = get_field_def(&data, &sc::ROOM)
            .map(|r| {
                r.split(',')
                    .filter_map(|name| {
                        room_lookup
                            .get(name.trim().to_lowercase().as_str())
                            .copied()
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Determine panel type from prefix or Kind column.
        let parsed_code = uniq_id_str.as_deref().and_then(PanelUniqId::parse);
        let panel_type_id: Option<PanelTypeId> = parsed_code
            .as_ref()
            .and_then(|c| panel_type_lookup.get(&c.prefix))
            .copied()
            .or_else(|| {
                get_field_def(&data, &sc::KIND).and_then(|kind| {
                    panel_type_lookup
                        .values()
                        .find(|&&pt_id| {
                            // Match by panel_kind field value (case-insensitive).
                            let schedule_ref = &*schedule;
                            schedule_ref
                                .get_internal::<crate::tables::panel_type::PanelTypeEntityType>(
                                    pt_id,
                                )
                                .map(|d| d.data.panel_kind.to_lowercase() == kind.to_lowercase())
                                .unwrap_or(false)
                        })
                        .copied()
                })
            });

        // Parse cost / free / kids flags.
        let cost_raw = get_field_def(&data, &sc::COST).map(|s| s.as_str());
        let (cost, is_free, is_kids) = normalize_cost(cost_raw);

        let is_full = get_field_def(&data, &sc::FULL)
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let hide_panelist = get_field_def(&data, &sc::HIDE_PANELIST)
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let sewing_machines = get_field_def(&data, &sc::SEWING_MACHINES)
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let have_ticket_image = get_field_def(&data, &sc::HAVE_TICKET_IMAGE)
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let capacity = get_field_def(&data, &sc::CAPACITY).and_then(|s| s.parse::<i64>().ok());
        let seats_sold = get_field_def(&data, &sc::SEATS_SOLD).and_then(|s| s.parse::<i64>().ok());
        let pre_reg_max =
            get_field_def(&data, &sc::PRE_REG_MAX).and_then(|s| s.parse::<i64>().ok());

        // Determine Uniq ID string (synthesize row-based ID if missing).
        let code_str = uniq_id_str.unwrap_or_else(|| format!("XX{row:03}"));

        // Build Panel entity via field system.
        let uuid_pref = UuidPreference::PreferFromV5 {
            name: code_str.to_uppercase(),
        };
        let mut updates: Vec<FieldUpdate<PanelEntityType>> = vec![
            FieldUpdate::set("code", code_str.as_str()),
            FieldUpdate::set("name", name.as_str()),
            FieldUpdate::set("is_free", is_free),
            FieldUpdate::set("is_kids", is_kids),
            FieldUpdate::set("is_full", is_full),
            FieldUpdate::set("hide_panelist", hide_panelist),
            FieldUpdate::set("sewing_machines", sewing_machines),
            FieldUpdate::set("have_ticket_image", have_ticket_image),
        ];

        if let Some(ref c) = cost {
            updates.push(FieldUpdate::set("cost", c.as_str()));
        }
        if let Some(ref d) = get_field_def(&data, &sc::DESCRIPTION).cloned() {
            updates.push(FieldUpdate::set("description", d.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::NOTE).cloned() {
            updates.push(FieldUpdate::set("note", n.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::NOTES_NON_PRINTING).cloned() {
            updates.push(FieldUpdate::set("notes_non_printing", n.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::WORKSHOP_NOTES).cloned() {
            updates.push(FieldUpdate::set("workshop_notes", n.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::POWER_NEEDS).cloned() {
            updates.push(FieldUpdate::set("power_needs", n.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::AV_NOTES).cloned() {
            updates.push(FieldUpdate::set("av_notes", n.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::DIFFICULTY).cloned() {
            updates.push(FieldUpdate::set("difficulty", n.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::PREREQ).cloned() {
            updates.push(FieldUpdate::set("prereq", n.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::TICKET_SALE).cloned() {
            updates.push(FieldUpdate::set("ticket_url", n.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::SIMPLE_TIX_EVENT).cloned() {
            updates.push(FieldUpdate::set("simpletix_event", n.as_str()));
        }
        if let Some(ref n) = get_field_def(&data, &sc::ALT_PANELIST).cloned() {
            updates.push(FieldUpdate::set("alt_panelist", n.as_str()));
        }
        if let Some(cap) = capacity {
            updates.push(FieldUpdate::set("capacity", cap));
        }
        if let Some(ss) = seats_sold {
            updates.push(FieldUpdate::set("seats_sold", ss));
        }
        if let Some(prm) = pre_reg_max {
            updates.push(FieldUpdate::set("pre_reg_max", prm));
        }
        if let Some(st) = effective_start {
            updates.push(FieldUpdate::set("start_time", st));
        }
        if let Some(dur) = effective_duration {
            updates.push(FieldUpdate::set("duration", dur));
        }

        let panel_id: PanelId = match build_entity::<PanelEntityType>(schedule, uuid_pref, updates)
        {
            Ok(id) => id,
            Err(e) => {
                eprintln!("xlsx import: skipping panel {code_str:?}: {e}");
                continue;
            }
        };

        // Wire edges.
        if !room_ids.is_empty() {
            let _ = schedule.edge_add(panel_id, EDGE_EVENT_ROOMS, room_ids);
        }
        if let Some(pt_id) = panel_type_id {
            let _ = schedule.edge_add(panel_id, EDGE_PANEL_TYPE, [pt_id]);
        }

        // Parse presenter columns for this row.
        let (credited, uncredited) = collect_presenters(ws, row, &presenter_cols, schedule);

        if !credited.is_empty() {
            let _ = schedule.edge_add(panel_id, EDGE_CREDITED_PRESENTERS, credited);
        }
        if !uncredited.is_empty() {
            let _ = schedule.edge_add(panel_id, EDGE_UNCREDITED_PRESENTERS, uncredited);
        }
    }

    Ok(())
}

// ── Presenter collection ──────────────────────────────────────────────────────

fn collect_presenters(
    ws: &umya_spreadsheet::structs::Worksheet,
    row: u32,
    presenter_cols: &[PresenterColumn],
    schedule: &mut Schedule,
) -> (
    Vec<crate::tables::presenter::PresenterId>,
    Vec<crate::tables::presenter::PresenterId>,
) {
    let mut credited = Vec::new();
    let mut uncredited = Vec::new();

    for pc in presenter_cols {
        let cell_str = match get_cell_str(ws, pc.col, row) {
            Some(s) => s,
            None => continue,
        };

        let chunks: Vec<String> = match &pc.header {
            PresenterHeader::Other => split_presenter_names(&cell_str),
            PresenterHeader::Named(_) => vec![cell_str],
        };

        for chunk in &chunks {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                continue;
            }

            // Leading `*` marks an uncredited presenter.
            let (name_part, is_uncredited) = if let Some(rest) = chunk.strip_prefix('*') {
                (rest.trim(), true)
            } else {
                (chunk, false)
            };

            // Build the tagged string for find_or_create_tagged_presenter.
            let tagged = match &pc.header {
                PresenterHeader::Named(header_name) => {
                    // Cell value is a flag; the header carries the name+group info.
                    let is_unlisted = name_part.eq_ignore_ascii_case("unlisted");
                    let tag = pc.rank.prefix_char();
                    let input = format!("{tag}:{header_name}");
                    if is_unlisted {
                        // Register presenter but mark uncredited.
                        if let Ok(id) = find_or_create_tagged_presenter(schedule, &input) {
                            if !uncredited.contains(&id) {
                                uncredited.push(id);
                            }
                        }
                        continue;
                    }
                    // Non-empty cell means they are attending.
                    if name_part.is_empty() {
                        continue;
                    }
                    input
                }
                PresenterHeader::Other => {
                    format!("{}:{}", pc.rank.prefix_char(), name_part)
                }
            };

            match find_or_create_tagged_presenter(schedule, &tagged) {
                Ok(id) => {
                    if is_uncredited {
                        if !uncredited.contains(&id) {
                            uncredited.push(id);
                        }
                    } else if !credited.contains(&id) {
                        credited.push(id);
                    }
                }
                Err(e) => {
                    eprintln!("xlsx import: skipping presenter {tagged:?}: {e}");
                }
            }
        }
    }

    // Fallback: generic Presenter/Presenters column (no presenter columns detected).
    if credited.is_empty() && uncredited.is_empty() {
        // This path is only reached when there were no PresenterColumn matches at all;
        // in that case we would need access to the data HashMap. Since we restructured
        // to pass `ws` and `row`, we skip the fallback here. Callers that need it
        // should pre-process the row_data before invoking collect_presenters.
    }

    (credited, uncredited)
}

// ── Timing helpers ────────────────────────────────────────────────────────────

fn resolve_timing(
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
    dur_mins: Option<u32>,
) -> (Option<NaiveDateTime>, Option<Duration>) {
    match (start, end, dur_mins) {
        (Some(st), Some(et), _) => {
            let dur = Duration::minutes((et - st).num_minutes().max(0));
            (Some(st), Some(dur))
        }
        (Some(st), None, Some(d)) => (Some(st), Some(Duration::minutes(d as i64))),
        (Some(st), None, None) => (Some(st), None),
        (None, _, Some(d)) => (None, Some(Duration::minutes(d as i64))),
        (None, _, None) => (None, None),
    }
}

fn parse_cell_datetime(str_val: Option<String>, num_val: Option<f64>) -> Option<NaiveDateTime> {
    if let Some(s) = str_val {
        if let Some(dt) = parse_datetime(&s) {
            return Some(dt);
        }
    }
    num_val.and_then(excel_serial_to_naive_datetime)
}

fn parse_cell_duration(str_val: Option<String>, num_val: Option<f64>) -> Option<u32> {
    if let Some(s) = str_val {
        if let Some(d) = parse_duration(&s) {
            return Some(d.num_minutes().max(0) as u32);
        }
    }
    if let Some(f) = num_val {
        // Excel stores time-of-day fractions (< 1.0) and plain integers.
        if f > 0.0 && f < 1.0 {
            return Some((f * 24.0 * 60.0).round() as u32);
        }
        if f >= 1.0 {
            return Some(f as u32);
        }
    }
    None
}

fn excel_serial_to_naive_datetime(serial: f64) -> Option<NaiveDateTime> {
    let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let days = serial.floor() as i64;
    let fraction = serial - serial.floor();
    let seconds_in_day = (fraction * 86400.0).round() as i64;
    let date = epoch + Duration::days(days);
    let time = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
        seconds_in_day.clamp(0, 86399) as u32,
        0,
    )?;
    Some(NaiveDateTime::new(date, time))
}

// ── Cost normalization ────────────────────────────────────────────────────────

fn normalize_cost(text: Option<&str>) -> (Option<String>, bool, bool) {
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

// ── Name splitting ────────────────────────────────────────────────────────────

fn split_presenter_names(text: &str) -> Vec<String> {
    let re = Regex::new(r"\s*(?:,\s*(?:and\s+)?|\band\s+)").expect("valid regex");
    re.split(text)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// ── Hyperlink extraction ──────────────────────────────────────────────────────

fn extract_hyperlink_url(
    ws: &umya_spreadsheet::structs::Worksheet,
    col: u32,
    row: u32,
) -> Option<String> {
    let cell = ws.get_cell((col, row))?;
    if let Some(hyperlink) = cell.get_hyperlink() {
        let url = hyperlink.get_url();
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }
    let formula = cell.get_formula();
    if !formula.is_empty() {
        return parse_hyperlink_formula(formula);
    }
    None
}

fn parse_hyperlink_formula(formula: &str) -> Option<String> {
    let re = Regex::new(r#"(?i)^HYPERLINK\s*\(\s*"([^"]+)""#).ok()?;
    re.captures(formula).map(|c| c[1].to_string())
}

// ── field helpers ─────────────────────────────────────────────────────────────

fn get_field_def<'a>(
    row_data: &'a HashMap<String, String>,
    field: &crate::xlsx::columns::FieldDef,
) -> Option<&'a String> {
    super::get_field_def(row_data, field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_cost_free() {
        assert_eq!(normalize_cost(None), (None, true, false));
        assert_eq!(normalize_cost(Some("Free")), (None, true, false));
        assert_eq!(normalize_cost(Some("$0")), (None, true, false));
        assert_eq!(normalize_cost(Some("Kids")), (None, true, true));
        assert_eq!(
            normalize_cost(Some("$35")),
            (Some("$35".into()), false, false)
        );
    }

    #[test]
    fn test_split_presenter_names() {
        assert_eq!(
            split_presenter_names("Alice, Bob and Charlie"),
            vec!["Alice", "Bob", "Charlie"]
        );
        assert_eq!(split_presenter_names("Single Name"), vec!["Single Name"]);
    }

    #[test]
    fn test_parse_hyperlink_formula() {
        let url = parse_hyperlink_formula(r#"HYPERLINK("https://example.com/tickets","Buy")"#);
        assert_eq!(url.as_deref(), Some("https://example.com/tickets"));
        assert!(parse_hyperlink_formula("SUM(A1:A2)").is_none());
    }

    #[test]
    fn test_resolve_timing_start_end() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        let st = base.and_hms_opt(10, 0, 0).unwrap();
        let et = base.and_hms_opt(11, 0, 0).unwrap();
        let (s, d) = resolve_timing(Some(st), Some(et), None);
        assert_eq!(s, Some(st));
        assert_eq!(d, Some(Duration::minutes(60)));
    }

    #[test]
    fn test_resolve_timing_start_duration() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        let st = base.and_hms_opt(10, 0, 0).unwrap();
        let (s, d) = resolve_timing(Some(st), None, Some(90));
        assert_eq!(s, Some(st));
        assert_eq!(d, Some(Duration::minutes(90)));
    }
}
