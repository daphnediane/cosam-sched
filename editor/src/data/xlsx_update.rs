use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use umya_spreadsheet::structs::Worksheet;

use super::panel_type::PanelType;
use super::room::Room;
use super::schedule::Schedule;
use super::source_info::{ChangeState, SourceInfo};
use super::xlsx_import::canonical_header;

/// Update an existing XLSX file in place, preserving formatting, formulas,
/// and extra columns. Only modifies rows that have changed.
pub fn update_xlsx(schedule: &Schedule, path: &Path) -> Result<()> {
    let mut book = umya_spreadsheet::reader::xlsx::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read XLSX {}: {e}", path.display()))?;

    if schedule.imported_sheets.has_room_map {
        if let Some(sheet_name) = find_sheet_name(schedule.rooms.iter().map(|r| &r.source)) {
            update_rooms_sheet(&mut book, &sheet_name, schedule)?;
        }
    }

    if schedule.imported_sheets.has_panel_types {
        if let Some(sheet_name) =
            find_sheet_name(schedule.panel_types.iter().map(|pt| &pt.source))
        {
            update_panel_types_sheet(&mut book, &sheet_name, schedule)?;
        }
    }

    if schedule.imported_sheets.has_schedule {
        if let Some(sheet_name) = find_sheet_name(schedule.events.iter().map(|e| &e.source)) {
            update_schedule_sheet(&mut book, &sheet_name, schedule)?;
        }
    }

    umya_spreadsheet::writer::xlsx::write(&book, path)
        .map_err(|e| anyhow::anyhow!("Failed to write XLSX {}: {e}", path.display()))?;

    Ok(())
}

/// After a successful save, remove Deleted items and reset all change states.
pub fn post_save_cleanup(schedule: &mut Schedule) {
    schedule
        .events
        .retain(|e| e.change_state != ChangeState::Deleted);
    schedule
        .rooms
        .retain(|r| r.change_state != ChangeState::Deleted);
    schedule
        .panel_types
        .retain(|pt| pt.change_state != ChangeState::Deleted);
    schedule
        .presenters
        .retain(|p| p.change_state != ChangeState::Deleted);
    schedule
        .timeline
        .retain(|t| t.change_state != ChangeState::Deleted);
    schedule
        .time_types
        .retain(|tt| tt.change_state != ChangeState::Deleted);

    for event in &mut schedule.events {
        event.change_state = ChangeState::Unchanged;
    }
    for room in &mut schedule.rooms {
        room.change_state = ChangeState::Unchanged;
    }
    for panel_type in &mut schedule.panel_types {
        panel_type.change_state = ChangeState::Unchanged;
    }
    for presenter in &mut schedule.presenters {
        presenter.change_state = ChangeState::Unchanged;
    }
    for entry in &mut schedule.timeline {
        entry.change_state = ChangeState::Unchanged;
    }
    for time_type in &mut schedule.time_types {
        time_type.change_state = ChangeState::Unchanged;
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

fn set_cell_str(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    keys: &[&str],
    value: &str,
) {
    for key in keys {
        if let Some(&col) = header_map.get(*key) {
            worksheet.get_cell_mut((col, row)).set_value(value);
            return;
        }
    }
}

fn set_cell_opt_str(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    keys: &[&str],
    value: &Option<String>,
) {
    let value = value.as_deref().unwrap_or("");
    set_cell_str(worksheet, header_map, row, keys, value);
}

fn set_cell_u32(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    keys: &[&str],
    value: u32,
) {
    set_cell_str(worksheet, header_map, row, keys, &value.to_string());
}

fn set_cell_bool(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    keys: &[&str],
    value: bool,
) {
    let text = if value { "Yes" } else { "" };
    set_cell_str(worksheet, header_map, row, keys, text);
}

fn calamine_row_to_umya(row_index: u32) -> u32 {
    row_index + 1
}

// ── Rooms ──────────────────────────────────────────────────────────────────

fn write_room_to_row(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    room: &Room,
) {
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Room_Name", "Room", "Name"],
        &room.short_name,
    );
    set_cell_str(worksheet, header_map, row, &["Long_Name"], &room.long_name);
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Hotel_Room", "HotelRoom"],
        &room.hotel_room,
    );
    set_cell_u32(
        worksheet,
        header_map,
        row,
        &["Sort_Key", "SortKey"],
        room.sort_key,
    );
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
    let highest_row = worksheet.get_highest_row();

    let mut rows_to_delete: Vec<u32> = Vec::new();
    let mut rows_to_append: Vec<&Room> = Vec::new();

    for room in &schedule.rooms {
        match room.change_state {
            ChangeState::Deleted => {
                if let Some(row_index) = room.source.as_ref().and_then(|s| s.row_index) {
                    rows_to_delete.push(calamine_row_to_umya(row_index));
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                if let Some(row_index) = room.source.as_ref().and_then(|s| s.row_index) {
                    let umya_row = calamine_row_to_umya(row_index);
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_room_to_row(worksheet, &header_map, umya_row, room);
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
        write_room_to_row(worksheet, &header_map, next_row, room);
        next_row += 1;
    }

    Ok(())
}

// ── Panel Types ────────────────────────────────────────────────────────────

fn write_panel_type_to_row(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    panel_type: &PanelType,
) {
    set_cell_str(worksheet, header_map, row, &["Prefix"], &panel_type.prefix);
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Panel_Kind", "PanelKind", "Kind"],
        &panel_type.kind,
    );
    set_cell_opt_str(worksheet, header_map, row, &["Color"], &panel_type.color);
    set_cell_opt_str(worksheet, header_map, row, &["BW", "Bw"], &panel_type.bw_color);
    set_cell_bool(
        worksheet,
        header_map,
        row,
        &["Is_Break"],
        panel_type.is_break,
    );
    set_cell_bool(
        worksheet,
        header_map,
        row,
        &["Is_Cafe", "Is_Café"],
        panel_type.is_cafe,
    );
    set_cell_bool(
        worksheet,
        header_map,
        row,
        &["Is_Workshop"],
        panel_type.is_workshop,
    );
    set_cell_bool(
        worksheet,
        header_map,
        row,
        &["Is_Room_Hours", "IsRoomHours"],
        panel_type.is_room_hours,
    );
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Hidden"],
        if panel_type.is_hidden { "Yes" } else { "" },
    );
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
    let highest_row = worksheet.get_highest_row();

    let mut rows_to_delete: Vec<u32> = Vec::new();
    let mut rows_to_append: Vec<&PanelType> = Vec::new();

    for panel_type in &schedule.panel_types {
        match panel_type.change_state {
            ChangeState::Deleted => {
                if let Some(row_index) = panel_type.source.as_ref().and_then(|s| s.row_index) {
                    rows_to_delete.push(calamine_row_to_umya(row_index));
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                if let Some(row_index) = panel_type.source.as_ref().and_then(|s| s.row_index) {
                    let umya_row = calamine_row_to_umya(row_index);
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_panel_type_to_row(worksheet, &header_map, umya_row, panel_type);
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
        write_panel_type_to_row(worksheet, &header_map, next_row, panel_type);
        next_row += 1;
    }

    Ok(())
}

// ── Schedule (Events) ──────────────────────────────────────────────────────

fn write_event_to_row(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    event: &super::event::Event,
    schedule: &Schedule,
) {
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Uniq_ID", "UniqID", "ID", "Id"],
        &event.id,
    );
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Name", "Panel_Name", "PanelName"],
        &event.name,
    );
    set_cell_opt_str(
        worksheet,
        header_map,
        row,
        &["Description"],
        &event.description,
    );

    let start_str = event.start_time.format("%-m/%-d/%Y %-I:%M %p").to_string();
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Start_Time", "StartTime", "Start"],
        &start_str,
    );

    let end_str = event.end_time.format("%-m/%-d/%Y %-I:%M %p").to_string();
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["End_Time", "EndTime", "End", "Lend"],
        &end_str,
    );

    set_cell_u32(worksheet, header_map, row, &["Duration"], event.duration);

    let room_name = event
        .room_id
        .and_then(|rid| schedule.room_by_id(rid))
        .map(|r| r.short_name.as_str())
        .unwrap_or("");
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Room", "Room_Name", "RoomName"],
        room_name,
    );

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
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Kind", "Panel_Kind", "PanelKind"],
        kind,
    );

    set_cell_opt_str(worksheet, header_map, row, &["Cost"], &event.cost);
    set_cell_opt_str(worksheet, header_map, row, &["Capacity"], &event.capacity);
    set_cell_opt_str(
        worksheet,
        header_map,
        row,
        &["Difficulty"],
        &event.difficulty,
    );
    set_cell_opt_str(worksheet, header_map, row, &["Note"], &event.note);
    set_cell_opt_str(worksheet, header_map, row, &["Prereq"], &event.prereq);
    set_cell_opt_str(
        worksheet,
        header_map,
        row,
        &["Ticket_Sale", "TicketSale"],
        &event.ticket_url,
    );
    set_cell_bool(worksheet, header_map, row, &["Full"], event.is_full);
    set_cell_bool(
        worksheet,
        header_map,
        row,
        &["Hide_Panelist", "HidePanelist"],
        event.hide_panelist,
    );
    set_cell_opt_str(
        worksheet,
        header_map,
        row,
        &["Alt_Panelist", "AltPanelist"],
        &event.alt_panelist,
    );
}

fn update_schedule_sheet(
    book: &mut umya_spreadsheet::Spreadsheet,
    sheet_name: &str,
    schedule: &Schedule,
) -> Result<()> {
    let worksheet = book
        .get_sheet_by_name(sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
    let header_map = build_header_map(worksheet);
    let highest_row = worksheet.get_highest_row();

    let mut rows_to_delete: Vec<u32> = Vec::new();
    let mut events_to_append: Vec<&super::event::Event> = Vec::new();

    for event in &schedule.events {
        match event.change_state {
            ChangeState::Deleted => {
                if let Some(row_index) = event.source.as_ref().and_then(|s| s.row_index) {
                    rows_to_delete.push(calamine_row_to_umya(row_index));
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                if let Some(row_index) = event.source.as_ref().and_then(|s| s.row_index) {
                    let umya_row = calamine_row_to_umya(row_index);
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_event_to_row(worksheet, &header_map, umya_row, event, schedule);
                }
            }
            ChangeState::Added => {
                events_to_append.push(event);
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
    for event in events_to_append {
        let worksheet = book
            .get_sheet_by_name_mut(sheet_name)
            .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
        write_event_to_row(worksheet, &header_map, next_row, event, schedule);
        next_row += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::event::Event;
    use crate::data::presenter::Presenter;
    use crate::data::schedule::{Meta, Schedule};
    use crate::data::source_info::ImportedSheetPresence;
    use crate::data::timeline::{TimeType, TimelineEntry};
    use chrono::NaiveDateTime;

    fn make_schedule_with_change_states() -> Schedule {
        let dt = |s: &str| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap();

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
            timeline: vec![
                TimelineEntry {
                    id: "TL01".to_string(),
                    start_time: "2026-06-26T09:00:00".to_string(),
                    description: "Opening".to_string(),
                    time_type: None,
                    note: None,
                    source: None,
                    change_state: ChangeState::Unchanged,
                },
                TimelineEntry {
                    id: "TL02".to_string(),
                    start_time: "2026-06-26T10:00:00".to_string(),
                    description: "Deleted entry".to_string(),
                    time_type: None,
                    note: None,
                    source: None,
                    change_state: ChangeState::Deleted,
                },
            ],
            events: vec![
                Event {
                    id: "GP001".to_string(),
                    name: "Unchanged Event".to_string(),
                    description: None,
                    start_time: dt("2026-06-26T09:00:00"),
                    end_time: dt("2026-06-26T10:00:00"),
                    duration: 60,
                    room_id: Some(1),
                    panel_type: None,
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
                    source: Some(SourceInfo {
                        file_path: Some("test.xlsx".to_string()),
                        sheet_name: Some("Schedule".to_string()),
                        row_index: Some(1),
                    }),
                    change_state: ChangeState::Unchanged,
                },
                Event {
                    id: "GP002".to_string(),
                    name: "Modified Event".to_string(),
                    description: None,
                    start_time: dt("2026-06-26T11:00:00"),
                    end_time: dt("2026-06-26T12:00:00"),
                    duration: 60,
                    room_id: None,
                    panel_type: None,
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
                    source: Some(SourceInfo {
                        file_path: Some("test.xlsx".to_string()),
                        sheet_name: Some("Schedule".to_string()),
                        row_index: Some(2),
                    }),
                    change_state: ChangeState::Modified,
                },
                Event {
                    id: "GP003".to_string(),
                    name: "Deleted Event".to_string(),
                    description: None,
                    start_time: dt("2026-06-26T13:00:00"),
                    end_time: dt("2026-06-26T14:00:00"),
                    duration: 60,
                    room_id: None,
                    panel_type: None,
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
                    source: Some(SourceInfo {
                        file_path: Some("test.xlsx".to_string()),
                        sheet_name: Some("Schedule".to_string()),
                        row_index: Some(3),
                    }),
                    change_state: ChangeState::Deleted,
                },
                Event {
                    id: "GP004".to_string(),
                    name: "Added Event".to_string(),
                    description: None,
                    start_time: dt("2026-06-26T15:00:00"),
                    end_time: dt("2026-06-26T16:00:00"),
                    duration: 60,
                    room_id: None,
                    panel_type: None,
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
                    change_state: ChangeState::Added,
                },
            ],
            rooms: vec![
                Room {
                    uid: 1,
                    short_name: "Main".to_string(),
                    long_name: "Main Hall".to_string(),
                    hotel_room: "Ballroom A".to_string(),
                    sort_key: 1,
                    source: None,
                    change_state: ChangeState::Unchanged,
                },
                Room {
                    uid: 2,
                    short_name: "Old".to_string(),
                    long_name: "Old Room".to_string(),
                    hotel_room: "".to_string(),
                    sort_key: 2,
                    source: None,
                    change_state: ChangeState::Deleted,
                },
            ],
            panel_types: vec![PanelType {
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
                change_state: ChangeState::Modified,
            }],
            time_types: vec![TimeType {
                uid: "time-type-split".to_string(),
                prefix: "SPLIT".to_string(),
                kind: "Split".to_string(),
                source: None,
                change_state: ChangeState::Converted,
            }],
            presenters: vec![
                Presenter {
                    name: "Alice".to_string(),
                    rank: "guest".to_string(),
                    is_group: false,
                    members: Vec::new(),
                    groups: Vec::new(),
                    always_grouped: false,
                    source: None,
                    change_state: ChangeState::Converted,
                },
                Presenter {
                    name: "Bob".to_string(),
                    rank: "staff".to_string(),
                    is_group: false,
                    members: Vec::new(),
                    groups: Vec::new(),
                    always_grouped: false,
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
        let mut schedule = make_schedule_with_change_states();

        assert_eq!(schedule.events.len(), 4);
        assert_eq!(schedule.rooms.len(), 2);
        assert_eq!(schedule.presenters.len(), 2);
        assert_eq!(schedule.timeline.len(), 2);

        post_save_cleanup(&mut schedule);

        assert_eq!(schedule.events.len(), 3, "Deleted event should be removed");
        assert_eq!(schedule.rooms.len(), 1, "Deleted room should be removed");
        assert_eq!(
            schedule.presenters.len(),
            1,
            "Deleted presenter should be removed"
        );
        assert_eq!(
            schedule.timeline.len(),
            1,
            "Deleted timeline entry should be removed"
        );

        assert!(!schedule.events.iter().any(|e| e.id == "GP003"));
        assert!(!schedule.rooms.iter().any(|r| r.short_name == "Old"));
        assert!(!schedule.presenters.iter().any(|p| p.name == "Bob"));
    }

    #[test]
    fn test_post_save_cleanup_resets_change_states() {
        let mut schedule = make_schedule_with_change_states();

        post_save_cleanup(&mut schedule);

        for event in &schedule.events {
            assert_eq!(
                event.change_state,
                ChangeState::Unchanged,
                "Event '{}' should be Unchanged after cleanup",
                event.id
            );
        }
        for room in &schedule.rooms {
            assert_eq!(room.change_state, ChangeState::Unchanged);
        }
        for panel_type in &schedule.panel_types {
            assert_eq!(panel_type.change_state, ChangeState::Unchanged);
        }
        for presenter in &schedule.presenters {
            assert_eq!(presenter.change_state, ChangeState::Unchanged);
        }
        for entry in &schedule.timeline {
            assert_eq!(entry.change_state, ChangeState::Unchanged);
        }
        for time_type in &schedule.time_types {
            assert_eq!(time_type.change_state, ChangeState::Unchanged);
        }
    }

    #[test]
    fn test_post_save_cleanup_preserves_data() {
        let mut schedule = make_schedule_with_change_states();
        post_save_cleanup(&mut schedule);

        assert!(schedule.events.iter().any(|e| e.id == "GP001"));
        assert!(schedule.events.iter().any(|e| e.id == "GP002"));
        assert!(schedule.events.iter().any(|e| e.id == "GP004"));
        assert!(schedule.rooms.iter().any(|r| r.short_name == "Main"));
        assert!(schedule.presenters.iter().any(|p| p.name == "Alice"));
        assert_eq!(schedule.time_types.len(), 1);
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
    fn test_calamine_row_to_umya() {
        assert_eq!(calamine_row_to_umya(0), 1);
        assert_eq!(calamine_row_to_umya(1), 2);
        assert_eq!(calamine_row_to_umya(10), 11);
    }
}
