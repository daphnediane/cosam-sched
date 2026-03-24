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

use super::panel::{ExtraFields, ExtraValue, FormulaValue, Panel, apply_common_prefix};
use super::panel_id::PanelId;
use super::panel_type::PanelType;
use super::presenter::{Presenter, PresenterGroup, PresenterMember, PresenterRank};
use super::room::Room;
use super::schedule::{Meta, Schedule};
use super::source_info::{ChangeState, ImportedSheetPresence, SourceInfo};
use super::timeline::TimelineEntry;

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
    let presenter_ranks = read_presenter_ranks(&book, &file_path_str)?;

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
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    };

    let (panels, presenters, timeline_entries) = read_panels(
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

/// Capture raw-header columns that are not in `known_aliases` as ExtraFields metadata.
/// Uses canonical_header normalization to match regardless of spacing/punctuation.
fn collect_extra_metadata(
    row_data: &HashMap<String, String>,
    raw_headers: &[String],
    known_aliases: &[&str],
) -> Option<ExtraFields> {
    let known_canonical: HashSet<String> = known_aliases
        .iter()
        .filter_map(|h| canonical_header(h))
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

        const ROOM_KNOWN: &[&str] = &[
            "Room_Name",
            "Room",
            "Name",
            "Long_Name",
            "Hotel_Room",
            "HotelRoom",
            "Sort_Key",
            "SortKey",
            "Is_Break",
        ];
        let metadata = collect_extra_metadata(&data, &raw_headers, ROOM_KNOWN);

        rooms.push(Room {
            uid,
            short_name,
            long_name,
            hotel_room,
            sort_key,
            is_break: false,
            metadata,
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

        const PANEL_TYPE_KNOWN: &[&str] = &[
            "Prefix",
            "Panel_Kind",
            "PanelKind",
            "Kind",
            "Color",
            "BW",
            "Bw",
            "Is_Break",
            "Is_Cafe",
            "Is_Café",
            "Is_Workshop",
            "Is_Room_Hours",
            "IsRoomHours",
            "Is_Split",
            "Hidden",
            "Is_TimeLine",
            "Is_Timeline",
            "IsTimeLine",
            "Is_Private",
            "IsPrivate",
        ];
        let metadata = collect_extra_metadata(&data, &raw_headers, PANEL_TYPE_KNOWN);

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
                metadata,
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
pub(super) enum PresenterHeader {
    Named(String),
    Other,
}

#[derive(Debug)]
pub(super) struct PresenterColumn {
    pub(super) col: u32,
    pub(super) rank_prefix: char,
    pub(super) header: PresenterHeader,
}

pub(super) fn rank_for_prefix(prefix: char) -> Option<&'static str> {
    match prefix {
        'g' => Some("guest"),
        'j' => Some("judge"),
        's' => Some("staff"),
        'i' => Some("invited_guest"),
        'p' => Some("fan_panelist"),
        _ => None,
    }
}

pub(super) fn parse_presenter_header(header: &str, col: u32) -> Option<PresenterColumn> {
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
    rank: PresenterRank,
    is_member: PresenterMember,
    is_grouped: PresenterGroup,
}

/// Parse presenter data from a cell value, register it in the collection maps,
/// and return `(uid, is_credited)` if a presenter was found.
fn parse_presenter_data(
    header: &PresenterHeader,
    rank: &str,
    data: &str,
    presenter_map: &mut HashMap<String, PresenterInfo>,
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

    // Initialize group entry in presenter_map if needed
    if let Some(ref g) = group_name {
        let entry = presenter_map
            .entry(g.clone())
            .or_insert_with(|| PresenterInfo {
                rank: PresenterRank::from_str(rank),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
            });
        // Update existing group entry if needed
        match &mut entry.is_grouped {
            PresenterGroup::IsGroup(_, shown) => {
                *shown = *shown || always_shown_group;
            }
            PresenterGroup::NotGroup => {
                entry.is_grouped =
                    PresenterGroup::IsGroup(std::collections::BTreeSet::new(), always_shown_group);
            }
        }
    }

    // If presenter name is empty but group is present, the presenter IS the group
    if presenter_name.is_empty() || Some(presenter_name.clone()) == group_name {
        return match group_name {
            Some(ref g) => Some((g.clone(), !uncredited)),
            None => None,
        };
    }

    // Handle group membership first if we have a group
    if let Some(ref group_name) = group_name {
        // Get or create the group entry
        let group_entry =
            presenter_map
                .entry(group_name.clone())
                .or_insert_with(|| PresenterInfo {
                    rank: PresenterRank::from_str(rank),
                    is_member: PresenterMember::NotMember,
                    is_grouped: PresenterGroup::NotGroup,
                });

        // Add presenter to group's members
        if let PresenterGroup::IsGroup(members, _) = &mut group_entry.is_grouped {
            members.insert(presenter_name.clone());
        }
    }

    // Now register the presenter in the map
    let presenter_name_for_entry = presenter_name.clone();
    let entry = presenter_map
        .entry(presenter_name_for_entry)
        .or_insert_with(|| PresenterInfo {
            rank: PresenterRank::from_str(rank),
            is_member: PresenterMember::NotMember,
            is_grouped: PresenterGroup::NotGroup,
        });

    // Set presenter's group membership if we have a group
    if let Some(ref group_name) = group_name {
        match &mut entry.is_member {
            PresenterMember::IsMember(groups, grouped) => {
                groups.insert(group_name.clone());
                *grouped = *grouped || always_grouped;
            }
            PresenterMember::NotMember => {
                entry.is_member = PresenterMember::IsMember(
                    std::collections::BTreeSet::from([group_name.clone()]),
                    always_grouped,
                );
            }
        }
    }

    Some((presenter_name, !uncredited))
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

    // M-DD-YY HH:MM format (e.g., "6-27-26 18:00")
    let re_short = Regex::new(r"^(\d{1,2})-(\d{1,2})-(\d{2})\s+(\d{1,2}):(\d{2})$").ok()?;
    if let Some(caps) = re_short.captures(text) {
        let month: u32 = caps[1].parse().ok()?;
        let day: u32 = caps[2].parse().ok()?;
        let year_short: u32 = caps[3].parse().ok()?;
        let hour: u32 = caps[4].parse().ok()?;
        let minute: u32 = caps[5].parse().ok()?;

        // Convert 2-digit year to 4-digit year (assuming 2000s for 00-99)
        let year = if year_short >= 70 {
            1900 + year_short as i32
        } else {
            2000 + year_short as i32
        };

        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
        let time = chrono::NaiveTime::from_hms_opt(hour, minute, 0)?;
        return Some(NaiveDateTime::new(date, time));
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

    // Plain number = minutes (only integers, not decimals)
    if let Ok(minutes) = text.parse::<u32>() {
        return Some(minutes);
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
    presenter_ranks: &HashMap<String, String>,
) -> Result<(IndexMap<String, Panel>, Vec<Presenter>, Vec<TimelineEntry>)> {
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
                DataRange {
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
        None => return Ok((IndexMap::new(), Vec::new(), Vec::new())),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok((IndexMap::new(), Vec::new(), Vec::new()));
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

    // Known canonical header names for standard schedule columns
    let known_canonical_headers: HashSet<&str> = [
        "Uniq_ID",
        "UniqID",
        "ID",
        "Id",
        "Name",
        "Panel_Name",
        "PanelName",
        "Description",
        "Start_Time",
        "StartTime",
        "Start",
        "End_Time",
        "EndTime",
        "End",
        "Duration",
        "Room",
        "Room_Name",
        "RoomName",
        "Kind",
        "Panel_Kind",
        "PanelKind",
        "Cost",
        "Capacity",
        "Difficulty",
        "Note",
        "Prereq",
        "Ticket_Sale",
        "TicketSale",
        "Full",
        "Is_Full",
        "IsFull",
        "Hide_Panelist",
        "HidePanelist",
        "Alt_Panelist",
        "AltPanelist",
        "Presenter",
        "Presenters",
        "Presenter_s",
        "Seats_Sold",
        "SeatsSold",
        "PreReg_Max",
        "PreRegMax",
        "Notes_Non_Printing",
        "NotesNonPrinting",
        "Workshop_Notes",
        "WorkshopNotes",
        "Power_Needs",
        "PowerNeeds",
        "Sewing_Machines",
        "SewingMachines",
        "AV_Notes",
        "AVNotes",
        "Have_Ticket_Image",
        "HaveTicketImage",
        "SimpleTix_Event",
        "SimpleTixEvent",
        "Lstart",
        "Lend",
        "Old_Uniq_Id",
        "OldUniqId",
    ]
    .iter()
    .copied()
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
    let mut panels: IndexMap<String, Panel> = IndexMap::new();
    let mut timeline_entries: Vec<TimelineEntry> = Vec::new();

    let start_time_col = col_map
        .get("Start_Time")
        .or_else(|| col_map.get("StartTime"))
        .or_else(|| col_map.get("Start"))
        .copied();
    let end_time_col = col_map
        .get("End_Time")
        .or_else(|| col_map.get("EndTime"))
        .or_else(|| col_map.get("End"))
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

        let raw_uniq_id = get_field(&data, &["Uniq_ID", "UniqID", "ID", "Id"]).cloned();
        // A leading * means this row was soft-deleted by xlsx_update; strip it and mark deleted.
        let (uniq_id, is_deleted_row) = match raw_uniq_id {
            Some(ref s) if s.starts_with('*') => {
                (Some(s.trim_start_matches('*').to_string()), true)
            }
            other => (other, false),
        };
        let raw_name = match get_field(&data, &["Name", "Panel_Name", "PanelName"]) {
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

        // Allow panels without start times - they might be unscheduled
        let start_time = start_time.unwrap_or_else(|| {
            // Default to a placeholder time for unscheduled panels
            chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2026, 6, 26).unwrap(),
                chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            )
        });

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
                // Panel is unscheduled - no end time or duration
                // Use placeholder values but let is_scheduled() handle it
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
                get_field(&data, &["Presenter", "Presenters", "Presenter_s"])
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
                let note = get_field(&data, &["Note"]).cloned();

                // Create a TimelineEntry instead of a regular Panel
                let timeline_entry = TimelineEntry {
                    id: uniq_id.unwrap_or_else(|| format!("TL{}", row)).to_string(),
                    start_time: start_time.format("%Y-%m-%dT%H:%M:%S").to_string(),
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

        // Find or create the base panel, handling duplicates
        let is_duplicate = panels.contains_key(&panel_id.base_id());
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

        // Handle duplicate Uniq ID cases
        if is_duplicate {
            if panel.name == name {
                // Same Uniq ID + Same Name → Different sessions with alpha suffixes
            } else {
                // Same Uniq ID + Different Name → Update to new unused ID of same panel type
                // TODO: Generate new unused ID of same panel type
            }
        }

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

        // Create the session - always add new during import, handle conflicts in post-processing
        let session_id = if let Some(ref id) = uniq_id {
            id.clone()
        } else {
            format!("{}-session-{}", panel_id_str, part_sessions_count)
        };

        let session = part.create_new_session(panel_id.session_num, session_id);

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
        if is_deleted_row {
            session.change_state = ChangeState::Deleted;
        }

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
                session.metadata = metadata;
            }
        }
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
            // Use preserved rank from People sheet if available, otherwise use inferred rank
            let rank = if let Some(preserved_rank) = presenter_ranks.get(&name) {
                PresenterRank::from_str(preserved_rank)
            } else {
                info.rank
            };

            Presenter {
                id: None,
                name,
                rank,
                is_member: info.is_member.clone(),
                is_grouped: info.is_grouped.clone(),
                metadata: None,
                source: None,
                change_state: ChangeState::Converted,
            }
        })
        .collect();

    presenters.sort_by(|a, b| a.name.cmp(&b.name));

    Ok((panels, presenters, timeline_entries))
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
    fn test_parse_presenter_data_with_prefixes() {
        let mut presenter_map: HashMap<String, PresenterInfo> = HashMap::new();

        // Test <Name prefix (always_grouped)
        let header = PresenterHeader::Other;
        let result = parse_presenter_data(
            &header,
            "fan_panelist",
            "<John Doe=Test Group",
            &mut presenter_map,
        );
        assert_eq!(result, Some(("John Doe".to_string(), true)));

        // Check John Doe's always_grouped status and group membership
        let john_presenter = presenter_map.get("John Doe").unwrap();
        let is_always_grouped = match &john_presenter.is_member {
            PresenterMember::IsMember(_, always_grouped) => *always_grouped,
            PresenterMember::NotMember => false,
        };
        assert!(is_always_grouped);
        let mut expected_groups = std::collections::BTreeSet::new();
        expected_groups.insert("Test Group".to_string());
        let john_groups = match &john_presenter.is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert_eq!(john_groups, &expected_groups);

        // Check Test Group exists and has John as member
        let test_group = presenter_map.get("Test Group").unwrap();
        if let PresenterGroup::IsGroup(members, _) = &test_group.is_grouped {
            assert!(members.contains("John Doe"));
        } else {
            panic!("Test Group should be a group");
        }

        // Test ==Group prefix (always_shown)
        let result2 = parse_presenter_data(
            &header,
            "fan_panelist",
            "Jane Doe==Always Shown Group",
            &mut presenter_map,
        );
        assert_eq!(result2, Some(("Jane Doe".to_string(), true)));

        // Check Jane Doe's group membership
        let jane_presenter = presenter_map.get("Jane Doe").unwrap();
        let jane_groups = match &jane_presenter.is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        let mut expected_jane_groups = std::collections::BTreeSet::new();
        expected_jane_groups.insert("Always Shown Group".to_string());
        assert_eq!(jane_groups, &expected_jane_groups);

        // Check Always Shown Group exists and is always_shown
        let always_shown_group = presenter_map.get("Always Shown Group").unwrap();
        if let PresenterGroup::IsGroup(_, always_shown) = &always_shown_group.is_grouped {
            assert!(always_shown);
        } else {
            panic!("Always Shown Group should be a group");
        }
        // Check Jane is a member
        if let PresenterGroup::IsGroup(members, _) = &always_shown_group.is_grouped {
            assert!(members.contains("Jane Doe"));
        } else {
            panic!("Always Shown Group should be a group");
        }

        // Test combination: <Name==Group
        let result3 = parse_presenter_data(
            &header,
            "fan_panelist",
            "<Bob Smith==Special Group",
            &mut presenter_map,
        );
        assert_eq!(result3, Some(("Bob Smith".to_string(), true)));

        // Check Bob Smith's always_grouped status and group membership
        let bob_presenter = presenter_map.get("Bob Smith").unwrap();
        let bob_always_grouped = match &bob_presenter.is_member {
            PresenterMember::IsMember(_, always_grouped) => *always_grouped,
            PresenterMember::NotMember => false,
        };
        assert!(bob_always_grouped);
        let mut expected_bob_groups = std::collections::BTreeSet::new();
        expected_bob_groups.insert("Special Group".to_string());
        let bob_groups = match &bob_presenter.is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert_eq!(bob_groups, &expected_bob_groups);

        // Check Special Group exists, is always_shown, and has Bob as member
        let special_group = presenter_map.get("Special Group").unwrap();
        if let PresenterGroup::IsGroup(members, always_shown) = &special_group.is_grouped {
            assert!(always_shown);
            assert!(members.contains("Bob Smith"));
        } else {
            panic!("Special Group should be a group");
        }
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

    fn empty_presenter_map() -> HashMap<String, PresenterInfo> {
        HashMap::new()
    }

    #[test]
    fn test_parse_data_named_simple() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("Yaya Han".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "Yaya Han");
        assert!(credited);
        assert!(pm.contains_key("Yaya Han"));
    }

    #[test]
    fn test_parse_data_named_unlisted() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("Secret Guest".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Unlisted", &mut pm).expect("should parse");
        assert_eq!(uid, "Secret Guest");
        assert!(!credited, "Unlisted should be uncredited");
    }

    #[test]
    fn test_parse_data_named_star_uncredited() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("Helper".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "*Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "Helper");
        assert!(!credited, "* prefix should be uncredited");
    }

    #[test]
    fn test_parse_data_named_with_double_eq_group() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("John==UNC Staff".to_string());
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "John");
        let mut expected_groups = std::collections::BTreeSet::new();
        expected_groups.insert("UNC Staff".to_string());
        let john_groups = match &pm["John"].is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert_eq!(john_groups, &expected_groups);
        let john_always_grouped = match &pm["John"].is_member {
            PresenterMember::IsMember(_, always_grouped) => *always_grouped,
            PresenterMember::NotMember => false,
        };
        assert!(!john_always_grouped);
        // Check that UNC Staff group was created with always_shown=true
        if let Some(unc_staff_info) = pm.get("UNC Staff") {
            let is_always_shown = match &unc_staff_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(
                is_always_shown,
                "UNC Staff should be always_shown due to == prefix"
            );
        } else {
            panic!("UNC Staff group should have been created");
        }
    }

    #[test]
    fn test_parse_data_named_lt_always_grouped() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("<Jane=UNC Staff".to_string());
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "Jane");
        let jane_always_grouped = match &pm["Jane"].is_member {
            PresenterMember::IsMember(_, always_grouped) => *always_grouped,
            PresenterMember::NotMember => false,
        };
        assert!(jane_always_grouped, "< prefix should set always_grouped");
        let mut expected_jane_groups = std::collections::BTreeSet::new();
        expected_jane_groups.insert("UNC Staff".to_string());
        let jane_groups = match &pm["Jane"].is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert_eq!(jane_groups, &expected_jane_groups);
        // Check that UNC Staff group was created but NOT always_shown (single =)
        if let Some(unc_staff_info) = pm.get("UNC Staff") {
            let is_always_shown = match &unc_staff_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(
                !is_always_shown,
                "single = should not set always_shown_group"
            );
        }
    }

    #[test]
    fn test_parse_data_named_lt_double_eq_combined() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("<Bob==Team".to_string());
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "Bob");
        assert!(
            match &pm["Bob"].is_member {
                PresenterMember::IsMember(_, always_grouped) => *always_grouped,
                PresenterMember::NotMember => false,
            },
            "< prefix should set always_grouped"
        );
        // Check that Team group was created with always_shown=true
        if let Some(team_info) = pm.get("Team") {
            let is_always_shown = match &team_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(is_always_shown, "== should set always_shown_group");
        }
    }

    #[test]
    fn test_parse_data_other_simple() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Alice", &mut pm).expect("should parse");
        assert_eq!(uid, "Alice");
        assert!(credited);
        assert!(pm.contains_key("Alice"));
    }

    #[test]
    fn test_parse_data_other_with_group() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Triffin Morris=UNC Staff", &mut pm)
                .expect("should parse");
        assert_eq!(uid, "Triffin Morris");
        let mut expected_triffin_groups = std::collections::BTreeSet::new();
        expected_triffin_groups.insert("UNC Staff".to_string());
        let triffin_groups = match &pm["Triffin Morris"].is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert_eq!(triffin_groups, &expected_triffin_groups);
        // Check that UNC Staff group was created but NOT always_shown (single =)
        if let Some(unc_staff_info) = pm.get("UNC Staff") {
            let is_always_shown = match &unc_staff_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(
                !is_always_shown,
                "single = should not set always_shown_group"
            );
        }
    }

    #[test]
    fn test_parse_data_other_with_double_eq_group() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        let (uid, _credited) = parse_presenter_data(
            &header,
            "guest",
            &"Triffin Morris==UNC Staff".replace("==", "=="),
            &mut pm,
        )
        .expect("should parse");
        assert_eq!(uid, "Triffin Morris");
        // Check that UNC Staff group was created with always_shown=true
        if let Some(unc_staff_info) = pm.get("UNC Staff") {
            let is_always_shown = match &unc_staff_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(is_always_shown, "== should set always_shown_group");
        }
    }

    #[test]
    fn test_parse_data_other_star_uncredited() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        let (uid, credited) = parse_presenter_data(
            &header,
            "guest",
            &"*Triffin Morris=UNC Staff".replace("=", "=="),
            &mut pm,
        )
        .expect("should parse");
        assert_eq!(uid, "Triffin Morris");
        assert!(!credited, "* prefix should be uncredited");
    }

    #[test]
    fn test_parse_data_blank_returns_none() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        assert!(parse_presenter_data(&header, "guest", "", &mut pm).is_none());
        assert!(parse_presenter_data(&header, "guest", "  ", &mut pm).is_none());
    }

    #[test]
    fn test_parse_data_empty_name_with_group() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("==UNC Staff".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "UNC Staff", "empty name should use group as uid");
        assert!(credited);
        assert!(pm.contains_key("UNC Staff"));

        // Verify that UNC Staff is NOT a member of itself (no circular reference)
        let unc_staff_info = pm.get("UNC Staff").unwrap();
        let unc_staff_groups = match &unc_staff_info.is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert!(
            unc_staff_groups.is_empty(),
            "UNC Staff should not have itself as a group"
        );

        // Verify that UNC Staff is not in group_members as a member of itself
        let group_members = match &unc_staff_info.is_grouped {
            PresenterGroup::IsGroup(members, _) => members,
            PresenterGroup::NotGroup => &std::collections::BTreeSet::new(),
        };
        assert!(
            !group_members.contains(&"UNC Staff".to_string()),
            "UNC Staff should not be in group_members as its own member"
        );

        // Verify that UNC Staff is in always_shown_groups (due to == prefix)
        let is_always_shown = match &unc_staff_info.is_grouped {
            PresenterGroup::IsGroup(_, always_shown) => *always_shown,
            PresenterGroup::NotGroup => false,
        };
        assert!(is_always_shown, "UNC Staff should be always_shown");
    }

    #[test]
    fn test_parse_unc_staff_circular_reference_bug() {
        let mut pm = empty_presenter_map();

        // Test case that caused the bug: G:==UNC Staff
        let header = PresenterHeader::Named("==UNC Staff".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");

        assert_eq!(uid, "UNC Staff");
        assert!(credited);

        // Verify the presenter is registered
        assert!(pm.contains_key("UNC Staff"));
        let presenter_info = pm.get("UNC Staff").unwrap();

        // CRITICAL: UNC Staff should not be a member of itself
        let presenter_groups = match &presenter_info.is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert!(
            presenter_groups.is_empty(),
            "UNC Staff should not have any groups when it's the group itself"
        );

        // CRITICAL: UNC Staff should not have itself as a member
        let group_members = match &presenter_info.is_grouped {
            PresenterGroup::IsGroup(members, _) => members,
            PresenterGroup::NotGroup => &std::collections::BTreeSet::new(),
        };
        assert!(
            !group_members.contains(&"UNC Staff".to_string()),
            "UNC Staff should not be listed as a member of itself"
        );

        // UNC Staff should be always_shown due to == prefix
        let is_always_shown = match &presenter_info.is_grouped {
            PresenterGroup::IsGroup(_, always_shown) => *always_shown,
            PresenterGroup::NotGroup => false,
        };
        assert!(is_always_shown, "UNC Staff should be always_shown");

        // Verify is_group flag is set in the final presenter structure
        // This would be set later in the processing pipeline
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

fn read_presenter_ranks(book: &Spreadsheet, _file_path: &str) -> Result<HashMap<String, String>> {
    let mut ranks = HashMap::new();

    // Try to find People sheet
    if let Some(ws) = book.get_sheet_by_name("People") {
        let max_col = ws.get_highest_column();
        let mut header_map = HashMap::new();
        for col in 1..=max_col {
            let value = ws.get_value((col, 1));
            if let Some(key) = canonical_header(&value) {
                header_map.entry(key).or_insert(col);
            }
        }

        if let (Some(name_col), Some(rank_col)) = (
            header_map.get("Name").copied(),
            header_map.get("Rank").copied(),
        ) {
            let highest_row = ws.get_highest_row();
            for row in 2..=highest_row {
                let name = ws.get_value((name_col, row)).trim().to_string();
                let rank = ws.get_value((rank_col, row)).trim().to_string();

                if !name.is_empty() && !rank.is_empty() {
                    ranks.insert(name, rank);
                }
            }
        }
    }

    Ok(ranks)
}
