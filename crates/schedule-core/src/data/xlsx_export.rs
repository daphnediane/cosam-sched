/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use umya_spreadsheet::structs::{Table, TableColumn, TableStyleInfo, Worksheet};

#[allow(unused_imports)]
use super::presenter::{Presenter, PresenterGroup, PresenterMember, PresenterRank};
use super::room::Room;
use super::schedule::Schedule;
use super::source_info::ChangeState;
#[allow(unused_imports)]
use super::{panel::Panel, panel_type::PanelType};

const SCHEDULE_FIXED_HEADERS: &[&str] = &[
    "Uniq ID",
    "Name",
    "Description",
    "Start Time",
    "End Time",
    "Duration",
    "Room",
    "Kind",
    "Cost",
    "Capacity",
    "Difficulty",
    "Note",
    "Prereq",
    "Ticket Sale",
    "Full",
    "Hide Panelist",
    "Alt Panelist",
];

const RANK_ORDER: &[(&str, char)] = &[
    ("guest", 'G'),
    ("judge", 'J'),
    ("staff", 'S'),
    ("invited_guest", 'I'),
    ("fan_panelist", 'P'),
];

const MIN_PANELS_FOR_NAMED_COLUMN: usize = 3;

#[derive(Debug)]
struct ExportPresenterColumn {
    header: String,
    presenter_name: Option<String>,
    rank: String,
    is_other: bool,
}

fn build_presenter_columns(schedule: &Schedule) -> Vec<ExportPresenterColumn> {
    let presenter_map: HashMap<&str, &Presenter> = schedule
        .presenters
        .iter()
        .filter(|p| p.change_state != ChangeState::Deleted)
        .map(|p| (p.name.as_str(), p))
        .collect();

    let mut event_count: HashMap<&str, usize> = HashMap::new();
    // Count presenters from panels (not events)
    for (_, panel) in &schedule.panels {
        if panel.change_state == ChangeState::Deleted {
            continue;
        }

        // Count presenters from panel credited/uncredited lists
        for name in &panel.credited_presenters {
            *event_count.entry(name.as_str()).or_insert(0) += 1;
        }
        for name in &panel.uncredited_presenters {
            *event_count.entry(name.as_str()).or_insert(0) += 1;
        }

        // Also count from parts and sessions
        for part in &panel.parts {
            if part.change_state == ChangeState::Deleted {
                continue;
            }
            for name in &part.credited_presenters {
                *event_count.entry(name.as_str()).or_insert(0) += 1;
            }
            for name in &part.uncredited_presenters {
                *event_count.entry(name.as_str()).or_insert(0) += 1;
            }
            for session in &part.sessions {
                if session.change_state == ChangeState::Deleted {
                    continue;
                }
                // Count presenters from session credited/uncredited lists
                for name in &session.credited_presenters {
                    *event_count.entry(name.as_str()).or_insert(0) += 1;
                }
                for name in &session.uncredited_presenters {
                    *event_count.entry(name.as_str()).or_insert(0) += 1;
                }
            }
        }
    }

    let mut columns = Vec::new();

    for &(rank_str, prefix_char) in RANK_ORDER {
        let mut named_for_rank: Vec<(&str, &Presenter)> = Vec::new();
        let mut has_other = false;

        for (&name, &presenter) in &presenter_map {
            if presenter.rank.as_str() != rank_str {
                continue;
            }
            if presenter.is_group() {
                continue;
            }
            let count = event_count.get(name).copied().unwrap_or(0);
            let has_groups = !presenter.groups().is_empty();
            if has_groups || count >= MIN_PANELS_FOR_NAMED_COLUMN {
                named_for_rank.push((name, presenter));
            } else if count > 0 {
                has_other = true;
            }
        }

        // Check for unknown presenters (not in presenter_map) who have panels
        for (&name, &count) in &event_count {
            if presenter_map.contains_key(name) {
                continue;
            }
            if rank_str == "fan_panelist" && count > 0 {
                has_other = true;
            }
        }

        named_for_rank.sort_by_key(|(name, _)| *name);

        for (name, presenter) in named_for_rank {
            let header = if presenter.always_grouped() {
                if let Some(group) = presenter.groups().first() {
                    format!("{}:{}=={}", prefix_char, name, group)
                } else {
                    format!("{}:{}", prefix_char, name)
                }
            } else if let Some(group) = presenter.groups().first() {
                format!("{}:{}={}", prefix_char, name, group)
            } else {
                format!("{}:{}", prefix_char, name)
            };

            columns.push(ExportPresenterColumn {
                header,
                presenter_name: Some(name.to_string()),
                rank: rank_str.to_string(),
                is_other: false,
            });
        }

        if has_other {
            columns.push(ExportPresenterColumn {
                header: format!("{}:Other", prefix_char),
                presenter_name: None,
                rank: rank_str.to_string(),
                is_other: true,
            });
        }
    }

    columns
}

pub fn export_to_xlsx(schedule: &Schedule, path: &Path) -> Result<()> {
    let mut book = umya_spreadsheet::new_file();

    let presenter_columns = build_presenter_columns(schedule);
    let schedule_headers: Vec<String> = SCHEDULE_FIXED_HEADERS
        .iter()
        .map(|s| s.to_string())
        .chain(presenter_columns.iter().map(|c| c.header.clone()))
        .chain(vec!["Lstart".to_string(), "Lend".to_string()])
        .collect();
    {
        let ws = book
            .get_sheet_mut(&0)
            .ok_or_else(|| anyhow::anyhow!("No default sheet"))?;
        ws.set_name("Schedule");
        let last_row = write_schedule_sheet(ws, schedule, &presenter_columns)?;
        let header_refs: Vec<&str> = schedule_headers.iter().map(|s| s.as_str()).collect();
        add_table(ws, "Schedule", &header_refs, last_row);
    }

    let room_headers = &["Room Name", "Long Name", "Hotel Room", "Sort Key"];
    book.new_sheet("Rooms")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    {
        let ws = book
            .get_sheet_by_name_mut("Rooms")
            .ok_or_else(|| anyhow::anyhow!("Sheet 'Rooms' not found"))?;
        let last_row = write_rooms_sheet(ws, &schedule.rooms);
        add_table(ws, "RoomMap", room_headers, last_row);
    }

    let presenter_headers = &[
        "Name",
        "Rank",
        "Is Group",
        "Members",
        "Groups",
        "Always Grouped",
    ];
    book.new_sheet("People")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    {
        let ws = book
            .get_sheet_by_name_mut("People")
            .ok_or_else(|| anyhow::anyhow!("Sheet 'People' not found"))?;
        let last_row = write_presenters_sheet(ws, &schedule.presenters);
        add_table(ws, "Presenters", presenter_headers, last_row);
    }

    let prefix_headers = &[
        "Prefix",
        "Panel Kind",
        "Color",
        "BW",
        "Is Break",
        "Is Workshop",
        "Is Café",
        "Is Room Hours",
        "Hidden",
        "Is TimeLine",
        "Is Private",
    ];
    book.new_sheet("PanelTypes")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    {
        let ws = book
            .get_sheet_by_name_mut("PanelTypes")
            .ok_or_else(|| anyhow::anyhow!("Sheet 'PanelTypes' not found"))?;
        let last_row = write_panel_types_sheet(ws, &schedule.panel_types);
        add_table(ws, "Prefix", prefix_headers, last_row);
    }

    // Add Grid sheet
    book.new_sheet("Grid").map_err(|e| anyhow::anyhow!("{e}"))?;
    {
        let ws = book
            .get_sheet_by_name_mut("Grid")
            .ok_or_else(|| anyhow::anyhow!("Sheet 'Grid' not found"))?;
        super::xlsx_grid::write_grid_sheet(ws, schedule)?;
    }

    umya_spreadsheet::writer::xlsx::write(&book, path)
        .map_err(|e| anyhow::anyhow!("Failed to write XLSX {}: {e}", path.display()))?;

    Ok(())
}

fn add_table(ws: &mut Worksheet, name: &str, headers: &[&str], last_data_row: u32) {
    let num_cols = headers.len() as u32;
    let last_row = last_data_row.max(2);
    let mut table = Table::new(name, ((1u32, 1u32), (num_cols, last_row)));
    table.set_display_name(name);
    for header in headers {
        table.add_column(TableColumn::new(header));
    }
    let style = TableStyleInfo::new("TableStyleMedium2", false, false, true, false);
    table.set_style_info(Some(style));
    ws.add_table(table);
}

fn set_headers(ws: &mut Worksheet, headers: &[&str]) {
    for (col_0, header) in headers.iter().enumerate() {
        let col = col_0 as u32 + 1;
        ws.get_cell_mut((col, 1)).set_value(*header);
    }
}

fn set_str(ws: &mut Worksheet, col: u32, row: u32, value: &str) {
    ws.get_cell_mut((col, row)).set_value(value);
}

fn set_opt(ws: &mut Worksheet, col: u32, row: u32, value: &Option<String>) {
    if let Some(v) = value {
        ws.get_cell_mut((col, row)).set_value(v.as_str());
    }
}

fn write_rooms_sheet(ws: &mut Worksheet, rooms: &[Room]) -> u32 {
    set_headers(ws, &["Room Name", "Long Name", "Hotel Room", "Sort Key"]);

    let mut row = 2u32;
    for room in rooms {
        if room.change_state == ChangeState::Deleted {
            continue;
        }
        set_str(ws, 1, row, &room.short_name);
        set_str(ws, 2, row, &room.long_name);
        set_str(ws, 3, row, &room.hotel_room);
        set_str(ws, 4, row, &room.sort_key.to_string());
        row += 1;
    }
    row - 1
}

fn write_panel_types_sheet(
    ws: &mut Worksheet,
    panel_types: &indexmap::IndexMap<String, PanelType>,
) -> u32 {
    set_headers(
        ws,
        &[
            "Prefix",
            "Panel Kind",
            "Color",
            "BW",
            "Is Break",
            "Is Workshop",
            "Is Café",
            "Is Room Hours",
            "Hidden",
            "Is TimeLine",
            "Is Private",
        ],
    );

    let mut row = 2u32;
    for (prefix, pt) in panel_types {
        if pt.change_state == ChangeState::Deleted {
            continue;
        }
        set_str(ws, 1, row, prefix);
        set_str(ws, 2, row, &pt.kind);
        if let Some(c) = pt.color() {
            set_str(ws, 3, row, c);
        }
        if let Some(bw) = pt.bw_color() {
            set_str(ws, 4, row, bw);
        }
        if pt.is_break {
            set_str(ws, 5, row, "Yes");
        }
        if pt.is_workshop {
            set_str(ws, 6, row, "Yes");
        }
        if pt.is_cafe {
            set_str(ws, 7, row, "Yes");
        }
        if pt.is_room_hours {
            set_str(ws, 8, row, "Yes");
        }
        if pt.is_hidden {
            set_str(ws, 9, row, "Yes");
        }
        if pt.is_timeline {
            set_str(ws, 10, row, "Yes");
        }
        if pt.is_private {
            set_str(ws, 11, row, "Yes");
        }
        row += 1;
    }

    row - 1
}

/// Represents a flattened session for XLSX export
struct ExportSession {
    id: String,
    name: String,
    description: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    duration: u32,
    room_id: Option<u32>,
    panel_type: Option<String>,
    cost: Option<String>,
    capacity: Option<String>,
    difficulty: Option<String>,
    note: Option<String>,
    prereq: Option<String>,
    ticket_url: Option<String>,
    is_full: bool,
    hide_panelist: bool,
    alt_panelist: Option<String>,
    presenters: Vec<String>,
    // Track which presenters are credited vs uncredited
    credited_presenters: HashSet<String>,
}

/// Flatten the panel hierarchy into exportable sessions
fn flatten_panel_sessions(schedule: &Schedule) -> Vec<ExportSession> {
    let mut sessions = Vec::new();

    for (_, panel) in &schedule.panels {
        if panel.change_state == ChangeState::Deleted {
            continue;
        }

        for part in &panel.parts {
            if part.change_state == ChangeState::Deleted {
                continue;
            }

            for session in &part.sessions {
                if session.change_state == ChangeState::Deleted {
                    continue;
                }

                // Combine presenters from panel, part, and session
                // Use a HashSet to track which presenters are credited
                use std::collections::HashSet;
                let mut presenters = Vec::new();
                let mut credited_presenters: HashSet<&str> = HashSet::new();
                let mut all_presenters: HashSet<&str> = HashSet::new();

                // Collect all presenters and track which are credited
                // Panel level
                for name in &panel.credited_presenters {
                    credited_presenters.insert(name.as_str());
                    all_presenters.insert(name.as_str());
                }
                for name in &panel.uncredited_presenters {
                    all_presenters.insert(name.as_str());
                }

                // Part level
                for name in &part.credited_presenters {
                    credited_presenters.insert(name.as_str());
                    all_presenters.insert(name.as_str());
                }
                for name in &part.uncredited_presenters {
                    all_presenters.insert(name.as_str());
                }

                // Session level
                for name in &session.credited_presenters {
                    credited_presenters.insert(name.as_str());
                    all_presenters.insert(name.as_str());
                }
                for name in &session.uncredited_presenters {
                    all_presenters.insert(name.as_str());
                }

                // Build the presenters list (for ExportSession)
                presenters.extend(all_presenters.iter().map(|&s| s.to_string()));

                // Convert HashSet to String HashSet for storage
                let credited_set: HashSet<String> =
                    credited_presenters.iter().map(|&s| s.to_string()).collect();

                // Use session-specific room if available, otherwise fall back to first room
                let room_id = session.room_ids.first().copied();

                sessions.push(ExportSession {
                    id: session.id.clone(),
                    name: panel.name.clone(),
                    description: session
                        .description
                        .as_ref()
                        .or_else(|| part.description.as_ref())
                        .or_else(|| panel.description.as_ref())
                        .cloned(),
                    start_time: session.start_time.clone(),
                    end_time: session.end_time.clone(),
                    duration: session.duration,
                    room_id,
                    panel_type: panel.panel_type.clone(),
                    cost: panel.cost.clone(),
                    capacity: session
                        .capacity
                        .as_ref()
                        .or_else(|| panel.capacity.as_ref())
                        .cloned(),
                    difficulty: panel.difficulty.clone(),
                    note: session
                        .note
                        .as_ref()
                        .or_else(|| part.note.as_ref())
                        .or_else(|| panel.note.as_ref())
                        .cloned(),
                    prereq: session
                        .prereq
                        .as_ref()
                        .or_else(|| part.prereq.as_ref())
                        .or_else(|| panel.prereq.as_ref())
                        .cloned(),
                    ticket_url: panel.ticket_url.clone(),
                    is_full: session.is_full,
                    hide_panelist: panel.alt_panelist.is_some(), // Approximation
                    alt_panelist: session
                        .alt_panelist
                        .as_ref()
                        .or_else(|| part.alt_panelist.as_ref())
                        .or_else(|| panel.alt_panelist.as_ref())
                        .cloned(),
                    presenters,
                    credited_presenters: credited_set,
                });
            }
        }
    }

    sessions
}

fn write_schedule_sheet(
    ws: &mut Worksheet,
    schedule: &Schedule,
    presenter_columns: &[ExportPresenterColumn],
) -> Result<u32> {
    let fixed_count = SCHEDULE_FIXED_HEADERS.len() as u32;
    for (col_0, header) in SCHEDULE_FIXED_HEADERS.iter().enumerate() {
        let col = col_0 as u32 + 1;
        ws.get_cell_mut((col, 1)).set_value(*header);
    }
    for (i, pcol) in presenter_columns.iter().enumerate() {
        let col = fixed_count + i as u32 + 1;
        ws.get_cell_mut((col, 1)).set_value(pcol.header.as_str());
    }

    // Add Lstart and Lend headers after presenter columns
    let lstart_col = fixed_count + presenter_columns.len() as u32 + 1;
    let lend_col = fixed_count + presenter_columns.len() as u32 + 2;
    ws.get_cell_mut((lstart_col, 1)).set_value("Lstart");
    ws.get_cell_mut((lend_col, 1)).set_value("Lend");

    let presenter_map: HashMap<&str, &Presenter> = schedule
        .presenters
        .iter()
        .filter(|p| p.change_state != ChangeState::Deleted)
        .map(|p| (p.name.as_str(), p))
        .collect();

    let named_presenters: HashMap<&str, u32> = presenter_columns
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            c.presenter_name
                .as_deref()
                .map(|name| (name, fixed_count + i as u32 + 1))
        })
        .collect();

    let other_columns: Vec<(usize, &ExportPresenterColumn)> = presenter_columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_other)
        .collect();

    let mut row = 2u32;

    let sessions = flatten_panel_sessions(schedule);

    for session in sessions {
        set_str(ws, 1, row, &session.id);
        set_str(ws, 2, row, &session.name);
        set_opt(ws, 3, row, &session.description);

        if let Some(start_time) = &session.start_time {
            set_str(ws, 4, row, start_time);
        }

        if let Some(end_time) = &session.end_time {
            set_str(ws, 5, row, end_time);
        }

        set_str(ws, 6, row, &session.duration.to_string());

        let room_name = session
            .room_id
            .and_then(|rid| schedule.room_by_id(rid))
            .map(|r| r.short_name.as_str())
            .unwrap_or("");
        set_str(ws, 7, row, room_name);

        let kind = session
            .panel_type
            .as_ref()
            .and_then(|pt_uid| schedule.panel_types.get(pt_uid))
            .map(|pt| pt.kind.as_str())
            .unwrap_or("");
        set_str(ws, 8, row, kind);

        set_opt(ws, 9, row, &session.cost);
        set_opt(ws, 10, row, &session.capacity);
        set_opt(ws, 11, row, &session.difficulty);
        set_opt(ws, 12, row, &session.note);
        set_opt(ws, 13, row, &session.prereq);
        set_opt(ws, 14, row, &session.ticket_url);
        if session.is_full {
            set_str(ws, 15, row, "Yes");
        }
        if session.hide_panelist {
            set_str(ws, 16, row, "Yes");
        }
        set_opt(ws, 17, row, &session.alt_panelist);

        let mut other_names: HashMap<&str, Vec<&str>> = HashMap::new();

        // Handle all presenters using the stored credited/uncredited sets
        for presenter_name in &session.presenters {
            if let Some(&col) = named_presenters.get(presenter_name.as_str()) {
                // Check if this presenter is credited
                if session.credited_presenters.contains(presenter_name) {
                    set_str(ws, col, row, "Yes");
                } else {
                    set_str(ws, col, row, "*");
                }
            } else {
                // Not a named column, add to Other
                let rank = presenter_map
                    .get(presenter_name.as_str())
                    .map(|p| p.rank.as_str())
                    .unwrap_or("fan_panelist");

                if session.credited_presenters.contains(presenter_name) {
                    other_names
                        .entry(rank)
                        .or_default()
                        .push(presenter_name.as_str());
                } else {
                    // Add with * prefix for uncredited
                    let prefixed = format!("*{}", presenter_name);
                    other_names
                        .entry(rank)
                        .or_default()
                        .push(Box::leak(prefixed.into_boxed_str()));
                }
            }
        }

        // Handle Other columns
        for &(i, ref ocol) in &other_columns {
            let rank = ocol.rank.as_str();
            if let Some(names) = other_names.get(rank) {
                let col = fixed_count + i as u32 + 1;
                set_str(ws, col, row, &names.join(", "));
            }
        }

        // Set Lstart formula (dynamic column position)
        let lstart_formula =
            "IF(ISBLANK([@[Start Time]]),MAX([Start Time])+TIME(80,0,0),[@[Start Time]])";
        let cell = ws.get_cell_mut((lstart_col, row));
        // Try to set as formula (without = prefix)
        cell.set_formula(lstart_formula);
        // Also set the calculated value for Excel compatibility
        if let Some(start_time) = &session.start_time {
            // For events, use the actual start time
            cell.set_value(start_time);
        } else {
            // For timeline entries, this will be handled separately
            cell.set_value("");
        }

        // Set Lend formula (dynamic column position)
        let lend_formula = "=[@Lstart]+IF(ISBLANK([@Duration]),0,[@Duration])";
        let cell = ws.get_cell_mut((lend_col, row));
        // Remove the = prefix for set_formula
        let lend_formula_clean = &lend_formula[1..];
        cell.set_formula(lend_formula_clean);
        // Also set the calculated value for Excel compatibility
        if let Some(start_time) = &session.start_time {
            // Calculate end time: start_time + duration
            use chrono::Duration;
            let end_time = if session.duration > 0 {
                // Parse the start_time string and add duration minutes
                chrono::NaiveDateTime::parse_from_str(start_time, "%-m/%-d/%Y %-I:%M %p")
                    .map(|dt| dt + Duration::minutes(session.duration as i64))
                    .map(|dt| dt.format("%-m/%-d/%Y %-I:%M %p").to_string())
                    .unwrap_or_else(|_| start_time.clone())
            } else {
                start_time.clone()
            };
            cell.set_value(&end_time);
        } else {
            cell.set_value("");
        }

        row += 1;
    }

    for entry in &schedule.timeline {
        if entry.change_state == ChangeState::Deleted {
            continue;
        }
        let start_time: NaiveDateTime = entry
            .start_time
            .parse()
            .with_context(|| format!("Invalid timeline start time: {}", entry.start_time))?;
        let end_time = start_time + chrono::Duration::minutes(30);

        let prefix = entry
            .panel_type
            .as_deref()
            .unwrap_or("SPLIT")
            .to_uppercase();

        set_str(ws, 1, row, &entry.id);
        set_str(ws, 2, row, &entry.description);
        set_opt(ws, 3, row, &entry.note);
        set_str(
            ws,
            4,
            row,
            &start_time.format("%-m/%-d/%Y %-I:%M %p").to_string(),
        );
        set_str(
            ws,
            5,
            row,
            &end_time.format("%-m/%-d/%Y %-I:%M %p").to_string(),
        );
        set_str(ws, 6, row, "30");
        set_str(ws, 8, row, &prefix);

        // Set Lstart formula for timeline entries (dynamic column position)
        let lstart_formula = "=[@[Start Time]]";
        let cell = ws.get_cell_mut((lstart_col, row));
        cell.set_formula(&lstart_formula[1..]); // Remove = prefix
        cell.set_value(&start_time.format("%-m/%-d/%Y %-I:%M %p").to_string());

        // Set Lend formula for timeline entries (dynamic column position)
        let lend_formula = "=[@[End Time]]";
        let cell = ws.get_cell_mut((lend_col, row));
        cell.set_formula(&lend_formula[1..]); // Remove = prefix
        cell.set_value(&end_time.format("%-m/%-d/%Y %-I:%M %p").to_string());

        row += 1;
    }

    Ok(row - 1)
}

fn write_presenters_sheet(ws: &mut Worksheet, presenters: &[Presenter]) -> u32 {
    set_headers(
        ws,
        &[
            "Name",
            "Rank",
            "Is Group",
            "Members",
            "Groups",
            "Always Grouped",
        ],
    );

    let mut row = 2u32;
    for presenter in presenters {
        if presenter.change_state == ChangeState::Deleted {
            continue;
        }
        set_str(ws, 1, row, &presenter.name);
        set_str(ws, 2, row, presenter.rank.as_str());
        if presenter.is_group() {
            set_str(ws, 3, row, "Yes");
        }
        if !presenter.members().is_empty() {
            set_str(
                ws,
                4,
                row,
                &presenter
                    .members()
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        if !presenter.groups().is_empty() {
            set_str(
                ws,
                5,
                row,
                &presenter
                    .groups()
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        if presenter.always_grouped() {
            set_str(ws, 6, row, "Yes");
        }
        row += 1;
    }
    row - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::event::Event;
    use crate::data::panel::{PanelPart, PanelSession};
    use crate::data::schedule::Meta;
    use crate::data::source_info::ImportedSheetPresence;

    fn make_test_schedule() -> Schedule {
        #[allow(unused_variables)]
        let dt = |s: &str| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap();

        Schedule {
            conflicts: Vec::new(),
            meta: Meta {
                title: "Test".to_string(),
                generated: "2026-01-01".to_string(),
                version: Some(4),
                variant: None,
                generator: None,
                start_time: None,
                end_time: None,
                next_presenter_id: None,
                creator: None,
                last_modified_by: None,
                modified: None,
            },
            timeline: Vec::new(),
            panels: {
                let mut panels = indexmap::IndexMap::new();
                panels.insert(
                    "GP001".to_string(),
                    Panel {
                        id: "GP001".to_string(),
                        name: "Test Panel".to_string(),
                        panel_type: Some("panel-type-GP".to_string()),
                        description: None,
                        note: None,
                        prereq: None,
                        alt_panelist: None,
                        cost: None,
                        capacity: None,
                        pre_reg_max: None,
                        difficulty: None,
                        ticket_url: None,
                        is_free: false,
                        is_kids: false,
                        credited_presenters: vec![],
                        uncredited_presenters: vec![],
                        simple_tix_event: None,
                        have_ticket_image: None,
                        parts: vec![PanelPart {
                            part_num: None,
                            description: None,
                            note: None,
                            prereq: None,
                            alt_panelist: None,
                            credited_presenters: vec![],
                            uncredited_presenters: vec!["Alice".to_string()],
                            sessions: vec![PanelSession {
                                id: "GP002S1".to_string(),
                                session_num: Some(1),
                                description: None,
                                note: None,
                                prereq: None,
                                alt_panelist: None,
                                room_ids: vec![1],
                                start_time: Some("2026-01-01T10:00:00".to_string()),
                                end_time: Some("2026-01-01T11:00:00".to_string()),
                                duration: 60,
                                is_full: false,
                                capacity: None,
                                seats_sold: None,
                                pre_reg_max: None,
                                ticket_url: None,
                                simple_tix_event: None,
                                hide_panelist: false,
                                credited_presenters: vec!["Alice".to_string()],
                                uncredited_presenters: vec![],
                                notes_non_printing: None,
                                workshop_notes: None,
                                power_needs: None,
                                sewing_machines: false,
                                av_notes: None,
                                source: None,
                                change_state: ChangeState::Unchanged,
                                conflicts: vec![],
                                metadata: indexmap::IndexMap::new(),
                            }],
                            change_state: ChangeState::Unchanged,
                        }],
                        metadata: None,
                        change_state: ChangeState::Unchanged,
                    },
                );
                panels
            },
            rooms: vec![Room {
                uid: 1,
                short_name: "Main".to_string(),
                long_name: "Main Hall".to_string(),
                hotel_room: "Grand Ballroom".to_string(),
                sort_key: 1,
                is_break: false,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            }],
            panel_types: {
                let mut pt_map = indexmap::IndexMap::new();
                let mut colors = indexmap::IndexMap::new();
                colors.insert("color".to_string(), "#E2F9D7".to_string());
                pt_map.insert(
                    "GP".to_string(),
                    PanelType {
                        prefix: "GP".to_string(),
                        kind: "Guest Panel".to_string(),
                        colors,
                        is_break: false,
                        is_cafe: false,
                        is_workshop: false,
                        is_hidden: false,
                        is_room_hours: false,
                        is_timeline: false,
                        is_private: false,
                        metadata: None,
                        source: None,
                        change_state: ChangeState::Unchanged,
                    },
                );
                pt_map
            },
            presenters: vec![Presenter {
                id: None,
                name: "Alice".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            }],
            imported_sheets: ImportedSheetPresence::default(),
        }
    }

    #[test]
    fn test_export_roundtrip() {
        let schedule = make_test_schedule();
        let dir = std::env::temp_dir();
        let path = dir.join("test_export_roundtrip.xlsx");

        export_to_xlsx(&schedule, &path).expect("export should succeed");
        assert!(path.exists(), "XLSX file should be created");

        let book =
            umya_spreadsheet::reader::xlsx::read(&path).expect("should read back exported XLSX");

        let room_ws = book
            .get_sheet_by_name("Rooms")
            .expect("Rooms sheet should exist");
        assert_eq!(room_ws.get_value((1, 1)), "Room Name");
        assert_eq!(room_ws.get_value((1, 2)), "Main");

        let prefix_ws = book
            .get_sheet_by_name("PanelTypes")
            .expect("PanelTypes sheet should exist");
        assert_eq!(prefix_ws.get_value((1, 1)), "Prefix");
        assert_eq!(prefix_ws.get_value((1, 2)), "GP");

        let sched_ws = book
            .get_sheet_by_name("Schedule")
            .expect("Schedule sheet should exist");
        assert_eq!(sched_ws.get_value((1, 1)), "Uniq ID");
        assert_eq!(sched_ws.get_value((1, 2)), "GP002S1");
        assert_eq!(sched_ws.get_value((2, 2)), "Test Panel");

        let fixed_col_count = SCHEDULE_FIXED_HEADERS.len() as u32;
        let other_col = fixed_col_count + 1;
        assert_eq!(sched_ws.get_value((other_col, 1)), "G:Other");
        assert_eq!(sched_ws.get_value((other_col, 2)), "Alice");

        let pres_ws = book
            .get_sheet_by_name("People")
            .expect("People sheet should exist");
        assert_eq!(pres_ws.get_value((1, 1)), "Name");
        assert_eq!(pres_ws.get_value((1, 2)), "Alice");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_skips_deleted() {
        let mut schedule = make_test_schedule();
        schedule.rooms.push(Room {
            uid: 2,
            short_name: "Deleted".to_string(),
            long_name: "Gone".to_string(),
            hotel_room: "".to_string(),
            sort_key: 99,
            is_break: false,
            metadata: None,
            source: None,
            change_state: ChangeState::Deleted,
        });

        let dir = std::env::temp_dir();
        let path = dir.join("test_export_skips_deleted.xlsx");

        export_to_xlsx(&schedule, &path).expect("export should succeed");

        let book =
            umya_spreadsheet::reader::xlsx::read(&path).expect("should read back exported XLSX");
        let room_ws = book.get_sheet_by_name("Rooms").unwrap();
        assert_eq!(room_ws.get_value((1, 2)), "Main");
        assert_eq!(
            room_ws.get_value((1, 3)),
            "",
            "Deleted room should not appear"
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_presenter_columns() {
        let dt = |s: &str| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap();
        #[allow(unused_variables)]
        let make_event = |id: &str, presenters: Vec<&str>| Event {
            id: id.to_string(),
            name: format!("Panel {id}"),
            description: None,
            start_time: dt("2026-06-26T09:00:00"),
            end_time: dt("2026-06-26T10:00:00"),
            duration: 60,
            room_id: Some(1),
            panel_type: Some("panel-type-gp".to_string()),
            cost: None,
            capacity: None,
            difficulty: None,
            note: None,
            prereq: None,
            ticket_url: None,
            presenters: presenters.into_iter().map(String::from).collect(),
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

        let mut schedule = make_test_schedule();
        schedule.presenters = vec![
            Presenter {
                id: None,
                name: "Pro".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::IsMember(
                    {
                        let mut groups = std::collections::BTreeSet::new();
                        groups.insert("Pros and Cons".to_string());
                        groups
                    },
                    false,
                ),
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                id: None,
                name: "Con".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::IsMember(
                    {
                        let mut groups = std::collections::BTreeSet::new();
                        groups.insert("Pros and Cons".to_string());
                        groups
                    },
                    true, // always_grouped
                ),
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                id: None,
                name: "Pros and Cons".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::IsGroup(
                    {
                        let mut members = std::collections::BTreeSet::new();
                        members.insert("Pro".to_string());
                        members.insert("Con".to_string());
                        members
                    },
                    false,
                ),
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                id: None,
                name: "Bob".to_string(),
                rank: PresenterRank::from_str("fan_panelist"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
        ];
        // Create test panels with presenters
        let mut panels = indexmap::IndexMap::new();
        panels.insert(
            "GP001".to_string(),
            Panel {
                id: "GP001".to_string(),
                name: "Panel 1".to_string(),
                panel_type: Some("panel-type-GP".to_string()),
                description: None,
                note: None,
                prereq: None,
                alt_panelist: None,
                cost: None,
                capacity: None,
                pre_reg_max: None,
                difficulty: None,
                ticket_url: None,
                is_free: false,
                is_kids: false,
                credited_presenters: vec!["Pro".to_string(), "Con".to_string()],
                uncredited_presenters: vec![],
                simple_tix_event: None,
                have_ticket_image: None,
                parts: vec![PanelPart {
                    part_num: None,
                    description: None,
                    note: None,
                    prereq: None,
                    alt_panelist: None,
                    credited_presenters: vec!["Pro".to_string(), "Con".to_string()],
                    uncredited_presenters: vec![],
                    sessions: vec![PanelSession {
                        id: "GP001S1".to_string(),
                        session_num: Some(1),
                        description: None,
                        note: None,
                        prereq: None,
                        alt_panelist: None,
                        room_ids: vec![1],
                        start_time: Some("2026-01-01T10:00:00".to_string()),
                        end_time: Some("2026-01-01T11:00:00".to_string()),
                        duration: 60,
                        is_full: false,
                        capacity: None,
                        seats_sold: None,
                        pre_reg_max: None,
                        ticket_url: None,
                        simple_tix_event: None,
                        hide_panelist: false,
                        credited_presenters: vec!["Pro".to_string(), "Con".to_string()],
                        uncredited_presenters: vec![],
                        notes_non_printing: None,
                        workshop_notes: None,
                        power_needs: None,
                        sewing_machines: false,
                        av_notes: None,
                        source: None,
                        change_state: ChangeState::Unchanged,
                        conflicts: vec![],
                        metadata: indexmap::IndexMap::new(),
                    }],
                    change_state: ChangeState::Unchanged,
                }],
                metadata: None,
                change_state: ChangeState::Unchanged,
            },
        );
        panels.insert(
            "GP002".to_string(),
            Panel {
                id: "GP002".to_string(),
                name: "Panel 2".to_string(),
                panel_type: Some("panel-type-GP".to_string()),
                description: None,
                note: None,
                prereq: None,
                alt_panelist: None,
                cost: None,
                capacity: None,
                pre_reg_max: None,
                difficulty: None,
                ticket_url: None,
                is_free: false,
                is_kids: false,
                credited_presenters: vec!["Pro".to_string()],
                uncredited_presenters: vec![],
                simple_tix_event: None,
                have_ticket_image: None,
                parts: vec![PanelPart {
                    part_num: None,
                    description: None,
                    note: None,
                    prereq: None,
                    alt_panelist: None,
                    credited_presenters: vec!["Pro".to_string(), "Con".to_string()],
                    uncredited_presenters: vec![],
                    sessions: vec![PanelSession {
                        id: "GP002S1".to_string(),
                        session_num: Some(1),
                        description: None,
                        note: None,
                        prereq: None,
                        alt_panelist: None,
                        room_ids: vec![1],
                        start_time: Some("2026-01-01T10:00:00".to_string()),
                        end_time: Some("2026-01-01T11:00:00".to_string()),
                        duration: 60,
                        is_full: false,
                        capacity: None,
                        seats_sold: None,
                        pre_reg_max: None,
                        ticket_url: None,
                        simple_tix_event: None,
                        hide_panelist: false,
                        credited_presenters: vec!["Pro".to_string(), "Con".to_string()],
                        uncredited_presenters: vec![],
                        notes_non_printing: None,
                        workshop_notes: None,
                        power_needs: None,
                        sewing_machines: false,
                        av_notes: None,
                        source: None,
                        change_state: ChangeState::Unchanged,
                        conflicts: vec![],
                        metadata: indexmap::IndexMap::new(),
                    }],
                    change_state: ChangeState::Unchanged,
                }],
                metadata: None,
                change_state: ChangeState::Unchanged,
            },
        );
        panels.insert(
            "GP003".to_string(),
            Panel {
                id: "GP003".to_string(),
                name: "Panel 3".to_string(),
                panel_type: Some("panel-type-GP".to_string()),
                description: None,
                note: None,
                prereq: None,
                alt_panelist: None,
                cost: None,
                capacity: None,
                pre_reg_max: None,
                difficulty: None,
                ticket_url: None,
                is_free: false,
                is_kids: false,
                credited_presenters: vec!["Pro".to_string(), "Bob".to_string()],
                uncredited_presenters: vec![],
                simple_tix_event: None,
                have_ticket_image: None,
                parts: vec![PanelPart {
                    part_num: None,
                    description: None,
                    note: None,
                    prereq: None,
                    alt_panelist: None,
                    credited_presenters: vec!["Pro".to_string(), "Con".to_string()],
                    uncredited_presenters: vec![],
                    sessions: vec![PanelSession {
                        id: "GP003S1".to_string(),
                        session_num: Some(1),
                        description: None,
                        note: None,
                        prereq: None,
                        alt_panelist: None,
                        room_ids: vec![1],
                        start_time: Some("2026-01-01T10:00:00".to_string()),
                        end_time: Some("2026-01-01T11:00:00".to_string()),
                        duration: 60,
                        is_full: false,
                        capacity: None,
                        seats_sold: None,
                        pre_reg_max: None,
                        ticket_url: None,
                        simple_tix_event: None,
                        hide_panelist: false,
                        credited_presenters: vec!["Pro".to_string(), "Con".to_string()],
                        uncredited_presenters: vec![],
                        notes_non_printing: None,
                        workshop_notes: None,
                        power_needs: None,
                        sewing_machines: false,
                        av_notes: None,
                        source: None,
                        change_state: ChangeState::Unchanged,
                        conflicts: vec![],
                        metadata: indexmap::IndexMap::new(),
                    }],
                    change_state: ChangeState::Unchanged,
                }],
                metadata: None,
                change_state: ChangeState::Unchanged,
            },
        );
        schedule.panels = panels;

        let columns = build_presenter_columns(&schedule);
        let headers: Vec<&str> = columns.iter().map(|c| c.header.as_str()).collect();

        assert!(
            headers.contains(&"G:Con==Pros and Cons"),
            "Con should have ==Group header, got: {headers:?}"
        );
        assert!(
            headers.contains(&"G:Pro=Pros and Cons"),
            "Pro should have =Group header, got: {headers:?}"
        );
        assert!(
            headers.contains(&"P:Other"),
            "Bob (1 panel) should go to P:Other, got: {headers:?}"
        );
        assert!(
            !headers
                .iter()
                .any(|h| h.contains("Pros and Cons") && h.starts_with("G:Pros")),
            "Group entity 'Pros and Cons' should not get its own column"
        );

        let dir = std::env::temp_dir();
        let path = dir.join("test_export_presenter_columns.xlsx");
        export_to_xlsx(&schedule, &path).expect("export should succeed");

        let book =
            umya_spreadsheet::reader::xlsx::read(&path).expect("should read back exported XLSX");
        let sched_ws = book.get_sheet_by_name("Schedule").unwrap();

        let fixed = SCHEDULE_FIXED_HEADERS.len() as u32;
        let mut pro_col = None;
        let mut other_col = None;
        for i in 0..columns.len() {
            let col = fixed + i as u32 + 1;
            let header = sched_ws.get_value((col, 1));
            if header.starts_with("G:Pro=") {
                pro_col = Some(col);
            }
            if header == "P:Other" {
                other_col = Some(col);
            }
        }

        let pro_col = pro_col.expect("Pro column should exist");
        assert_eq!(sched_ws.get_value((pro_col, 2)), "Yes");
        assert_eq!(sched_ws.get_value((pro_col, 3)), "Yes");
        assert_eq!(sched_ws.get_value((pro_col, 4)), "Yes");

        let other_col = other_col.expect("P:Other column should exist");
        assert_eq!(sched_ws.get_value((other_col, 2)), "");
        assert_eq!(sched_ws.get_value((other_col, 4)), "Bob");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_credited_uncredited_presenters() {
        let mut schedule = make_test_schedule();

        // Add presenters with different credit status
        schedule.presenters = vec![
            Presenter {
                id: None,
                name: "Alice".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                id: None,
                name: "Bob".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                id: None,
                name: "Charlie".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
        ];

        // Create test panels with mixed credited/uncredited presenters
        // Alice appears in 3 panels, so she gets her own column
        // Bob appears in only 1 panel, so he goes to Other column
        let mut panels = indexmap::IndexMap::new();
        for (i, panel_id) in ["GP001", "GP002", "GP003"].iter().enumerate() {
            panels.insert(
                panel_id.to_string(),
                Panel {
                    id: panel_id.to_string(),
                    name: format!("Panel {}", i + 1),
                    panel_type: Some("panel-type-GP".to_string()),
                    description: None,
                    note: None,
                    prereq: None,
                    alt_panelist: None,
                    cost: None,
                    capacity: None,
                    pre_reg_max: None,
                    difficulty: None,
                    ticket_url: None,
                    is_free: false,
                    is_kids: false,
                    credited_presenters: vec!["Alice".to_string()],
                    uncredited_presenters: if i == 0 {
                        vec!["Bob".to_string()]
                    } else {
                        vec![]
                    },
                    simple_tix_event: None,
                    have_ticket_image: None,
                    parts: vec![PanelPart {
                        part_num: None,
                        description: None,
                        note: None,
                        prereq: None,
                        alt_panelist: None,
                        credited_presenters: vec!["Alice".to_string()],
                        uncredited_presenters: if i == 0 {
                            vec!["Bob".to_string()]
                        } else {
                            vec![]
                        },
                        sessions: vec![PanelSession {
                            id: format!("{}S1", panel_id),
                            session_num: Some(1),
                            description: None,
                            note: None,
                            prereq: None,
                            alt_panelist: None,
                            room_ids: vec![1],
                            start_time: Some("2026-01-01T10:00:00".to_string()),
                            end_time: Some("2026-01-01T11:00:00".to_string()),
                            duration: 60,
                            is_full: false,
                            capacity: None,
                            seats_sold: None,
                            pre_reg_max: None,
                            ticket_url: None,
                            simple_tix_event: None,
                            hide_panelist: false,
                            credited_presenters: vec!["Alice".to_string()],
                            uncredited_presenters: if i == 0 {
                                vec!["Bob".to_string()]
                            } else {
                                vec![]
                            },
                            notes_non_printing: None,
                            workshop_notes: None,
                            power_needs: None,
                            sewing_machines: false,
                            av_notes: None,
                            source: None,
                            change_state: ChangeState::Unchanged,
                            conflicts: vec![],
                            metadata: indexmap::IndexMap::new(),
                        }],
                        change_state: ChangeState::Unchanged,
                    }],
                    metadata: None,
                    change_state: ChangeState::Unchanged,
                },
            );
        }
        schedule.panels = panels;

        let dir = std::env::temp_dir();
        let path = dir.join("test_credited_uncredited.xlsx");
        export_to_xlsx(&schedule, &path).expect("export should succeed");
        use crate::data::timeline::TimelineEntry;
        use crate::data::xlsx_import::{XlsxImportOptions, import_xlsx};
        use std::env;

        let path = env::temp_dir().join("timeline_test.xlsx");

        // Create a schedule with timeline entries
        let mut schedule = Schedule {
            conflicts: Vec::new(),
            meta: Meta {
                title: "Timeline Test".to_string(),
                generated: "2026-01-01".to_string(),
                version: Some(7),
                variant: Some("full".to_string()),
                generator: None,
                start_time: None,
                end_time: None,
                next_presenter_id: None,
                creator: None,
                last_modified_by: None,
                modified: None,
            },
            timeline: vec![
                TimelineEntry {
                    id: "SPLIT01".to_string(),
                    start_time: "2026-06-26T09:00:00".to_string(),
                    description: "Opening Ceremony".to_string(),
                    panel_type: Some("SP".to_string()),
                    note: Some("Welcome everyone".to_string()),
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Unchanged,
                },
                TimelineEntry {
                    id: "SPLIT02".to_string(),
                    start_time: "2026-06-26T18:00:00".to_string(),
                    description: "Closing Ceremony".to_string(),
                    panel_type: Some("SP".to_string()),
                    note: None,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Unchanged,
                },
            ],
            panels: indexmap::IndexMap::new(),
            rooms: vec![Room {
                uid: 1,
                short_name: "Main Hall".to_string(),
                long_name: "Main Hall".to_string(),
                hotel_room: "Ballroom".to_string(),
                sort_key: 1,
                is_break: false,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            }],
            panel_types: indexmap::IndexMap::new(),
            presenters: Vec::new(),
            imported_sheets: Default::default(),
        };

        // Add a timeline panel type
        schedule.panel_types.insert(
            "SP".to_string(),
            PanelType {
                prefix: "SP".to_string(),
                kind: "Timeline Entry".to_string(),
                colors: indexmap::IndexMap::new(),
                is_break: false,
                is_cafe: false,
                is_workshop: false,
                is_hidden: false,
                is_room_hours: false,
                is_timeline: true,
                is_private: false,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
        );

        // Export to XLSX
        export_to_xlsx(&schedule, &path).unwrap();

        // Import back
        let import_options = XlsxImportOptions {
            title: "Timeline Test".to_string(),
            schedule_table: "Schedule".to_string(),
            rooms_table: "Rooms".to_string(),
            panel_types_table: "Panel_Types".to_string(),
            use_modified_as_generated: false,
        };
        let imported_schedule = import_xlsx(&path, &import_options).unwrap();

        // Verify timeline entries are preserved
        assert_eq!(
            imported_schedule.timeline.len(),
            2,
            "Should have 2 timeline entries"
        );

        let first_entry = &imported_schedule.timeline[0];
        assert_eq!(first_entry.id, "SPLIT01");
        assert_eq!(first_entry.description, "Opening Ceremony");
        assert_eq!(first_entry.start_time, "2026-06-26T09:00:00");
        assert_eq!(first_entry.panel_type, Some("SP".to_string()));
        assert_eq!(first_entry.note, None); // Note field not exported in current implementation

        let second_entry = &imported_schedule.timeline[1];
        assert_eq!(second_entry.id, "SPLIT02");
        assert_eq!(second_entry.description, "Closing Ceremony");
        assert_eq!(second_entry.start_time, "2026-06-26T18:00:00");
        assert_eq!(second_entry.panel_type, Some("SP".to_string()));
        assert_eq!(second_entry.note, None);

        std::fs::remove_file(&path).ok();
    }
}
