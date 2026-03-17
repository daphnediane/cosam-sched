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

pub fn export_to_xlsx(schedule: &Schedule, path: &Path) -> Result<()> {
    let mut book = umya_spreadsheet::new_file();

    let room_headers = &["Room Name", "Long Name", "Hotel Room", "Sort Key"];
    {
        let ws = book
            .get_sheet_mut(&0)
            .ok_or_else(|| anyhow::anyhow!("No default sheet"))?;
        ws.set_name("RoomMap");
        let last_row = write_rooms_sheet(ws, &schedule.rooms);
        add_table(ws, "RoomMapTable", room_headers, last_row);
    }

    let prefix_headers = &[
        "Prefix", "Panel Kind", "Color", "BW", "Is Break", "Is Workshop", "Is Café",
        "Is Room Hours", "Hidden",
    ];
    book.new_sheet("Prefix")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    {
        let ws = book
            .get_sheet_by_name_mut("Prefix")
            .ok_or_else(|| anyhow::anyhow!("Sheet 'Prefix' not found"))?;
        let last_row = write_panel_types_sheet(ws, &schedule.panel_types, &schedule.time_types);
        add_table(ws, "PrefixTable", prefix_headers, last_row);
    }

    let schedule_headers = &[
        "Uniq ID", "Name", "Description", "Start Time", "End Time", "Duration", "Room",
        "Kind", "Cost", "Capacity", "Difficulty", "Note", "Prereq", "Ticket Sale", "Full",
        "Hide Panelist", "Alt Panelist", "Presenters",
    ];
    book.new_sheet("Schedule")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    {
        let ws = book
            .get_sheet_by_name_mut("Schedule")
            .ok_or_else(|| anyhow::anyhow!("Sheet 'Schedule' not found"))?;
        let last_row = write_schedule_sheet(ws, schedule)?;
        add_table(ws, "ScheduleTable", schedule_headers, last_row);
    }

    let presenter_headers = &[
        "Name", "Rank", "Is Group", "Members", "Groups", "Always Grouped",
    ];
    book.new_sheet("Presenters")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    {
        let ws = book
            .get_sheet_by_name_mut("Presenters")
            .ok_or_else(|| anyhow::anyhow!("Sheet 'Presenters' not found"))?;
        let last_row = write_presenters_sheet(ws, &schedule.presenters);
        add_table(ws, "PresentersTable", presenter_headers, last_row);
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

fn write_schedule_sheet(ws: &mut Worksheet, schedule: &Schedule) -> Result<u32> {
    set_headers(
        ws,
        &[
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
            "Presenters",
        ],
    );

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

        if !event.presenters.is_empty() {
            set_str(ws, 18, row, &event.presenters.join(", "));
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

        let room_ws = book.get_sheet_by_name("RoomMap")
            .expect("RoomMap sheet should exist");
        assert_eq!(room_ws.get_value((1, 1)), "Room Name");
        assert_eq!(room_ws.get_value((1, 2)), "Main");

        let prefix_ws = book.get_sheet_by_name("Prefix")
            .expect("Prefix sheet should exist");
        assert_eq!(prefix_ws.get_value((1, 1)), "Prefix");
        assert_eq!(prefix_ws.get_value((1, 2)), "GP");

        let sched_ws = book.get_sheet_by_name("Schedule")
            .expect("Schedule sheet should exist");
        assert_eq!(sched_ws.get_value((1, 1)), "Uniq ID");
        assert_eq!(sched_ws.get_value((1, 2)), "GP001");
        assert_eq!(sched_ws.get_value((2, 2)), "Test Panel");

        let pres_ws = book.get_sheet_by_name("Presenters")
            .expect("Presenters sheet should exist");
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
        let room_ws = book.get_sheet_by_name("RoomMap").unwrap();
        assert_eq!(room_ws.get_value((1, 2)), "Main");
        assert_eq!(room_ws.get_value((1, 3)), "", "Deleted room should not appear");

        std::fs::remove_file(&path).ok();
    }
}
