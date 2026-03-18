/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use regex::Regex;
use umya_spreadsheet::structs::Worksheet;
use umya_spreadsheet::Spreadsheet;

use super::event::Event;
use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::room::Room;
use super::schedule::{Meta, Schedule};
use super::source_info::{ChangeState, ImportedSheetPresence, SourceInfo};
use super::timeline::{TimeType, TimelineEntry};

pub struct XlsxImportOptions {
    pub title: String,
    pub schedule_table: String,
    pub rooms_table: String,
    pub panel_types_table: String,
}

impl Default for XlsxImportOptions {
    fn default() -> Self {
        Self {
            title: "Event Schedule".to_string(),
            schedule_table: "Schedule".to_string(),
            rooms_table: "RoomMap".to_string(),
            panel_types_table: "Prefix".to_string(),
        }
    }
}

fn convert_split_types_to_timeline(
    panel_types: &[PanelType],
    events: &[Event],
) -> (Vec<PanelType>, Vec<TimeType>, Vec<TimelineEntry>, Vec<Event>) {
    let mut time_types = Vec::new();
    let mut timeline = Vec::new();
    let mut filtered_panel_types = panel_types.to_vec();
    let mut split_uids: std::collections::HashSet<String> = std::collections::HashSet::new();

    let split_types: Vec<_> = panel_types
        .iter()
        .filter(|pt| is_split_prefix(&pt.prefix))
        .collect();

    for split_type in split_types {
        let time_type = TimeType {
            uid: TimeType::uid_from_prefix(&split_type.prefix),
            prefix: split_type.prefix.clone(),
            kind: split_type.kind.clone(),
            source: None,
            change_state: ChangeState::Converted,
        };
        time_types.push(time_type);
        split_uids.insert(split_type.effective_uid());

        let split_events: Vec<_> = events
            .iter()
            .filter(|e| {
                e.panel_type
                    .as_ref()
                    .map(|pt| pt == &split_type.effective_uid())
                    .unwrap_or(false)
            })
            .collect();

        for (i, event) in split_events.iter().enumerate() {
            let timeline_entry = TimelineEntry {
                id: format!("{}{:02}", split_type.prefix, i + 1),
                start_time: event.start_time.format("%Y-%m-%dT%H:%M:%S").to_string(),
                description: event.name.clone(),
                time_type: Some(TimeType::uid_from_prefix(&split_type.prefix)),
                note: event.note.clone(),
                source: None,
                change_state: ChangeState::Converted,
            };
            timeline.push(timeline_entry);
        }

        filtered_panel_types.retain(|pt| pt.prefix != split_type.prefix);
    }

    let filtered_events: Vec<Event> = events
        .iter()
        .filter(|e| {
            e.panel_type
                .as_ref()
                .map(|uid| !split_uids.contains(uid))
                .unwrap_or(true)
        })
        .cloned()
        .collect();

    (filtered_panel_types, time_types, timeline, filtered_events)
}

// Helper function to determine if a prefix indicates a split/time type
fn is_split_prefix(prefix: &str) -> bool {
    prefix.to_uppercase() == "SPLIT"
        || prefix.to_uppercase().starts_with("SP")
        || prefix.to_uppercase().starts_with("SPLIT")
}

pub fn import_xlsx(path: &Path, options: &XlsxImportOptions) -> Result<Schedule> {
    let book = umya_spreadsheet::reader::xlsx::read(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;

    let file_path_str = path.display().to_string();

    let rooms = read_rooms(&book, &options.rooms_table, &file_path_str)?;
    let panel_types = read_panel_types(&book, &options.panel_types_table, &file_path_str)?;
    let (events, presenters) = read_events(
        &book,
        &options.schedule_table,
        &rooms,
        &panel_types,
        &file_path_str,
    )?;

    let mut panel_types = panel_types;

    let imported_sheets = ImportedSheetPresence {
        has_room_map: !rooms.is_empty() && rooms.iter().any(|r| r.source.is_some()),
        has_panel_types: !panel_types.is_empty()
            && panel_types.iter().any(|pt| pt.source.is_some()),
        has_presenters: false,
        has_schedule: !events.is_empty(),
    };

    let mut used_prefixes = std::collections::HashSet::new();

    for event in &events {
        if let Some(ref panel_type_uid) = event.panel_type {
            if let Some(prefix) = panel_type_uid.strip_prefix("panel-type-") {
                used_prefixes.insert(prefix.to_uppercase());
            }
        }
    }

    for prefix in used_prefixes {
        if !panel_types.iter().any(|pt| pt.prefix == prefix) {
            let kind = format!("{} Panel", prefix);
            let is_workshop = prefix.ends_with('W');
            let is_break = prefix.to_uppercase().starts_with("BR");

            panel_types.push(PanelType {
                uid: Some(format!("panel-type-{}", prefix.to_lowercase())),
                prefix: prefix.clone(),
                kind,
                color: None,
                is_break,
                is_cafe: false,
                is_workshop,
                is_hidden: false,
                is_room_hours: false,
                bw_color: None,
                source: None,
                change_state: ChangeState::Converted,
            });
        }
    }

    let generated = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let (panel_types, time_types, timeline, events) =
        convert_split_types_to_timeline(&panel_types, &events);

    let mut schedule = Schedule {
        conflicts: Vec::new(),
        meta: Meta {
            title: options.title.clone(),
            generated,
            version: Some(4),
            generator: Some(format!("cosam-editor {}", env!("CARGO_PKG_VERSION"))),
            start_time: None,
            end_time: None,
        },
        timeline,
        events,
        rooms,
        panel_types,
        time_types,
        presenters,
        imported_sheets,
    };

    super::post_process::apply_schedule_parity(&mut schedule);
    Ok(schedule)
}

pub(super) fn canonical_header(header: &str) -> Option<String> {
    let trimmed = header.trim();
    if trimmed.is_empty() {
        return None;
    }
    let result = Regex::new(r"[\s/:().,]+")
        .expect("valid regex")
        .replace_all(trimmed, "_");
    let result = result.trim_matches('_');
    if result.is_empty() {
        return None;
    }
    Some(result.to_string())
}

/// Describes a contiguous data range in a worksheet (all coordinates are 1-based umya values).
/// `header_row` holds the column headers; data rows start at `header_row + 1`.
struct DataRange {
    sheet_name: String,
    start_col: u32,
    header_row: u32,
    end_col: u32,
    end_row: u32,
}

impl DataRange {
    fn has_data(&self) -> bool {
        self.end_row > self.header_row && self.end_col >= self.start_col
    }
}

/// Search order:
///   1. Named table matching `primary_name` (case-insensitive) on any sheet.
///   2. Sheet whose name matches `primary_name` (case-insensitive).
///   3. Sheets whose names match each entry in `fallback_sheet_names` in order.
fn find_data_range(
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

fn get_cell_str(ws: &Worksheet, col: u32, row: u32) -> Option<String> {
    let value = ws.get_value((col, row));
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn get_cell_number(ws: &Worksheet, col: u32, row: u32) -> Option<f64> {
    ws.get_value_number((col, row))
}

/// Extract a URL from a cell: checks the hyperlink relationship first, then parses a
/// `HYPERLINK("url","text")` formula. Returns `None` if no URL is found.
fn extract_hyperlink_url(ws: &Worksheet, col: u32, row: u32) -> Option<String> {
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

fn build_column_map(
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

fn get_field<'a>(row_data: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a String> {
    for key in keys {
        if let Some(val) = row_data.get(*key) {
            return Some(val);
        }
    }
    None
}

fn row_to_map(
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

fn read_rooms(book: &Spreadsheet, preferred: &str, file_path: &str) -> Result<Vec<Room>> {
    let range = match find_data_range(book, preferred, &["RoomMap", "Rooms"]) {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok(Vec::new());
    }

    let (raw_headers, canonical_headers, _col_map) = build_column_map(ws, &range);
    let mut rooms = Vec::new();
    let mut next_uid: u32 = 1;

    for row in (range.header_row + 1)..=range.end_row {
        let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        let short_name = get_field(&data, &["Room_Name", "Room", "Name"]).cloned();
        let long_name_raw = get_field(&data, &["Long_Name"]).cloned();
        let hotel_room = get_field(&data, &["Hotel_Room", "HotelRoom"])
            .cloned()
            .unwrap_or_default();

        let long_name = match long_name_raw {
            Some(ref ln) if ln != "#ERROR!" => ln.clone(),
            _ => hotel_room.clone(),
        };

        let short_name = match short_name {
            Some(s) => s,
            None => {
                if long_name.is_empty() {
                    next_uid += 1;
                    continue;
                }
                long_name.clone()
            }
        };

        let sort_key: u32 = get_field(&data, &["Sort_Key", "SortKey"])
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| f as u32)
            .unwrap_or(999);

        let uid = next_uid;
        next_uid += 1;

        rooms.push(Room {
            uid,
            short_name,
            long_name,
            hotel_room,
            sort_key,
            source: Some(SourceInfo {
                file_path: Some(file_path.to_string()),
                sheet_name: Some(range.sheet_name.clone()),
                row_index: Some(row),
            }),
            change_state: ChangeState::Unchanged,
        });
    }

    rooms.sort_by_key(|r| r.sort_key);
    Ok(rooms)
}

fn read_panel_types(
    book: &Spreadsheet,
    preferred: &str,
    file_path: &str,
) -> Result<Vec<PanelType>> {
    let range = match find_data_range(book, preferred, &["Prefix", "PanelTypes"]) {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok(Vec::new());
    }

    let (raw_headers, canonical_headers, _col_map) = build_column_map(ws, &range);
    let mut types = Vec::new();

    for row in (range.header_row + 1)..=range.end_row {
        let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        let prefix = match get_field(&data, &["Prefix"]) {
            Some(p) if !p.is_empty() => p.to_uppercase(),
            _ => continue,
        };

        let kind = get_field(&data, &["Panel_Kind", "PanelKind", "Kind"])
            .cloned()
            .unwrap_or_else(|| prefix.clone());

        let is_break = get_field(&data, &["Is_Break"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| kind.to_lowercase().starts_with("br"));

        let is_cafe = get_field(&data, &["Is_Cafe", "Is_Café"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| {
                let lower = kind.to_lowercase();
                lower == "cafe" || lower == "café"
            });

        let is_workshop = get_field(&data, &["Is_Workshop"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix.len() == 2 && prefix.ends_with('W'));

        let is_room_hours = get_field(&data, &["Is_Room_Hours", "IsRoomHours"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let _is_split = get_field(&data, &["Is_Split"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| {
                prefix == "SPLIT"
                    || prefix.to_uppercase().starts_with("SP")
                    || prefix.to_uppercase().starts_with("SPLIT")
            });

        let color = get_field(&data, &["Color"]).cloned();
        let bw_color = get_field(&data, &["BW", "Bw"]).cloned();

        let uid = Some(PanelType::uid_from_prefix(&prefix));

        let is_hidden = get_field(&data, &["Hidden"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        types.push(PanelType {
            uid,
            prefix,
            kind,
            color,
            is_break,
            is_cafe,
            is_workshop,
            is_hidden,
            is_room_hours,
            bw_color,
            source: Some(SourceInfo {
                file_path: Some(file_path.to_string()),
                sheet_name: Some(range.sheet_name.clone()),
                row_index: Some(row),
            }),
            change_state: ChangeState::Unchanged,
        });
    }

    Ok(types)
}

fn is_truthy(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    !lower.is_empty() && lower != "0" && lower != "no" && lower != "false"
}

#[derive(Debug)]
struct PresenterColumn {
    col: u32,
    rank: String,
    is_other: bool,
    is_named: bool,
    header_name: Option<String>,
    group_name: Option<String>,
    always_grouped: bool,
}

fn parse_presenter_header(header: &str, col: u32) -> Option<PresenterColumn> {
    let header = header.trim();
    if header.is_empty() {
        return None;
    }

    let rank_prefixes: HashMap<char, &str> = [
        ('g', "guest"),
        ('j', "judge"),
        ('s', "staff"),
        ('i', "invited_guest"),
        ('p', "fan_panelist"),
    ]
    .into_iter()
    .collect();

    // Kind:Name==Group format (always grouped)
    let re_double_eq = Regex::new(r"(?i)^([GJSIP]):(.+)==(.+)$").expect("valid regex");
    if let Some(caps) = re_double_eq.captures(header) {
        let prefix = caps[1].to_lowercase().chars().next()?;
        let rank = rank_prefixes.get(&prefix)?;
        let mut name = caps[2].to_string();
        name = name.trim_start_matches('<').trim().to_string();
        let group = caps[3].trim().to_string();
        if name.is_empty() {
            return None;
        }
        return Some(PresenterColumn {
            col,
            rank: rank.to_string(),
            is_other: false,
            is_named: true,
            header_name: Some(name),
            group_name: Some(group),
            always_grouped: true,
        });
    }

    // Kind:Name=Group format (member of group)
    let re_single_eq = Regex::new(r"(?i)^([GJSIP]):(.+)=(.+)$").expect("valid regex");
    if let Some(caps) = re_single_eq.captures(header) {
        let prefix = caps[1].to_lowercase().chars().next()?;
        let rank = rank_prefixes.get(&prefix)?;
        let mut name = caps[2].to_string();
        name = name.trim_start_matches('<').trim().to_string();
        let group = caps[3].trim().to_string();
        if name.is_empty() {
            return None;
        }
        return Some(PresenterColumn {
            col,
            rank: rank.to_string(),
            is_other: false,
            is_named: true,
            header_name: Some(name),
            group_name: Some(group),
            always_grouped: false,
        });
    }

    // Kind:Name or Kind:Other format
    let re_kind = Regex::new(r"(?i)^([GJSIP]):(.+)$").expect("valid regex");
    if let Some(caps) = re_kind.captures(header) {
        let prefix = caps[1].to_lowercase().chars().next()?;
        let rank = rank_prefixes.get(&prefix)?;
        let mut name = caps[2].to_string();
        if let Some(eq_pos) = name.find('=') {
            name.truncate(eq_pos);
        }
        name = name.trim_start_matches('<').trim().to_string();

        if name.to_lowercase() == "other" {
            return Some(PresenterColumn {
                col,
                rank: rank.to_string(),
                is_other: true,
                is_named: false,
                header_name: None,
                group_name: None,
                always_grouped: false,
            });
        }

        if name.is_empty() {
            return None;
        }

        return Some(PresenterColumn {
            col,
            rank: rank.to_string(),
            is_other: false,
            is_named: true,
            header_name: Some(name),
            group_name: None,
            always_grouped: false,
        });
    }

    // Legacy: letter + digits (g1, p2, etc.)
    let re_legacy = Regex::new(r"(?i)^([gjsip])(\d+)$").expect("valid regex");
    if let Some(caps) = re_legacy.captures(header) {
        let prefix = caps[1].to_lowercase().chars().next()?;
        let rank = rank_prefixes.get(&prefix)?;
        return Some(PresenterColumn {
            col,
            rank: rank.to_string(),
            is_other: false,
            is_named: false,
            header_name: None,
            group_name: None,
            always_grouped: false,
        });
    }

    // "Guest1", "Staff2", etc.
    let re_word = Regex::new(r"(?i)^(Guest|Judge|Staff|Invited|Panelist|Fan_Panelist)\s*(\d+)$")
        .expect("valid regex");
    if let Some(caps) = re_word.captures(header) {
        let first_char = caps[1].to_lowercase().chars().next()?;
        let rank = rank_prefixes.get(&first_char)?;
        return Some(PresenterColumn {
            col,
            rank: rank.to_string(),
            is_other: false,
            is_named: false,
            header_name: None,
            group_name: None,
            always_grouped: false,
        });
    }

    // "Fan Panelist" (2016 format: fan panelist other column)
    if header.to_lowercase() == "fan panelist" {
        return Some(PresenterColumn {
            col,
            rank: "fan_panelist".to_string(),
            is_other: true,
            is_named: false,
            header_name: None,
            group_name: None,
            always_grouped: false,
        });
    }

    // "Other Guests" → guest other, "Other Staff" → staff other
    let lower = header.to_lowercase();
    if lower == "other guests" || lower == "other guest" {
        return Some(PresenterColumn {
            col,
            rank: "guest".to_string(),
            is_other: true,
            is_named: false,
            header_name: None,
            group_name: None,
            always_grouped: false,
        });
    }
    if lower == "other staff" {
        return Some(PresenterColumn {
            col,
            rank: "staff".to_string(),
            is_other: true,
            is_named: false,
            header_name: None,
            group_name: None,
            always_grouped: false,
        });
    }

    // Generic "Other"/"Others" → fan_panelist other
    if lower.starts_with("other") {
        return Some(PresenterColumn {
            col,
            rank: "fan_panelist".to_string(),
            is_other: true,
            is_named: false,
            header_name: None,
            group_name: None,
            always_grouped: false,
        });
    }

    None
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
        if let Some(dt) = parse_datetime_string(&s) {
            return Some(dt);
        }
    }
    if let Some(f) = num_val {
        return excel_serial_to_naive_datetime(f);
    }
    None
}

fn parse_datetime_string(text: &str) -> Option<NaiveDateTime> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // ISO format
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }

    // M/DD/YYYY H:MM AM/PM
    let re_us =
        Regex::new(r"^(\d{1,2})/(\d{1,2})/(\d{4})\s+(\d{1,2}):(\d{2})(?::(\d{2}))?\s*(AM|PM)?$")
            .ok()?;

    if let Some(caps) = re_us.captures(text) {
        let month: u32 = caps[1].parse().ok()?;
        let day: u32 = caps[2].parse().ok()?;
        let year: i32 = caps[3].parse().ok()?;
        let mut hour: u32 = caps[4].parse().ok()?;
        let minute: u32 = caps[5].parse().ok()?;
        let second: u32 = caps
            .get(6)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);

        if let Some(ampm) = caps.get(7) {
            match ampm.as_str() {
                "PM" if hour < 12 => hour += 12,
                "AM" if hour == 12 => hour = 0,
                _ => {}
            }
        }

        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
        let time = chrono::NaiveTime::from_hms_opt(hour, minute, second)?;
        return Some(NaiveDateTime::new(date, time));
    }

    None
}

fn parse_duration_value(str_val: Option<String>, num_val: Option<f64>) -> Option<u32> {
    if let Some(s) = str_val {
        if let Some(d) = parse_duration_string(&s) {
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

fn parse_duration_string(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // H:MM or HH:MM
    let re_hm = Regex::new(r"^(\d+):(\d{1,2})$").ok()?;
    if let Some(caps) = re_hm.captures(text) {
        let hours: u32 = caps[1].parse().ok()?;
        let minutes: u32 = caps[2].parse().ok()?;
        return Some(hours * 60 + minutes);
    }

    // Plain number = minutes
    if let Ok(minutes) = text.parse::<f64>() {
        return Some(minutes as u32);
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

fn read_events(
    book: &Spreadsheet,
    preferred: &str,
    rooms: &[Room],
    panel_types: &[PanelType],
    file_path: &str,
) -> Result<(Vec<Event>, Vec<Presenter>)> {
    let first_sheet_name = book
        .get_sheet_collection()
        .first()
        .map(|s| s.get_name().to_string());
    let first_sheet_ref: &str = first_sheet_name.as_deref().unwrap_or("");
    let range = match find_data_range(book, preferred, &["Schedule", first_sheet_ref]) {
        Some(r) => r,
        None => return Ok((Vec::new(), Vec::new())),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok((Vec::new(), Vec::new()));
    }

    let (raw_headers, canonical_headers, col_map) = build_column_map(ws, &range);

    let ticket_cols: std::collections::HashSet<u32> = raw_headers
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            let lower = h.to_lowercase();
            if lower == "ticket_sale"
                || lower == "ticketsale"
                || lower == "ticket sale"
                || lower == "simpletix_event"
                || lower == "simpletixevent"
                || lower == "simpletix event"
            {
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
        .map(|pt| (pt.prefix.to_lowercase(), pt))
        .collect();

    struct PresenterInfo {
        rank: String,
        groups: Vec<String>,
        always_grouped: bool,
    }
    let mut presenter_map: HashMap<String, PresenterInfo> = HashMap::new();
    let mut group_members: HashMap<String, Vec<String>> = HashMap::new();
    let mut events = Vec::new();

    for pc in &presenter_cols {
        if let Some(ref name) = pc.header_name {
            if let Some(ref group) = pc.group_name {
                group_members
                    .entry(group.clone())
                    .or_default()
                    .push(name.clone());
            }
        }
    }

    let start_time_col = col_map
        .get("Start_Time")
        .or_else(|| col_map.get("StartTime"))
        .or_else(|| col_map.get("Start"))
        .copied();
    let end_time_col = col_map
        .get("End_Time")
        .or_else(|| col_map.get("EndTime"))
        .or_else(|| col_map.get("End"))
        .or_else(|| col_map.get("Lend"))
        .copied();
    let duration_col = col_map.get("Duration").copied();

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

        let uniq_id = get_field(&data, &["Uniq_ID", "UniqID", "ID", "Id"]).cloned();
        let name = match get_field(&data, &["Name", "Panel_Name", "PanelName"]) {
            Some(n) => n.clone(),
            None => continue,
        };

        let start_time = parse_datetime_value(
            start_time_col.and_then(|c| get_cell_str(ws, c, row)),
            start_time_col.and_then(|c| get_cell_number(ws, c, row)),
        );
        let start_time = match start_time {
            Some(dt) => dt,
            None => continue,
        };

        let end_time_from_cell = parse_datetime_value(
            end_time_col.and_then(|c| get_cell_str(ws, c, row)),
            end_time_col.and_then(|c| get_cell_number(ws, c, row)),
        );
        let duration_minutes = parse_duration_value(
            duration_col.and_then(|c| get_cell_str(ws, c, row)),
            duration_col.and_then(|c| get_cell_number(ws, c, row)),
        );

        let (end_time, duration) = match (end_time_from_cell, duration_minutes) {
            (Some(et), Some(_)) => {
                let diff = (et - start_time).num_minutes().max(0) as u32;
                (et, diff)
            }
            (Some(et), None) => {
                let diff = (et - start_time).num_minutes().max(0) as u32;
                (et, diff)
            }
            (None, Some(d)) => {
                let et = start_time + chrono::Duration::minutes(d as i64);
                (et, d)
            }
            (None, None) => {
                let et = start_time + chrono::Duration::hours(1);
                (et, 60)
            }
        };

        let room_name = get_field(&data, &["Room", "Room_Name", "RoomName"]).cloned();
        let room_id = room_name
            .as_ref()
            .and_then(|rn| room_lookup.get(&rn.to_lowercase()))
            .map(|r| r.uid);

        let id_prefix = extract_id_prefix(uniq_id.as_deref());
        let kind_raw = get_field(&data, &["Kind", "Panel_Kind", "PanelKind"]).cloned();
        let panel_type = if !id_prefix.is_empty() {
            type_lookup.get(&id_prefix.to_lowercase()).copied()
        } else {
            None
        };

        let panel_type = panel_type.or_else(|| {
            kind_raw.as_ref().and_then(|kr| {
                panel_types
                    .iter()
                    .find(|pt| pt.kind.to_lowercase() == kr.to_lowercase())
            })
        });

        let cost_raw = get_field(&data, &["Cost"]).cloned();
        let (cost, is_free, is_kids) = normalize_cost(cost_raw.as_ref());
        let is_full = get_field(&data, &["Full"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let mut event_presenters: Vec<String> = Vec::new();
        for pc in &presenter_cols {
            let cell_str = match get_cell_str(ws, pc.col, row) {
                Some(s) => s,
                None => continue,
            };

            if pc.is_named {
                if let Some(ref header_name) = pc.header_name {
                    event_presenters.push(header_name.clone());
                    let entry =
                        presenter_map
                            .entry(header_name.clone())
                            .or_insert_with(|| PresenterInfo {
                                rank: pc.rank.clone(),
                                groups: Vec::new(),
                                always_grouped: pc.always_grouped,
                            });
                    if let Some(ref group) = pc.group_name {
                        if !entry.groups.contains(group) {
                            entry.groups.push(group.clone());
                        }
                    }
                    if pc.always_grouped {
                        entry.always_grouped = true;
                    }
                }
            } else {
                let names = if pc.is_other {
                    split_presenter_names(&cell_str)
                } else {
                    vec![cell_str]
                };
                for presenter_name in names {
                    if presenter_name.is_empty() {
                        continue;
                    }
                    let clean_name = if let Some(eq_pos) = presenter_name.find('=') {
                        presenter_name[..eq_pos].trim().to_string()
                    } else {
                        presenter_name
                    };
                    if clean_name.is_empty() {
                        continue;
                    }
                    event_presenters.push(clean_name.clone());
                    presenter_map
                        .entry(clean_name)
                        .or_insert_with(|| PresenterInfo {
                            rank: pc.rank.clone(),
                            groups: Vec::new(),
                            always_grouped: false,
                        });
                }
            }
        }

        // Fallback: Presenter/Presenters column
        if event_presenters.is_empty() {
            if let Some(presenter_raw) =
                get_field(&data, &["Presenter", "Presenters", "Presenter_s"])
            {
                for part in split_presenter_names(presenter_raw) {
                    presenter_map
                        .entry(part.clone())
                        .or_insert_with(|| PresenterInfo {
                            rank: "fan_panelist".to_string(),
                            groups: Vec::new(),
                            always_grouped: false,
                        });
                    event_presenters.push(part);
                }
            }
        }

        // Always extract the prefix from the event ID for auto panel type creation
        let id_prefix = extract_id_prefix(uniq_id.as_deref());
        let panel_type_uid = if !id_prefix.is_empty() {
            Some(format!("panel-type-{}", id_prefix.to_lowercase()))
        } else {
            panel_type.map(|pt| pt.effective_uid())
        };

        let hide_panelist = get_field(&data, &["Hide_Panelist", "HidePanelist"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let alt_panelist = get_field(&data, &["Alt_Panelist", "AltPanelist"]).cloned();

        events.push(Event {
            id: uniq_id.unwrap_or_else(|| format!("row{}", events.len())),
            name,
            description: get_field(&data, &["Description"]).cloned(),
            start_time,
            end_time,
            duration,
            room_id,
            panel_type: panel_type_uid,
            cost,
            capacity: get_field(&data, &["Capacity"]).cloned(),
            difficulty: get_field(&data, &["Difficulty"]).cloned(),
            note: get_field(&data, &["Note"]).cloned(),
            prereq: get_field(&data, &["Prereq"]).cloned(),
            ticket_url: get_field(&data, &["Ticket_Sale", "TicketSale"]).cloned(),
            presenters: event_presenters,
            credits: Vec::new(),
            conflicts: Vec::new(),
            is_free,
            is_full,
            is_kids,
            hide_panelist,
            alt_panelist,
            source: Some(SourceInfo {
                file_path: Some(file_path.to_string()),
                sheet_name: Some(range.sheet_name.clone()),
                row_index: Some(row),
            }),
            change_state: ChangeState::Unchanged,
        });
    }

    let mut presenters: Vec<Presenter> = presenter_map
        .into_iter()
        .map(|(name, info)| {
            let is_group = group_members.contains_key(&name);
            let members = group_members.get(&name).cloned().unwrap_or_default();
            Presenter {
                name,
                rank: info.rank,
                is_group,
                members,
                groups: info.groups,
                always_grouped: info.always_grouped,
                source: None,
                change_state: ChangeState::Converted,
            }
        })
        .collect();
    presenters.sort_by(|a, b| a.name.cmp(&b.name));

    Ok((events, presenters))
}

fn extract_id_prefix(id: Option<&str>) -> String {
    let id = match id {
        Some(id) => id,
        None => return String::new(),
    };
    let re = Regex::new(r"^([A-Za-z]+)").expect("valid regex");
    re.captures(id)
        .map(|caps| caps[1].to_uppercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_header() {
        assert_eq!(canonical_header("Start Time"), Some("Start_Time".into()));
        assert_eq!(canonical_header("Start_Time"), Some("Start_Time".into()));
        assert_eq!(canonical_header("  Room  "), Some("Room".into()));
        assert_eq!(canonical_header("Uniq ID"), Some("Uniq_ID".into()));
        assert_eq!(canonical_header(""), None);
        assert_eq!(canonical_header("   "), None);
    }

    #[test]
    fn test_parse_presenter_header_kind_name() {
        let col = parse_presenter_header("G:Yaya Han", 5).expect("should parse");
        assert_eq!(col.rank, "guest");
        assert!(col.is_named);
        assert!(!col.is_other);
        assert_eq!(col.header_name.as_deref(), Some("Yaya Han"));
    }

    #[test]
    fn test_parse_presenter_header_kind_other() {
        let col = parse_presenter_header("S:Other", 3).expect("should parse");
        assert_eq!(col.rank, "staff");
        assert!(col.is_other);
        assert!(!col.is_named);
    }

    #[test]
    fn test_parse_presenter_header_legacy() {
        let col = parse_presenter_header("g1", 0).expect("should parse");
        assert_eq!(col.rank, "guest");
        assert!(!col.is_named);
        assert!(!col.is_other);
    }

    #[test]
    fn test_parse_presenter_header_word_prefix() {
        let col = parse_presenter_header("Guest1", 0).expect("should parse");
        assert_eq!(col.rank, "guest");
    }

    #[test]
    fn test_parse_presenter_header_not_presenter() {
        assert!(parse_presenter_header("Room", 0).is_none());
        assert!(parse_presenter_header("Name", 0).is_none());
        assert!(parse_presenter_header("Duration", 0).is_none());
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration_string("1:00"), Some(60));
        assert_eq!(parse_duration_string("1:30"), Some(90));
        assert_eq!(parse_duration_string("2:00"), Some(120));
        assert_eq!(parse_duration_string("90"), Some(90));
        assert_eq!(parse_duration_string(""), None);
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
    fn test_extract_id_prefix() {
        assert_eq!(extract_id_prefix(Some("GP002")), "GP");
        assert_eq!(extract_id_prefix(Some("FW001")), "FW");
        assert_eq!(extract_id_prefix(Some("GW019A")), "GW");
        assert_eq!(extract_id_prefix(Some("SPLIT01")), "SPLIT");
        assert_eq!(extract_id_prefix(Some("BREAK01")), "BREAK");
        assert_eq!(extract_id_prefix(None), "");
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

    #[test]
    fn test_split_events_filtered_from_events() {
        use super::super::panel_type::PanelType;
        use chrono::NaiveDateTime;

        let split_pt = PanelType {
            uid: Some("panel-type-split".to_string()),
            prefix: "SPLIT".to_string(),
            kind: "Split".to_string(),
            color: None,
            is_break: false,
            is_cafe: false,
            is_workshop: false,
            is_hidden: false,
            is_room_hours: false,
            bw_color: None,
            source: None,
            change_state: ChangeState::Converted,
        };
        let regular_pt = PanelType {
            uid: Some("panel-type-gp".to_string()),
            prefix: "GP".to_string(),
            kind: "General Panel".to_string(),
            color: None,
            is_break: false,
            is_cafe: false,
            is_workshop: false,
            is_hidden: false,
            is_room_hours: false,
            bw_color: None,
            source: None,
            change_state: ChangeState::Converted,
        };

        let start = NaiveDateTime::parse_from_str("2026-06-27T10:00:00", "%Y-%m-%dT%H:%M:%S")
            .unwrap();
        let end = NaiveDateTime::parse_from_str("2026-06-27T10:30:00", "%Y-%m-%dT%H:%M:%S")
            .unwrap();

        let split_event = Event {
            id: "SPLIT01".to_string(),
            name: "Day Break".to_string(),
            panel_type: Some("panel-type-split".to_string()),
            start_time: start,
            end_time: end,
            duration: 30,
            room_id: None,
            description: None,
            cost: None,
            capacity: None,
            difficulty: None,
            note: None,
            prereq: None,
            ticket_url: None,
            presenters: Vec::new(),
            credits: Vec::new(),
            conflicts: Vec::new(),
            is_free: true,
            is_full: false,
            is_kids: false,
            hide_panelist: false,
            alt_panelist: None,
            source: None,
            change_state: ChangeState::Unchanged,
        };
        let regular_event = Event {
            id: "GP001".to_string(),
            name: "My Panel".to_string(),
            panel_type: Some("panel-type-gp".to_string()),
            ..split_event.clone()
        };

        let panel_types = vec![split_pt, regular_pt];
        let events = vec![split_event, regular_event];
        let (filtered_pts, _time_types, timeline, filtered_events) =
            convert_split_types_to_timeline(&panel_types, &events);

        assert!(
            !filtered_events.iter().any(|e| e.panel_type.as_deref()
                == Some("panel-type-split")),
            "SPLIT events must be removed from events"
        );
        assert_eq!(filtered_events.len(), 1);
        assert_eq!(filtered_events[0].id, "GP001");
        assert_eq!(timeline.len(), 1);
        assert!(
            !filtered_pts.iter().any(|pt| pt.prefix == "SPLIT"),
            "SPLIT panel type must be removed from panel_types"
        );
    }

    #[test]
    fn test_parse_datetime_string() {
        let dt = parse_datetime_string("2026-06-26T14:00:00").expect("should parse ISO");
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-06-26 14:00");

        let dt = parse_datetime_string("6/26/2026 2:00 PM").expect("should parse US date");
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-06-26 14:00");
    }
}
