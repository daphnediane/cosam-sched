/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use umya_spreadsheet::structs::Worksheet;

#[allow(unused_imports)]
use crate::data::presenter::{Presenter, PresenterGroup, PresenterMember, PresenterRank};
use crate::data::room::Room;
use crate::data::schedule::Schedule;
use crate::data::source_info::ChangeState;
use crate::data::time;
#[allow(unused_imports)]
use crate::data::{panel::Panel, panel_type::PanelType};
use crate::file::ScheduleFile;
use crate::xlsx::columns::people;

use super::common::{SCHEDULE_FIXED_HEADERS, add_table, flatten_panel_sessions};

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
    for (_, panel) in &schedule.panels {
        if panel.change_state == ChangeState::Deleted {
            continue;
        }

        for name in &panel.credited_presenters {
            *event_count.entry(name.as_str()).or_insert(0) += 1;
        }
        for name in &panel.uncredited_presenters {
            *event_count.entry(name.as_str()).or_insert(0) += 1;
        }

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

    for rank_enum in crate::data::presenter::PresenterRank::standard_ranks() {
        let rank_str = rank_enum.as_str();
        let prefix_char = rank_enum.prefix_char();
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

pub fn export_to_xlsx(sf: &ScheduleFile, path: &Path) -> Result<()> {
    let schedule = &sf.schedule;
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
        people::NAME.export,
        people::CLASSIFICATION.export,
        people::IS_GROUP.export,
        people::MEMBERS.export,
        people::GROUPS.export,
        people::ALWAYS_GROUPED.export,
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
        super::grid::write_grid_sheet(ws, schedule)?;
    }

    umya_spreadsheet::writer::xlsx::write(&book, path)
        .map_err(|e| anyhow::anyhow!("Failed to write XLSX {}: {e}", path.display()))?;

    Ok(())
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

    let sessions = flatten_panel_sessions(schedule, false);

    for session in &sessions {
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
        for presenter_name in &session.all_presenters {
            if let Some(&col) = named_presenters.get(presenter_name.as_str()) {
                // Check if this presenter is credited
                if session.credited_set.contains(presenter_name) {
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

                if session.credited_set.contains(presenter_name) {
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
        cell.set_formula(lstart_formula);
        // Also set the calculated value for Excel compatibility
        if let Some(start_time) = &session.start_time {
            cell.set_value(start_time);
        } else {
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
                chrono::NaiveDateTime::parse_from_str(start_time, time::XLSX_DISPLAY_FMT)
                    .map(|dt| dt + Duration::minutes(session.duration as i64))
                    .map(|dt| dt.format(time::XLSX_DISPLAY_FMT).to_string())
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
            &start_time.format(time::XLSX_DISPLAY_FMT).to_string(),
        );
        set_str(
            ws,
            5,
            row,
            &end_time.format(time::XLSX_DISPLAY_FMT).to_string(),
        );
        set_str(ws, 6, row, "30");
        set_str(ws, 8, row, &prefix);

        // Set Lstart formula for timeline entries (dynamic column position)
        let lstart_formula = "=[@[Start Time]]";
        let cell = ws.get_cell_mut((lstart_col, row));
        cell.set_formula(&lstart_formula[1..]); // Remove = prefix
        cell.set_value(&start_time.format(time::XLSX_DISPLAY_FMT).to_string());

        // Set Lend formula for timeline entries (dynamic column position)
        let lend_formula = "=[@[End Time]]";
        let cell = ws.get_cell_mut((lend_col, row));
        cell.set_formula(&lend_formula[1..]); // Remove = prefix
        cell.set_value(&end_time.format(time::XLSX_DISPLAY_FMT).to_string());

        row += 1;
    }

    Ok(row - 1)
}

fn write_presenters_sheet(ws: &mut Worksheet, presenters: &[Presenter]) -> u32 {
    set_headers(
        ws,
        &[
            people::NAME.export,
            people::CLASSIFICATION.export,
            people::IS_GROUP.export,
            people::MEMBERS.export,
            people::GROUPS.export,
            people::ALWAYS_GROUPED.export,
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
    use crate::data::panel::{Panel, PanelPart, PanelSession};
    use crate::data::schedule::Meta;
    use crate::data::source_info::ImportedSheetPresence;

    fn make_test_schedule() -> Schedule {
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
                        panel_type: Some("GP".to_string()),
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
                        metadata: None,
                        parts: vec![PanelPart {
                            part_num: None,
                            description: None,
                            note: None,
                            prereq: None,
                            alt_panelist: None,
                            credited_presenters: vec![],
                            uncredited_presenters: vec!["Alice".to_string()],
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
                                credited_presenters: vec!["Alice".to_string()],
                                uncredited_presenters: vec![],
                                notes_non_printing: None,
                                workshop_notes: None,
                                power_needs: None,
                                sewing_machines: false,
                                av_notes: None,
                                source: None,
                                change_state: ChangeState::Unchanged,
                                conflicts: Vec::new(),
                                metadata: indexmap::IndexMap::new(),
                            }],
                            change_state: ChangeState::Unchanged,
                        }],
                        change_state: ChangeState::Unchanged,
                    },
                );
                panels
            },
            rooms: vec![crate::data::room::Room {
                uid: 1,
                short_name: "Main".to_string(),
                long_name: "Main Hall".to_string(),
                hotel_room: "Ballroom A".to_string(),
                sort_key: 1,
                is_break: false,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            }],
            panel_types: indexmap::IndexMap::new(),
            presenters: Vec::new(),
            imported_sheets: ImportedSheetPresence {
                has_room_map: true,
                has_panel_types: false,
                has_presenters: false,
                has_schedule: true,
            },
        }
    }

    #[test]
    fn test_flatten_export_excludes_deleted() {
        let mut schedule = make_test_schedule();
        // Mark the session as deleted
        if let Some(panel) = schedule.panels.get_mut("GP001") {
            for part in &mut panel.parts {
                for session in &mut part.sessions {
                    session.change_state = ChangeState::Deleted;
                }
            }
        }
        let sessions = flatten_panel_sessions(&schedule, false);
        assert!(
            sessions.is_empty(),
            "Deleted sessions should be excluded from export"
        );
    }

    #[test]
    fn test_flatten_export_includes_active() {
        let schedule = make_test_schedule();
        let sessions = flatten_panel_sessions(&schedule, false);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "GP001S1");
        assert!(sessions[0].credited_set.contains("Alice"));
    }
}
