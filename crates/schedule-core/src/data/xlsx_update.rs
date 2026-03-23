/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use umya_spreadsheet::structs::Worksheet;

use super::panel::Panel;
use super::panel_type::PanelType;
use super::presenter::{Presenter, PresenterGroup, PresenterMember, PresenterRank};
use super::room::Room;
use super::schedule::Schedule;
use super::source_info::{ChangeState, SourceInfo};
use super::xlsx_import::canonical_header;

/// Update an existing XLSX file in place, preserving formatting, formulas,
/// and extra columns. Only modifies rows that have changed.
pub fn update_xlsx(schedule: &Schedule, path: &Path) -> Result<()> {
    let mut book = umya_spreadsheet::reader::xlsx::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read XLSX {}: {e}", path.display()))?;

    {
        let properties = book.get_properties_mut();
        if let Some(ref modified) = schedule.meta.modified {
            properties.set_modified(modified);
        } else {
            let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            properties.set_modified(&now);
        }
        if let Some(ref modified_by) = schedule.meta.last_modified_by {
            properties.set_last_modified_by(modified_by);
        }
    }

    if schedule.imported_sheets.has_room_map {
        if let Some(sheet_name) = find_sheet_name(schedule.rooms.iter().map(|r| &r.source)) {
            update_rooms_sheet(&mut book, &sheet_name, schedule)?;
        }
    }

    if schedule.imported_sheets.has_panel_types {
        if let Some(sheet_name) =
            find_sheet_name(schedule.panel_types.values().map(|pt| &pt.source))
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
        .panels
        .retain(|_, p| p.change_state != ChangeState::Deleted);
    schedule
        .rooms
        .retain(|r| r.change_state != ChangeState::Deleted);
    schedule
        .panel_types
        .retain(|_, pt| pt.change_state != ChangeState::Deleted);
    schedule
        .presenters
        .retain(|p| p.change_state != ChangeState::Deleted);
    schedule
        .timeline
        .retain(|t| t.change_state != ChangeState::Deleted);

    for event in &mut schedule.events {
        event.change_state = ChangeState::Unchanged;
    }
    for panel in schedule.panels.values_mut() {
        panel.change_state = ChangeState::Unchanged;
        for part in &mut panel.parts {
            part.change_state = ChangeState::Unchanged;
            for session in &mut part.sessions {
                session.change_state = ChangeState::Unchanged;
            }
        }
    }
    for room in &mut schedule.rooms {
        room.change_state = ChangeState::Unchanged;
    }
    for panel_type in schedule.panel_types.values_mut() {
        panel_type.change_state = ChangeState::Unchanged;
    }
    for presenter in &mut schedule.presenters {
        presenter.change_state = ChangeState::Unchanged;
    }
    for entry in &mut schedule.timeline {
        entry.change_state = ChangeState::Unchanged;
    }
}

/// Represents a flattened session for XLSX update
struct UpdateSession {
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
    change_state: ChangeState,
    source: Option<SourceInfo>,
}

/// Flatten the panel hierarchy into updateable sessions with change tracking
fn flatten_panel_sessions_for_update(schedule: &Schedule) -> Vec<UpdateSession> {
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
                let mut presenters = Vec::new();
                presenters.extend(panel.credited_presenters.iter().cloned());
                presenters.extend(panel.uncredited_presenters.iter().cloned());
                presenters.extend(part.credited_presenters.iter().cloned());
                presenters.extend(part.uncredited_presenters.iter().cloned());

                // Use session-specific room if available, otherwise fall back to first room
                let room_id = session.room_ids.first().copied();

                // Determine the overall change state (highest priority)
                let change_state =
                    match (panel.change_state, part.change_state, session.change_state) {
                        (ChangeState::Deleted, _, _) | (_, ChangeState::Deleted, _) => {
                            ChangeState::Deleted
                        }
                        (ChangeState::Added, _, _)
                        | (_, ChangeState::Added, _)
                        | (_, _, ChangeState::Added) => ChangeState::Added,
                        (ChangeState::Modified, _, _)
                        | (_, ChangeState::Modified, _)
                        | (_, _, ChangeState::Modified) => ChangeState::Modified,
                        (ChangeState::Replaced, _, _)
                        | (_, ChangeState::Replaced, _)
                        | (_, _, ChangeState::Replaced) => ChangeState::Replaced,
                        _ => ChangeState::Unchanged,
                    };

                sessions.push(UpdateSession {
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
                    change_state,
                    source: session.source.clone(), // Use session source info
                });
            }
        }
    }

    sessions
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

fn update_table_areas(worksheet: &mut Worksheet, new_last_row: u32) {
    let last_row = new_last_row.max(2);
    for table in worksheet.get_tables_mut() {
        let (start, end) = table.get_area();
        let start_col = *start.get_col_num();
        let start_row = *start.get_row_num();
        let end_col = *end.get_col_num();
        table.set_area(((start_col, start_row), (end_col, last_row)));
    }
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
                    rows_to_delete.push(row_index);
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                if let Some(row_index) = room.source.as_ref().and_then(|s| s.row_index) {
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_room_to_row(worksheet, &header_map, row_index, room);
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

    let final_last_row = next_row - 1;
    if let Some(ws) = book.get_sheet_by_name_mut(sheet_name) {
        update_table_areas(ws, final_last_row);
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
    let color_opt = panel_type.color().map(|s| s.to_string());
    set_cell_opt_str(worksheet, header_map, row, &["Color"], &color_opt);
    let bw_opt = panel_type.bw_color().map(|s| s.to_string());
    set_cell_opt_str(worksheet, header_map, row, &["BW", "Bw"], &bw_opt);
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

    for (_prefix, panel_type) in &schedule.panel_types {
        match panel_type.change_state {
            ChangeState::Deleted => {
                if let Some(row_index) = panel_type.source.as_ref().and_then(|s| s.row_index) {
                    rows_to_delete.push(row_index);
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                if let Some(row_index) = panel_type.source.as_ref().and_then(|s| s.row_index) {
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_panel_type_to_row(worksheet, &header_map, row_index, panel_type);
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

    let final_last_row = next_row - 1;
    if let Some(ws) = book.get_sheet_by_name_mut(sheet_name) {
        update_table_areas(ws, final_last_row);
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
        .and_then(|pt_uid| schedule.panel_types.get(pt_uid))
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

fn write_session_to_row(
    worksheet: &mut Worksheet,
    header_map: &HashMap<String, u32>,
    row: u32,
    session: &UpdateSession,
    schedule: &Schedule,
) {
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Uniq_ID", "UniqID", "ID", "Id"],
        &session.id,
    );
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Name", "Panel_Name", "PanelName"],
        &session.name,
    );
    set_cell_opt_str(
        worksheet,
        header_map,
        row,
        &["Description"],
        &session.description,
    );

    if let Some(start_time) = &session.start_time {
        set_cell_str(
            worksheet,
            header_map,
            row,
            &["Start_Time", "StartTime", "Start"],
            start_time,
        );
    }

    if let Some(end_time) = &session.end_time {
        set_cell_str(
            worksheet,
            header_map,
            row,
            &["End_Time", "EndTime", "End", "Lend"],
            end_time,
        );
    }

    set_cell_u32(worksheet, header_map, row, &["Duration"], session.duration);

    let room_name = session
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

    let kind = session
        .panel_type
        .as_ref()
        .and_then(|pt_uid| schedule.panel_types.get(pt_uid))
        .map(|pt| pt.kind.as_str())
        .unwrap_or("");
    set_cell_str(
        worksheet,
        header_map,
        row,
        &["Kind", "Panel_Kind", "PanelKind"],
        kind,
    );

    set_cell_opt_str(worksheet, header_map, row, &["Cost"], &session.cost);
    set_cell_opt_str(worksheet, header_map, row, &["Capacity"], &session.capacity);
    set_cell_opt_str(
        worksheet,
        header_map,
        row,
        &["Difficulty"],
        &session.difficulty,
    );
    set_cell_opt_str(worksheet, header_map, row, &["Note"], &session.note);
    set_cell_opt_str(worksheet, header_map, row, &["Prereq"], &session.prereq);
    set_cell_opt_str(
        worksheet,
        header_map,
        row,
        &["Ticket_Sale", "TicketSale"],
        &session.ticket_url,
    );
    set_cell_bool(worksheet, header_map, row, &["Full"], session.is_full);
    set_cell_bool(
        worksheet,
        header_map,
        row,
        &["Hide_Panelist", "HidePanelist"],
        session.hide_panelist,
    );
    set_cell_opt_str(
        worksheet,
        header_map,
        row,
        &["Alt_Panelist", "AltPanelist"],
        &session.alt_panelist,
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
    let mut sessions_to_append: Vec<&UpdateSession> = Vec::new();

    let sessions = flatten_panel_sessions_for_update(schedule);

    for session in &sessions {
        match session.change_state {
            ChangeState::Deleted => {
                if let Some(row_index) = session.source.as_ref().and_then(|s| s.row_index) {
                    rows_to_delete.push(row_index);
                }
            }
            ChangeState::Modified | ChangeState::Replaced => {
                if let Some(row_index) = session.source.as_ref().and_then(|s| s.row_index) {
                    let worksheet = book
                        .get_sheet_by_name_mut(sheet_name)
                        .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
                    write_session_to_row(worksheet, &header_map, row_index, session, schedule);
                }
            }
            ChangeState::Added => {
                sessions_to_append.push(session);
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
    for session in sessions_to_append {
        let worksheet = book
            .get_sheet_by_name_mut(sheet_name)
            .ok_or_else(|| anyhow::anyhow!("Sheet '{sheet_name}' not found"))?;
        write_session_to_row(worksheet, &header_map, next_row, session, schedule);
        next_row += 1;
    }

    let final_last_row = next_row - 1;
    if let Some(ws) = book.get_sheet_by_name_mut(sheet_name) {
        update_table_areas(ws, final_last_row);
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
    use crate::data::timeline::TimelineEntry;
    use chrono::NaiveDateTime;

    fn make_schedule_with_change_states() -> Schedule {
        let dt = |s: &str| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap();

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
            panels: indexmap::IndexMap::new(),
            timeline: vec![
                TimelineEntry {
                    id: "TL01".to_string(),
                    start_time: "2026-06-26T09:00:00".to_string(),
                    description: "Opening".to_string(),
                    panel_type: None,
                    note: None,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Unchanged,
                },
                TimelineEntry {
                    id: "TL02".to_string(),
                    start_time: "2026-06-26T10:00:00".to_string(),
                    description: "Deleted entry".to_string(),
                    panel_type: None,
                    note: None,
                    metadata: None,
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
                    is_break: false,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Unchanged,
                },
                Room {
                    uid: 2,
                    short_name: "Old".to_string(),
                    long_name: "Old Room".to_string(),
                    hotel_room: "".to_string(),
                    sort_key: 2,
                    is_break: false,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Deleted,
                },
            ],
            panel_types: {
                let mut pt_map = indexmap::IndexMap::new();
                pt_map.insert(
                    "GP".to_string(),
                    PanelType {
                        prefix: "GP".to_string(),
                        kind: "General Panel".to_string(),
                        colors: indexmap::IndexMap::new(),
                        is_break: false,
                        is_cafe: false,
                        is_workshop: false,
                        is_hidden: false,
                        is_room_hours: false,
                        is_timeline: false,
                        is_private: false,
                        metadata: None,
                        source: None,
                        change_state: ChangeState::Modified,
                    },
                );
                pt_map
            },
            presenters: vec![
                Presenter {
                    id: None,
                    name: "Alice".to_string(),
                    rank: PresenterRank::from_str("guest"),
                    is_member: PresenterMember::NotMember,
                    is_grouped: PresenterGroup::NotGroup,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Converted,
                },
                Presenter {
                    id: None,
                    name: "Bob".to_string(),
                    rank: PresenterRank::from_str("staff"),
                    is_member: PresenterMember::NotMember,
                    is_grouped: PresenterGroup::NotGroup,
                    metadata: None,
                    source: None,
                    change_state: ChangeState::Converted,
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
        for panel_type in schedule.panel_types.values() {
            assert_eq!(panel_type.change_state, ChangeState::Unchanged);
        }
        for presenter in &schedule.presenters {
            assert_eq!(presenter.change_state, ChangeState::Unchanged);
        }
        for entry in &schedule.timeline {
            assert_eq!(entry.change_state, ChangeState::Unchanged);
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
}
