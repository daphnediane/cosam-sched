/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use umya_spreadsheet::structs::{Table, TableColumn, TableStyleInfo, Worksheet};

use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::room::Room;
use super::schedule::Schedule;
use super::source_info::ChangeState;
use super::timeline::TimeType;

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
    for event in &schedule.events {
        if event.change_state == ChangeState::Deleted {
            continue;
        }
        for name in &event.presenters {
            *event_count.entry(name.as_str()).or_insert(0) += 1;
        }
    }

    let mut columns = Vec::new();

    for &(rank_str, prefix_char) in RANK_ORDER {
        let mut named_for_rank: Vec<(&str, &Presenter)> = Vec::new();
        let mut has_other = false;

        for (&name, &presenter) in &presenter_map {
            if presenter.rank != rank_str {
                continue;
            }
            if presenter.is_group {
                continue;
            }
            let count = event_count.get(name).copied().unwrap_or(0);
            let has_groups = !presenter.groups.is_empty();
            if has_groups || count >= MIN_PANELS_FOR_NAMED_COLUMN {
                named_for_rank.push((name, presenter));
            } else if count > 0 {
                has_other = true;
            }
        }

        for (&name, _) in &event_count {
            if presenter_map.contains_key(name) {
                continue;
            }
            if rank_str == "fan_panelist" {
                has_other = true;
            }
        }

        named_for_rank.sort_by_key(|(name, _)| *name);

        for (name, presenter) in named_for_rank {
            let header = if presenter.always_grouped {
                if let Some(group) = presenter.groups.first() {
                    format!("{}:{}=={}", prefix_char, name, group)
                } else {
                    format!("{}:{}", prefix_char, name)
                }
            } else if let Some(group) = presenter.groups.first() {
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
        "Name", "Rank", "Is Group", "Members", "Groups", "Always Grouped",
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
        "Prefix", "Panel Kind", "Color", "BW", "Is Break", "Is Workshop", "Is Café",
        "Is Room Hours", "Hidden",
    ];
    book.new_sheet("PanelTypes")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    {
        let ws = book
            .get_sheet_by_name_mut("PanelTypes")
            .ok_or_else(|| anyhow::anyhow!("Sheet 'PanelTypes' not found"))?;
        let last_row = write_panel_types_sheet(ws, &schedule.panel_types, &schedule.time_types);
        add_table(ws, "Prefix", prefix_headers, last_row);
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
    panel_types: &[PanelType],
    time_types: &[TimeType],
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
        ],
    );

    let mut row = 2u32;
    for pt in panel_types {
        if pt.change_state == ChangeState::Deleted {
            continue;
        }
        set_str(ws, 1, row, &pt.prefix);
        set_str(ws, 2, row, &pt.kind);
        set_opt(ws, 3, row, &pt.color);
        set_opt(ws, 4, row, &pt.bw_color);
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
        row += 1;
    }

    for tt in time_types {
        if tt.change_state == ChangeState::Deleted {
            continue;
        }
        set_str(ws, 1, row, &tt.prefix);
        set_str(ws, 2, row, &tt.kind);
        set_str(ws, 9, row, "Special");
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

    for event in &schedule.events {
        if event.change_state == ChangeState::Deleted {
            continue;
        }

        set_str(ws, 1, row, &event.id);
        set_str(ws, 2, row, &event.name);
        set_opt(ws, 3, row, &event.description);
        set_str(
            ws,
            4,
            row,
            &event.start_time.format("%-m/%-d/%Y %-I:%M %p").to_string(),
        );
        set_str(
            ws,
            5,
            row,
            &event.end_time.format("%-m/%-d/%Y %-I:%M %p").to_string(),
        );
        set_str(ws, 6, row, &event.duration.to_string());

        let room_name = event
            .room_id
            .and_then(|rid| schedule.room_by_id(rid))
            .map(|r| r.short_name.as_str())
            .unwrap_or("");
        set_str(ws, 7, row, room_name);

        let kind = event
            .panel_type
            .as_ref()
            .and_then(|pt_uid| {
                schedule
                    .panel_types
                    .iter()
                    .find(|pt| pt.effective_uid() == *pt_uid)
            })
            .map(|pt| pt.kind.as_str())
            .unwrap_or("");
        set_str(ws, 8, row, kind);

        set_opt(ws, 9, row, &event.cost);
        set_opt(ws, 10, row, &event.capacity);
        set_opt(ws, 11, row, &event.difficulty);
        set_opt(ws, 12, row, &event.note);
        set_opt(ws, 13, row, &event.prereq);
        set_opt(ws, 14, row, &event.ticket_url);
        if event.is_full {
            set_str(ws, 15, row, "Yes");
        }
        if event.hide_panelist {
            set_str(ws, 16, row, "Yes");
        }
        set_opt(ws, 17, row, &event.alt_panelist);

        let mut other_names: HashMap<&str, Vec<&str>> = HashMap::new();
        for presenter_name in &event.presenters {
            if let Some(&col) = named_presenters.get(presenter_name.as_str()) {
                set_str(ws, col, row, "Yes");
            } else {
                let rank = presenter_map
                    .get(presenter_name.as_str())
                    .map(|p| p.rank.as_str())
                    .unwrap_or("fan_panelist");
                other_names.entry(rank).or_default().push(presenter_name.as_str());
            }
        }

        for &(i, ref ocol) in &other_columns {
            if let Some(names) = other_names.get(ocol.rank.as_str()) {
                let col = fixed_count + i as u32 + 1;
                set_str(ws, col, row, &names.join(", "));
            }
        }

        row += 1;
    }

    for entry in &schedule.timeline {
        if entry.change_state == ChangeState::Deleted {
            continue;
        }
        let start_time: NaiveDateTime = entry.start_time.parse().with_context(|| {
            format!("Invalid timeline start time: {}", entry.start_time)
        })?;
        let end_time = start_time + chrono::Duration::minutes(30);

        let prefix = entry
            .time_type
            .as_ref()
            .and_then(|tt| tt.strip_prefix("time-type-"))
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
        set_str(ws, 2, row, &presenter.rank);
        if presenter.is_group {
            set_str(ws, 3, row, "Yes");
        }
        if !presenter.members.is_empty() {
            set_str(ws, 4, row, &presenter.members.join(", "));
        }
        if !presenter.groups.is_empty() {
            set_str(ws, 5, row, &presenter.groups.join(", "));
        }
        if presenter.always_grouped {
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
    use crate::data::schedule::Meta;
    use crate::data::source_info::ImportedSheetPresence;

    fn make_test_schedule() -> Schedule {
        let dt = |s: &str| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap()
        };

        Schedule {
            conflicts: Vec::new(),
            meta: Meta {
                title: "Test".to_string(),
                generated: "2026-01-01".to_string(),
                version: Some(4),
                generator: None,
                start_time: None,
                end_time: None,
            },
            timeline: Vec::new(),
            events: vec![Event {
                id: "GP001".to_string(),
                name: "Test Panel".to_string(),
                description: Some("A test panel".to_string()),
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
                presenters: vec!["Alice".to_string()],
                credits: Vec::new(),
                conflicts: Vec::new(),
                is_free: true,
                is_full: false,
                is_kids: false,
                hide_panelist: false,
                alt_panelist: None,
                source: None,
                change_state: ChangeState::Unchanged,
            }],
            rooms: vec![Room {
                uid: 1,
                short_name: "Main".to_string(),
                long_name: "Main Hall".to_string(),
                hotel_room: "Grand Ballroom".to_string(),
                sort_key: 1,
                source: None,
                change_state: ChangeState::Unchanged,
            }],
            panel_types: vec![PanelType {
                uid: Some("panel-type-gp".to_string()),
                prefix: "GP".to_string(),
                kind: "Guest Panel".to_string(),
                color: Some("#E2F9D7".to_string()),
                is_break: false,
                is_cafe: false,
                is_workshop: false,
                is_hidden: false,
                is_room_hours: false,
                bw_color: None,
                source: None,
                change_state: ChangeState::Unchanged,
            }],
            time_types: Vec::new(),
            presenters: vec![Presenter {
                name: "Alice".to_string(),
                rank: "guest".to_string(),
                is_group: false,
                members: Vec::new(),
                groups: Vec::new(),
                always_grouped: false,
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

        let book = umya_spreadsheet::reader::xlsx::read(&path)
            .expect("should read back exported XLSX");

        let room_ws = book.get_sheet_by_name("Rooms")
            .expect("Rooms sheet should exist");
        assert_eq!(room_ws.get_value((1, 1)), "Room Name");
        assert_eq!(room_ws.get_value((1, 2)), "Main");

        let prefix_ws = book.get_sheet_by_name("PanelTypes")
            .expect("PanelTypes sheet should exist");
        assert_eq!(prefix_ws.get_value((1, 1)), "Prefix");
        assert_eq!(prefix_ws.get_value((1, 2)), "GP");

        let sched_ws = book.get_sheet_by_name("Schedule")
            .expect("Schedule sheet should exist");
        assert_eq!(sched_ws.get_value((1, 1)), "Uniq ID");
        assert_eq!(sched_ws.get_value((1, 2)), "GP001");
        assert_eq!(sched_ws.get_value((2, 2)), "Test Panel");

        let fixed_col_count = SCHEDULE_FIXED_HEADERS.len() as u32;
        let other_col = fixed_col_count + 1;
        assert_eq!(sched_ws.get_value((other_col, 1)), "G:Other");
        assert_eq!(sched_ws.get_value((other_col, 2)), "Alice");

        let pres_ws = book.get_sheet_by_name("People")
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
            source: None,
            change_state: ChangeState::Deleted,
        });

        let dir = std::env::temp_dir();
        let path = dir.join("test_export_skips_deleted.xlsx");

        export_to_xlsx(&schedule, &path).expect("export should succeed");

        let book = umya_spreadsheet::reader::xlsx::read(&path)
            .expect("should read back exported XLSX");
        let room_ws = book.get_sheet_by_name("Rooms").unwrap();
        assert_eq!(room_ws.get_value((1, 2)), "Main");
        assert_eq!(room_ws.get_value((1, 3)), "", "Deleted room should not appear");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_presenter_columns() {
        let dt = |s: &str| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap()
        };
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
                name: "Pro".to_string(),
                rank: "guest".to_string(),
                is_group: false,
                members: Vec::new(),
                groups: vec!["Pros and Cons".to_string()],
                always_grouped: false,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                name: "Con".to_string(),
                rank: "guest".to_string(),
                is_group: false,
                members: Vec::new(),
                groups: vec!["Pros and Cons".to_string()],
                always_grouped: true,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                name: "Pros and Cons".to_string(),
                rank: "guest".to_string(),
                is_group: true,
                members: vec!["Pro".to_string(), "Con".to_string()],
                groups: Vec::new(),
                always_grouped: false,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                name: "Bob".to_string(),
                rank: "fan_panelist".to_string(),
                is_group: false,
                members: Vec::new(),
                groups: Vec::new(),
                always_grouped: false,
                source: None,
                change_state: ChangeState::Unchanged,
            },
        ];
        schedule.events = vec![
            make_event("GP001", vec!["Pro", "Con"]),
            make_event("GP002", vec!["Pro"]),
            make_event("GP003", vec!["Pro", "Bob"]),
        ];

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
            !headers.iter().any(|h| h.contains("Pros and Cons") && h.starts_with("G:Pros")),
            "Group entity 'Pros and Cons' should not get its own column"
        );

        let dir = std::env::temp_dir();
        let path = dir.join("test_export_presenter_columns.xlsx");
        export_to_xlsx(&schedule, &path).expect("export should succeed");

        let book = umya_spreadsheet::reader::xlsx::read(&path)
            .expect("should read back exported XLSX");
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
}
