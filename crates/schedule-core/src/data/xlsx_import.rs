/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use indexmap::IndexMap;
use regex::Regex;
use umya_spreadsheet::Spreadsheet;
use umya_spreadsheet::structs::Worksheet;

use super::panel::{Panel, apply_common_prefix};
use super::panel_id::PanelId;
use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::room::Room;
use super::schedule::{Meta, Schedule};
use super::source_info::{ChangeState, ImportedSheetPresence, SourceInfo};

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
                Some(
                    file_modified_datetime
                        .format("%Y-%m-%dT%H:%M:%SZ")
                        .to_string(),
                )
            }
        } else {
            // Failed to parse, use file modified time instead
            Some(
                file_modified_datetime
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
            )
        }
    };

    let rooms = read_rooms(&book, &options.rooms_table, &file_path_str)?;
    let panel_types = read_panel_types(&book, &options.panel_types_table, &file_path_str)?;

    let imported_sheets = ImportedSheetPresence {
        has_room_map: !rooms.is_empty() && rooms.iter().any(|r| r.source.is_some()),
        has_panel_types: !panel_types.is_empty()
            && panel_types.values().any(|pt| pt.source.is_some()),
        has_presenters: false,
        has_schedule: true, // We'll assume schedule exists if we get here
    };

    let generated = if options.use_modified_as_generated && modified.is_some() {
        modified.clone().unwrap()
    } else {
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    };

    let (panels, presenters) = read_panels(
        &book,
        &options.schedule_table,
        &rooms,
        &panel_types,
        &file_path_str,
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
        timeline: Vec::new(),
        panels,
        events: Vec::new(),
        rooms,
        panel_types,
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
            is_break: false,
            metadata: None,
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
) -> Result<IndexMap<String, PanelType>> {
    let range = match find_data_range(book, preferred, &["Prefix", "PanelTypes"]) {
        Some(r) => r,
        None => return Ok(IndexMap::new()),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok(IndexMap::new());
    }

    let (raw_headers, canonical_headers, _col_map) = build_column_map(ws, &range);
    let mut types = IndexMap::new();

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

        let mut colors = IndexMap::new();
        if let Some(c) = get_field(&data, &["Color"]).cloned() {
            if !c.is_empty() {
                colors.insert("color".to_string(), c);
            }
        }
        if let Some(bw) = get_field(&data, &["BW", "Bw"]).cloned() {
            if !bw.is_empty() {
                colors.insert("bw".to_string(), bw);
            }
        }

        let is_hidden = get_field(&data, &["Hidden"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        let is_timeline = get_field(&data, &["Is_TimeLine", "Is_Timeline", "IsTimeLine"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix == "SPLIT" || prefix.starts_with("SP"));

        let is_private = get_field(&data, &["Is_Private", "IsPrivate"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix == "SM" || prefix == "ZZ");

        types.insert(
            prefix.clone(),
            PanelType {
                prefix: prefix.clone(),
                kind,
                colors,
                is_break,
                is_cafe,
                is_workshop,
                is_hidden,
                is_room_hours,
                is_timeline,
                is_private,
                metadata: None,
                source: Some(SourceInfo {
                    file_path: Some(file_path.to_string()),
                    sheet_name: Some(range.sheet_name.clone()),
                    row_index: Some(row),
                }),
                change_state: ChangeState::Unchanged,
            },
        );
    }

    Ok(types)
}

fn is_truthy(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    !lower.is_empty() && lower != "0" && lower != "no" && lower != "false"
}

#[derive(Debug, Clone, PartialEq)]
enum PresenterHeader {
    Named(String),
    Other,
}

#[derive(Debug)]
struct PresenterColumn {
    col: u32,
    rank_prefix: char,
    header: PresenterHeader,
}

fn rank_for_prefix(prefix: char) -> Option<&'static str> {
    match prefix {
        'g' => Some("guest"),
        'j' => Some("judge"),
        's' => Some("staff"),
        'i' => Some("invited_guest"),
        'p' => Some("fan_panelist"),
        _ => None,
    }
}

fn parse_presenter_header(header: &str, col: u32) -> Option<PresenterColumn> {
    let header = header.trim();
    if header.is_empty() {
        return None;
    }

    // Kind:Rest format — [GJSIP]:...
    let re_kind = Regex::new(r"(?i)^([GJSIP]):(.+)$").expect("valid regex");
    if let Some(caps) = re_kind.captures(header) {
        let prefix = caps[1].to_lowercase().chars().next()?;
        // Verify it's a known rank prefix
        rank_for_prefix(prefix)?;
        let rest = caps[2].trim().to_string();
        if rest.is_empty() {
            return None;
        }
        let header_kind = if rest.eq_ignore_ascii_case("other") {
            PresenterHeader::Other
        } else {
            PresenterHeader::Named(rest)
        };
        return Some(PresenterColumn {
            col,
            rank_prefix: prefix,
            header: header_kind,
        });
    }

    // "Other Guests" → guest other, "Other Staff" → staff other
    let lower = header.to_lowercase();
    if lower == "other guests" || lower == "other guest" {
        return Some(PresenterColumn {
            col,
            rank_prefix: 'g',
            header: PresenterHeader::Other,
        });
    }
    if lower == "other staff" {
        return Some(PresenterColumn {
            col,
            rank_prefix: 's',
            header: PresenterHeader::Other,
        });
    }

    // "Fan Panelist" or generic "Other"/"Others"
    if lower == "fan panelist" || lower.starts_with("other") {
        return Some(PresenterColumn {
            col,
            rank_prefix: 'p',
            header: PresenterHeader::Other,
        });
    }

    None
}

struct PresenterInfo {
    rank: String,
    groups: Vec<String>,
    always_grouped: bool,
}

/// Parse presenter data from a cell value, register it in the collection maps,
/// and return `(uid, is_credited)` if a presenter was found.
fn parse_presenter_data(
    header: &PresenterHeader,
    rank: &str,
    data: &str,
    presenter_map: &mut HashMap<String, PresenterInfo>,
    group_members: &mut HashMap<String, Vec<String>>,
    always_shown_groups: &mut HashSet<String>,
) -> Option<(String, bool)> {
    let data = data.trim();
    if data.is_empty() {
        return None;
    }

    // Check for * prefix → uncredited
    let (data, mut uncredited) = if let Some(rest) = data.strip_prefix('*') {
        (rest.trim(), true)
    } else {
        (data, false)
    };

    // Determine encoded_name based on header type
    let encoded_name = match header {
        PresenterHeader::Named(header_name) => {
            // For named headers, the header IS the name
            // Check if data is "Unlisted" → uncredited
            if data.eq_ignore_ascii_case("unlisted") {
                uncredited = true;
            }
            header_name.clone()
        }
        PresenterHeader::Other => {
            // For Other headers, the cell data IS the name
            data.to_string()
        }
    };

    if encoded_name.is_empty() {
        return None;
    }

    // Split on first '=' to get presenter and optional group
    let (presenter_raw, group_raw) = if let Some(eq_pos) = encoded_name.find('=') {
        let name_part = encoded_name[..eq_pos].trim().to_string();
        let group_part = encoded_name[eq_pos + 1..].trim().to_string();
        (
            name_part,
            if group_part.is_empty() {
                None
            } else {
                Some(group_part)
            },
        )
    } else {
        (encoded_name, None)
    };

    // Check if presenter begins with '<' → always_grouped
    let (presenter_name, always_grouped) = if let Some(rest) = presenter_raw.strip_prefix('<') {
        (rest.trim().to_string(), true)
    } else {
        (presenter_raw, false)
    };

    // Check if group begins with '=' (original was '==') → always_shown_group
    let (group_name, always_shown_group) = match group_raw {
        Some(g) => {
            if let Some(rest) = g.strip_prefix('=') {
                (Some(rest.trim().to_string()), true)
            } else {
                (Some(g), false)
            }
        }
        None => (None, false),
    };

    // Filter out empty group after stripping
    let group_name = group_name.filter(|g| !g.is_empty());

    // If presenter name is empty but group is present, the presenter IS the group
    let uid = if presenter_name.is_empty() {
        match group_name {
            Some(ref g) => g.clone(),
            None => return None,
        }
    } else {
        presenter_name
    };

    // Register the presenter in the map
    let entry = presenter_map
        .entry(uid.clone())
        .or_insert_with(|| PresenterInfo {
            rank: rank.to_string(),
            groups: Vec::new(),
            always_grouped,
        });
    if always_grouped {
        entry.always_grouped = true;
    }

    // Register group membership
    if let Some(ref group) = group_name {
        if !entry.groups.contains(group) {
            entry.groups.push(group.clone());
        }
        group_members
            .entry(group.clone())
            .or_default()
            .push(uid.clone());
        if always_shown_group {
            always_shown_groups.insert(group.clone());
        }
    }

    Some((uid, !uncredited))
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

/// Prepend a narrowed base/part prefix tail to an existing sibling field value.
///
/// When a common prefix narrows and the stripped portion needs to be pushed down
/// to existing siblings, their stored value gets the old tail prepended:
/// - `prepend_suffix("DEF GH", None)` → `Some("DEF GH")`
/// - `prepend_suffix("DEF GH", Some("xyz"))` → `Some("DEF GH xyz")`
/// - `prepend_suffix("", Some("xyz"))` → `Some("xyz")` (no-op)
fn prepend_suffix(prefix: &str, existing: Option<&str>) -> Option<String> {
    if prefix.is_empty() {
        return existing.map(|s| s.to_string());
    }
    match existing.filter(|s| !s.is_empty()) {
        None => Some(prefix.to_string()),
        Some(val) => Some(format!("{} {}", prefix, val)),
    }
}

fn read_panels(
    book: &Spreadsheet,
    preferred: &str,
    rooms: &[Room],
    panel_types: &IndexMap<String, PanelType>,
    file_path: &str,
) -> Result<(IndexMap<String, Panel>, Vec<Presenter>)> {
    let first_sheet_name = book
        .get_sheet_collection()
        .first()
        .map(|s| s.get_name().to_string());
    let first_sheet_ref: &str = first_sheet_name.as_deref().unwrap_or("");
    let range = match find_data_range(book, preferred, &["Schedule", first_sheet_ref]) {
        Some(r) => r,
        None => return Ok((IndexMap::new(), Vec::new())),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok((IndexMap::new(), Vec::new()));
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
        .map(|(prefix, pt)| (prefix.to_lowercase(), pt))
        .collect();

    let mut presenter_map: HashMap<String, PresenterInfo> = HashMap::new();
    let mut group_members: HashMap<String, Vec<String>> = HashMap::new();
    let mut always_shown_groups: HashSet<String> = HashSet::new();
    let mut panels: IndexMap<String, Panel> = IndexMap::new();

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
        let raw_name = match get_field(&data, &["Name", "Panel_Name", "PanelName"]) {
            Some(n) => n.clone(),
            None => continue,
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

        let kind_raw = get_field(&data, &["Kind", "Panel_Kind", "PanelKind"]).cloned();
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

        let cost_raw = get_field(&data, &["Cost"]).cloned();
        let (cost, is_free, is_kids) = normalize_cost(cost_raw.as_ref());
        let is_full = get_field(&data, &["Full"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let mut credited_presenters: Vec<String> = Vec::new();
        let mut uncredited_presenters: Vec<String> = Vec::new();
        for pc in &presenter_cols {
            let cell_str = match get_cell_str(ws, pc.col, row) {
                Some(s) => s,
                None => continue,
            };

            let rank = rank_for_prefix(pc.rank_prefix).unwrap_or("fan_panelist");

            // For Other columns, split by commas; for Named, each chunk is the whole cell
            let chunks: Vec<String> = match &pc.header {
                PresenterHeader::Other => split_presenter_names(&cell_str),
                PresenterHeader::Named(_) => vec![cell_str],
            };

            for chunk in chunks {
                let (uid, is_credited) = match parse_presenter_data(
                    &pc.header,
                    rank,
                    &chunk,
                    &mut presenter_map,
                    &mut group_members,
                    &mut always_shown_groups,
                ) {
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

        // Get other fields
        let description = get_field(&data, &["Description"]).cloned();
        let note = get_field(&data, &["Note"]).cloned();
        let prereq = get_field(&data, &["Prereq"]).cloned();
        let alt_panelist = get_field(&data, &["Alt_Panelist", "AltPanelist"]).cloned();
        let capacity = get_field(&data, &["Capacity"]).cloned();
        let difficulty = get_field(&data, &["Difficulty"]).cloned();
        let ticket_url = get_field(&data, &["Ticket_Sale", "TicketSale"]).cloned();
        let simple_tix_event = get_field(&data, &["SimpleTix_Event", "SimpleTixEvent"]).cloned();
        let hide_panelist = get_field(&data, &["Hide_Panelist", "HidePanelist"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let seats_sold =
            get_field(&data, &["Seats_Sold", "SeatsSold"]).and_then(|s| s.parse::<u32>().ok());
        let pre_reg_max = get_field(&data, &["PreReg_Max", "PreRegMax"]).cloned();
        let notes_non_printing =
            get_field(&data, &["Notes_Non_Printing", "NotesNonPrinting"]).cloned();
        let workshop_notes = get_field(&data, &["Workshop_Notes", "WorkshopNotes"]).cloned();
        let power_needs = get_field(&data, &["Power_Needs", "PowerNeeds"]).cloned();
        let sewing_machines = get_field(&data, &["Sewing_Machines", "SewingMachines"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let av_notes = get_field(&data, &["AV_Notes", "AVNotes"]).cloned();
        let have_ticket_image =
            get_field(&data, &["Have_Ticket_Image", "HaveTicketImage"]).map(|s| is_truthy(s));

        // Find or create the base panel
        let panel = panels.entry(panel_id.base_id()).or_insert_with(|| {
            let mut p = Panel::new(panel_id.base_id());
            p.name = name.clone();
            p.panel_type = panel_type_uid.clone();
            p.cost = cost.clone();
            p.capacity = capacity.clone();
            p.difficulty = difficulty.clone();
            p.ticket_url = ticket_url.clone();
            p.is_free = is_free;
            p.is_kids = is_kids;
            p.simple_tix_event = simple_tix_event.clone();
            p.have_ticket_image = have_ticket_image;
            // Store first description/note/prereq at base level
            p.description = description.clone();
            p.note = note.clone();
            p.prereq = prereq.clone();
            p.credited_presenters = credited_presenters.clone();
            p.uncredited_presenters = uncredited_presenters.clone();
            p
        });

        // Apply common-prefix algorithm at base level.
        // Each call returns (new_entry_suffix, old_prefix_suffix_if_narrowed).
        // Neither value includes the separator space; join_parts adds it back.
        let (base_desc_suffix, narrowed_base_desc) = match description.as_deref() {
            Some(v) => apply_common_prefix(&mut panel.description, v),
            None => (String::new(), None),
        };
        let (base_note_suffix, narrowed_base_note) = match note.as_deref() {
            Some(v) => apply_common_prefix(&mut panel.note, v),
            None => (String::new(), None),
        };
        let (base_prereq_suffix, narrowed_base_prereq) = match prereq.as_deref() {
            Some(v) => apply_common_prefix(&mut panel.prereq, v),
            None => (String::new(), None),
        };

        // When the base prefix narrowed, push the old base tail to all existing parts.
        if narrowed_base_desc.is_some()
            || narrowed_base_note.is_some()
            || narrowed_base_prereq.is_some()
        {
            for ep in &mut panel.parts {
                if let Some(ref tail) = narrowed_base_desc {
                    ep.description = prepend_suffix(tail, ep.description.as_deref());
                }
                if let Some(ref tail) = narrowed_base_note {
                    ep.note = prepend_suffix(tail, ep.note.as_deref());
                }
                if let Some(ref tail) = narrowed_base_prereq {
                    ep.prereq = prepend_suffix(tail, ep.prereq.as_deref());
                }
            }
        }

        let panel_id_str = panel.id.clone();

        // Detect whether this part already exists before find_or_create_part.
        let part_already_exists = panel_id
            .part_num
            .map(|n| panel.parts.iter().any(|p| p.part_num == Some(n)))
            .unwrap_or(!panel.parts.is_empty());

        // Find or create the part
        let part = panel.find_or_create_part(panel_id.part_num);

        // Apply common-prefix at the part level using the base-level suffixes.
        let (part_desc_suffix, part_note_suffix, part_prereq_suffix) = if part_already_exists {
            let (s_desc, n_desc) = apply_common_prefix(&mut part.description, &base_desc_suffix);
            let (s_note, n_note) = apply_common_prefix(&mut part.note, &base_note_suffix);
            let (s_prereq, n_prereq) = apply_common_prefix(&mut part.prereq, &base_prereq_suffix);
            // When part-level fields narrowed, push old tails to all existing sessions.
            if n_desc.is_some() || n_note.is_some() || n_prereq.is_some() {
                for es in &mut part.sessions {
                    if let Some(ref tail) = n_desc {
                        es.description = prepend_suffix(tail, es.description.as_deref());
                    }
                    if let Some(ref tail) = n_note {
                        es.note = prepend_suffix(tail, es.note.as_deref());
                    }
                    if let Some(ref tail) = n_prereq {
                        es.prereq = prepend_suffix(tail, es.prereq.as_deref());
                    }
                }
            }
            (s_desc, s_note, s_prereq)
        } else {
            if !base_desc_suffix.is_empty() {
                part.description = Some(base_desc_suffix.clone());
            }
            if !base_note_suffix.is_empty() {
                part.note = Some(base_note_suffix.clone());
            }
            if !base_prereq_suffix.is_empty() {
                part.prereq = Some(base_prereq_suffix.clone());
            }
            (String::new(), String::new(), String::new())
        };

        // Add presenters to part
        for presenter in &credited_presenters {
            if !part.credited_presenters.contains(presenter) {
                part.credited_presenters.push(presenter.clone());
            }
        }
        for presenter in &uncredited_presenters {
            if !part.uncredited_presenters.contains(presenter) {
                part.uncredited_presenters.push(presenter.clone());
            }
        }

        // Clone values before creating session
        let part_sessions_count = part.sessions.len();

        // Create the session
        let session_id =
            uniq_id.unwrap_or_else(|| format!("{}-session-{}", panel_id_str, part_sessions_count));
        let session = part.find_or_create_session(panel_id.session_num, session_id);

        // Set session fields
        session.room_ids = room_ids;
        session.start_time = Some(start_time.format("%Y-%m-%dT%H:%M:%S").to_string());
        session.end_time = Some(end_time.format("%Y-%m-%dT%H:%M:%S").to_string());
        session.duration = duration;
        session.is_full = is_full;
        session.capacity = capacity;
        session.seats_sold = seats_sold;
        session.pre_reg_max = pre_reg_max;
        session.ticket_url = ticket_url;
        session.simple_tix_event = simple_tix_event;
        session.hide_panelist = hide_panelist;
        session.notes_non_printing = notes_non_printing;
        session.workshop_notes = workshop_notes;
        session.power_needs = power_needs;
        session.sewing_machines = sewing_machines;
        session.av_notes = av_notes;
        session.source = Some(SourceInfo {
            file_path: Some(file_path.to_string()),
            sheet_name: Some(range.sheet_name.clone()),
            row_index: Some(row),
        });

        // Store the part-level suffixes as the session's unique fields.
        if !part_desc_suffix.is_empty() {
            session.description = Some(part_desc_suffix);
        }
        if !part_note_suffix.is_empty() {
            session.note = Some(part_note_suffix);
        }
        if !part_prereq_suffix.is_empty() {
            session.prereq = Some(part_prereq_suffix);
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
            session.conflicts.push(super::event::EventConflict {
                conflict_type: "title_id_mismatch".to_string(),
                details: Some(conflict_details),
                conflict_event_id: None,
            });
        }

        // Store alt_panelist at session level; post-processing promotes uniform values upward.
        session.alt_panelist = alt_panelist;

        // Add presenters to session
        session.credited_presenters = credited_presenters;
        session.uncredited_presenters = uncredited_presenters;
    }

    // Post-processing: promote uniform alt_panelist values up the hierarchy.
    // If all sessions within a part share the same value, move it to the part level.
    // If all parts then share the same value, move it to the base level.
    for panel in panels.values_mut() {
        for part in &mut panel.parts {
            if part.sessions.is_empty() {
                continue;
            }
            let first = part.sessions[0].alt_panelist.clone();
            if part.sessions.iter().all(|s| s.alt_panelist == first) {
                part.alt_panelist = first;
                for session in &mut part.sessions {
                    session.alt_panelist = None;
                }
            }
        }

        if panel.parts.is_empty() {
            continue;
        }
        let first = panel.parts[0].alt_panelist.clone();
        if panel.parts.iter().all(|p| p.alt_panelist == first) {
            panel.alt_panelist = first;
            for part in &mut panel.parts {
                part.alt_panelist = None;
            }
        }
    }

    let mut presenters: Vec<Presenter> = presenter_map
        .into_iter()
        .map(|(name, info)| {
            let is_group = group_members.contains_key(&name);
            let members = group_members.get(&name).cloned().unwrap_or_default();
            let always_shown = is_group && always_shown_groups.contains(&name);
            Presenter {
                id: None,
                name,
                rank: info.rank,
                is_group,
                members,
                groups: info.groups,
                always_grouped: info.always_grouped,
                always_shown,
                metadata: None,
                source: None,
                change_state: ChangeState::Converted,
            }
        })
        .collect();
    presenters.sort_by(|a, b| a.name.cmp(&b.name));

    Ok((panels, presenters))
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
        assert_eq!(col.rank_prefix, 'g');
        assert_eq!(col.header, PresenterHeader::Named("Yaya Han".to_string()));
    }

    #[test]
    fn test_parse_presenter_header_kind_name_with_group() {
        // Header stores full rest including =Group; parsing happens in parse_presenter_data
        let col = parse_presenter_header("G:John==UNC Staff", 1).expect("should parse");
        assert_eq!(col.rank_prefix, 'g');
        assert_eq!(
            col.header,
            PresenterHeader::Named("John==UNC Staff".to_string())
        );
    }

    #[test]
    fn test_parse_presenter_header_kind_other() {
        let col = parse_presenter_header("S:Other", 3).expect("should parse");
        assert_eq!(col.rank_prefix, 's');
        assert_eq!(col.header, PresenterHeader::Other);
    }

    #[test]
    fn test_parse_presenter_header_other_guests() {
        let col = parse_presenter_header("Other Guests", 0).expect("should parse");
        assert_eq!(col.rank_prefix, 'g');
        assert_eq!(col.header, PresenterHeader::Other);
    }

    #[test]
    fn test_parse_presenter_header_fan_panelist() {
        let col = parse_presenter_header("Fan Panelist", 0).expect("should parse");
        assert_eq!(col.rank_prefix, 'p');
        assert_eq!(col.header, PresenterHeader::Other);
    }

    #[test]
    fn test_parse_presenter_header_not_presenter() {
        assert!(parse_presenter_header("Room", 0).is_none());
        assert!(parse_presenter_header("Name", 0).is_none());
        assert!(parse_presenter_header("Duration", 0).is_none());
        assert!(parse_presenter_header("g1", 0).is_none());
        assert!(parse_presenter_header("Guest1", 0).is_none());
    }

    // --- parse_presenter_data tests ---

    fn empty_maps() -> (
        HashMap<String, PresenterInfo>,
        HashMap<String, Vec<String>>,
        HashSet<String>,
    ) {
        (HashMap::new(), HashMap::new(), HashSet::new())
    }

    #[test]
    fn test_parse_data_named_simple() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Named("Yaya Han".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm, &mut gm, &mut asg)
                .expect("should parse");
        assert_eq!(uid, "Yaya Han");
        assert!(credited);
        assert!(pm.contains_key("Yaya Han"));
        assert!(gm.is_empty());
        assert!(asg.is_empty());
    }

    #[test]
    fn test_parse_data_named_unlisted() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Named("Secret Guest".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Unlisted", &mut pm, &mut gm, &mut asg)
                .expect("should parse");
        assert_eq!(uid, "Secret Guest");
        assert!(!credited, "Unlisted should be uncredited");
    }

    #[test]
    fn test_parse_data_named_star_uncredited() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Named("Helper".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "*Yes", &mut pm, &mut gm, &mut asg)
                .expect("should parse");
        assert_eq!(uid, "Helper");
        assert!(!credited, "* prefix should be uncredited");
    }

    #[test]
    fn test_parse_data_named_with_double_eq_group() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Named("John==UNC Staff".to_string());
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm, &mut gm, &mut asg)
                .expect("should parse");
        assert_eq!(uid, "John");
        assert_eq!(pm["John"].groups, vec!["UNC Staff"]);
        assert!(!pm["John"].always_grouped);
        assert!(
            asg.contains("UNC Staff"),
            "== should set always_shown_group"
        );
        assert!(gm.contains_key("UNC Staff"));
    }

    #[test]
    fn test_parse_data_named_lt_always_grouped() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Named("<Jane=UNC Staff".to_string());
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm, &mut gm, &mut asg)
                .expect("should parse");
        assert_eq!(uid, "Jane");
        assert!(
            pm["Jane"].always_grouped,
            "< prefix should set always_grouped"
        );
        assert_eq!(pm["Jane"].groups, vec!["UNC Staff"]);
        assert!(
            !asg.contains("UNC Staff"),
            "single = should not set always_shown_group"
        );
    }

    #[test]
    fn test_parse_data_named_lt_double_eq_combined() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Named("<Bob==Team".to_string());
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm, &mut gm, &mut asg)
                .expect("should parse");
        assert_eq!(uid, "Bob");
        assert!(
            pm["Bob"].always_grouped,
            "< prefix should set always_grouped"
        );
        assert!(asg.contains("Team"), "== should set always_shown_group");
        assert!(gm.contains_key("Team"));
    }

    #[test]
    fn test_parse_data_other_simple() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Other;
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Alice", &mut pm, &mut gm, &mut asg)
                .expect("should parse");
        assert_eq!(uid, "Alice");
        assert!(credited);
        assert!(pm.contains_key("Alice"));
        assert!(gm.is_empty());
    }

    #[test]
    fn test_parse_data_other_with_group() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Other;
        let (uid, _credited) = parse_presenter_data(
            &header,
            "guest",
            "Triffin Morris=UNC Staff",
            &mut pm,
            &mut gm,
            &mut asg,
        )
        .expect("should parse");
        assert_eq!(uid, "Triffin Morris");
        assert_eq!(pm["Triffin Morris"].groups, vec!["UNC Staff"]);
        assert!(!asg.contains("UNC Staff"));
    }

    #[test]
    fn test_parse_data_other_with_double_eq_group() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Other;
        let (uid, _credited) = parse_presenter_data(
            &header,
            "guest",
            "Triffin Morris==UNC Staff",
            &mut pm,
            &mut gm,
            &mut asg,
        )
        .expect("should parse");
        assert_eq!(uid, "Triffin Morris");
        assert!(
            asg.contains("UNC Staff"),
            "== should set always_shown_group"
        );
    }

    #[test]
    fn test_parse_data_other_star_uncredited() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Other;
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "*Helper", &mut pm, &mut gm, &mut asg)
                .expect("should parse");
        assert_eq!(uid, "Helper");
        assert!(!credited, "* prefix should be uncredited");
    }

    #[test]
    fn test_parse_data_blank_returns_none() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Other;
        assert!(parse_presenter_data(&header, "guest", "", &mut pm, &mut gm, &mut asg).is_none());
        assert!(parse_presenter_data(&header, "guest", "  ", &mut pm, &mut gm, &mut asg).is_none());
    }

    #[test]
    fn test_parse_data_empty_name_with_group() {
        let (mut pm, mut gm, mut asg) = empty_maps();
        let header = PresenterHeader::Named("==UNC Staff".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm, &mut gm, &mut asg)
                .expect("should parse");
        assert_eq!(uid, "UNC Staff", "empty name should use group as uid");
        assert!(credited);
        assert!(pm.contains_key("UNC Staff"));
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
    fn test_parse_datetime_string() {
        let dt = parse_datetime_string("2026-06-26T14:00:00").expect("should parse ISO");
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-06-26 14:00");

        let dt = parse_datetime_string("6/26/2026 2:00 PM").expect("should parse US date");
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-06-26 14:00");
    }
}
