use std::path::Path;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;

#[cfg(feature = "xlsx_export")]
use rust_xlsxwriter::{Format, Workbook, Worksheet, XlsxError};

use super::event::Event;
use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::room::Room;
use super::schedule::Schedule;
use super::timeline::{TimeType, TimelineEntry};

#[cfg(feature = "xlsx_export")]
pub fn export_to_xlsx(schedule: &Schedule, path: &Path) -> Result<()> {
    let mut workbook = Workbook::new();

    // Export Rooms sheet
    let worksheet = export_rooms_worksheet(&schedule.rooms)?;
    workbook.push_worksheet(worksheet);

    // Export Panel Types sheet (including time types as special)
    let worksheet = export_panel_types_worksheet(&schedule.panel_types, &schedule.time_types)?;
    workbook.push_worksheet(worksheet);

    // Export Schedule sheet (including timeline as events)
    let worksheet = export_schedule_worksheet(&schedule.events, &schedule.timeline)?;
    workbook.push_worksheet(worksheet);

    // Export Presenters sheet
    let worksheet = export_presenters_worksheet(&schedule.presenters)?;
    workbook.push_worksheet(worksheet);

    workbook
        .save(path)
        .map_err(|e| anyhow::anyhow!("Failed to save XLSX: {}", e))?;

    Ok(())
}

#[cfg(not(feature = "xlsx_export"))]
pub fn export_to_xlsx(_schedule: &Schedule, _path: &Path) -> Result<()> {
    anyhow::bail!("XLSX export feature not enabled. Enable with --features xlsx_export")
}

#[cfg(feature = "xlsx_export")]
fn export_rooms_worksheet(rooms: &[Room]) -> Result<Worksheet> {
    let mut worksheet = Worksheet::new();

    // Headers
    let headers = ["UID", "Short Name", "Long Name", "Hotel Room", "Sort Key"];
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string(0, col as u16, header)?;
    }

    // Data rows
    for (row, room) in rooms.iter().enumerate() {
        let row_idx = row as u32 + 1;
        worksheet.write_number(row_idx, 0, room.uid as f64)?;
        worksheet.write_string(row_idx, 1, &room.short_name)?;
        worksheet.write_string(row_idx, 2, &room.long_name)?;
        worksheet.write_string(row_idx, 3, &room.hotel_room)?;
        worksheet.write_number(row_idx, 4, room.sort_key as f64)?;
    }

    Ok(worksheet)
}

#[cfg(feature = "xlsx_export")]
fn export_panel_types_worksheet(
    panel_types: &[PanelType],
    time_types: &[TimeType],
) -> Result<Worksheet> {
    let mut worksheet = Worksheet::new();

    // Headers
    let headers = [
        "Prefix",
        "Panel Kind",
        "Color",
        "Is Break",
        "Is Workshop",
        "Is Café",
        "Hidden",
        "Is Split",
    ];
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string(0, col as u16, header)?;
    }

    // Panel type rows
    for (row, pt) in panel_types.iter().enumerate() {
        let row_idx = row as u32 + 1;
        worksheet.write_string(row_idx, 0, &pt.prefix)?;
        worksheet.write_string(row_idx, 1, &pt.kind)?;
        worksheet.write_string(row_idx, 2, &pt.color.clone().unwrap_or_default())?;
        worksheet.write_string(row_idx, 3, if pt.is_break { "1" } else { "" })?;
        worksheet.write_string(row_idx, 4, if pt.is_workshop { "1" } else { "" })?;
        worksheet.write_string(row_idx, 5, if pt.is_cafe { "1" } else { "" })?;
        worksheet.write_string(row_idx, 6, if pt.is_hidden { "1" } else { "" })?;
        worksheet.write_string(row_idx, 7, "")?; // Is Split = false for regular panel types
    }

    // Time type rows (marked as Special in Hidden column, Is Split = true)
    for (row_offset, tt) in time_types.iter().enumerate() {
        let row_idx = (panel_types.len() + row_offset + 1) as u32;
        worksheet.write_string(row_idx, 0, &tt.prefix)?;
        worksheet.write_string(row_idx, 1, &tt.kind)?;
        worksheet.write_string(row_idx, 2, "")?;
        worksheet.write_string(row_idx, 3, "")?;
        worksheet.write_string(row_idx, 4, "")?;
        worksheet.write_string(row_idx, 5, "")?;
        worksheet.write_string(row_idx, 6, "Special")?; // Mark as Special
        worksheet.write_string(row_idx, 7, "1")?; // Is Split = true for time types
    }

    Ok(worksheet)
}

#[cfg(feature = "xlsx_export")]
fn export_schedule_worksheet(events: &[Event], timeline: &[TimelineEntry]) -> Result<Worksheet> {
    let mut worksheet = Worksheet::new();

    // Headers
    let headers = [
        "ID",
        "Name",
        "Description",
        "Start Time",
        "End Time",
        "Duration",
        "Room",
        "Prefix",
        "Cost",
        "Capacity",
        "Difficulty",
        "Note",
        "Prereq",
        "Ticket URL",
        "Presenters",
    ];
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string(0, col as u16, header)?;
    }

    // Regular event rows
    for (row, event) in events.iter().enumerate() {
        let row_idx = row as u32 + 1;
        let room_id = event.room_id.map(|id| id.to_string()).unwrap_or_default();

        let prefix = event
            .panel_type
            .as_ref()
            .and_then(|pt| pt.strip_prefix("panel-type-"))
            .unwrap_or("")
            .to_uppercase();

        worksheet.write_string(row_idx, 0, &event.id)?;
        worksheet.write_string(row_idx, 1, &event.name)?;
        worksheet.write_string(row_idx, 2, &event.description.clone().unwrap_or_default())?;
        worksheet.write_string(
            row_idx,
            3,
            &event.start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
        )?;
        worksheet.write_string(
            row_idx,
            4,
            &event.end_time.format("%Y-%m-%d %H:%M:%S").to_string(),
        )?;
        worksheet.write_number(row_idx, 5, event.duration as f64)?;
        worksheet.write_string(row_idx, 6, &room_id)?;
        worksheet.write_string(row_idx, 7, &prefix)?;
        worksheet.write_string(row_idx, 8, &event.cost.clone().unwrap_or_default())?;
        worksheet.write_string(row_idx, 9, &event.capacity.clone().unwrap_or_default())?;
        worksheet.write_string(row_idx, 10, &event.difficulty.clone().unwrap_or_default())?;
        worksheet.write_string(row_idx, 11, &event.note.clone().unwrap_or_default())?;
        worksheet.write_string(row_idx, 12, &event.prereq.clone().unwrap_or_default())?;
        worksheet.write_string(row_idx, 13, &event.ticket_url.clone().unwrap_or_default())?;
        worksheet.write_string(row_idx, 14, &event.presenters.join(", "))?;
    }

    // Timeline rows (converted to events with 30-minute duration)
    for (row_offset, timeline_entry) in timeline.iter().enumerate() {
        let start_time: NaiveDateTime = timeline_entry.start_time.parse().with_context(|| {
            format!("Invalid timeline start time: {}", timeline_entry.start_time)
        })?;
        let end_time = start_time + chrono::Duration::minutes(30);

        let prefix = timeline_entry
            .time_type
            .as_ref()
            .and_then(|tt| tt.strip_prefix("time-type-"))
            .unwrap_or("SPLIT")
            .to_uppercase();

        let row_idx = (events.len() + row_offset + 1) as u32;
        worksheet.write_string(row_idx, 0, &timeline_entry.id)?;
        worksheet.write_string(row_idx, 1, &timeline_entry.description)?;
        worksheet.write_string(row_idx, 2, &timeline_entry.note.clone().unwrap_or_default())?;
        worksheet.write_string(
            row_idx,
            3,
            &start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
        )?;
        worksheet.write_string(
            row_idx,
            4,
            &end_time.format("%Y-%m-%d %H:%M:%S").to_string(),
        )?;
        worksheet.write_number(row_idx, 5, 30.0)?; // 30 minutes duration
        worksheet.write_string(row_idx, 6, "")?; // No room
        worksheet.write_string(row_idx, 7, &prefix)?;
        worksheet.write_string(row_idx, 8, "")?; // No cost
        worksheet.write_string(row_idx, 9, "")?; // No capacity
        worksheet.write_string(row_idx, 10, "")?; // No difficulty
        worksheet.write_string(row_idx, 11, "")?; // No note
        worksheet.write_string(row_idx, 12, "")?; // No prereq
        worksheet.write_string(row_idx, 13, "")?; // No ticket URL
        worksheet.write_string(row_idx, 14, "")?; // No presenters
    }

    Ok(worksheet)
}

#[cfg(feature = "xlsx_export")]
fn export_presenters_worksheet(presenters: &[Presenter]) -> Result<Worksheet> {
    let mut worksheet = Worksheet::new();

    // Headers
    let headers = [
        "Name",
        "Rank",
        "Is Group",
        "Members",
        "Groups",
        "Always Grouped",
    ];
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string(0, col as u16, header)?;
    }

    // Data rows
    for (row, presenter) in presenters.iter().enumerate() {
        let row_idx = row as u32 + 1;
        worksheet.write_string(row_idx, 0, &presenter.name)?;
        worksheet.write_string(row_idx, 1, &presenter.rank)?;
        worksheet.write_string(row_idx, 2, if presenter.is_group { "1" } else { "" })?;
        worksheet.write_string(row_idx, 3, &presenter.members.join(", "))?;
        worksheet.write_string(row_idx, 4, &presenter.groups.join(", "))?;
        worksheet.write_string(row_idx, 5, if presenter.always_grouped { "1" } else { "" })?;
    }

    Ok(worksheet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::panel_type::PanelType;
    use crate::data::room::Room;
    use crate::data::timeline::TimeType;

    #[test]
    fn test_export_rooms_worksheet() {
        let rooms = vec![Room {
            uid: 1,
            short_name: "Main".to_string(),
            long_name: "Main Hall".to_string(),
            hotel_room: "Grand Ballroom".to_string(),
            sort_key: 1,
        }];

        #[cfg(feature = "xlsx_export")]
        {
            let worksheet = export_rooms_worksheet(&rooms).unwrap();
            // Test would need to verify worksheet content
        }
    }

    #[test]
    fn test_export_panel_types_worksheet() {
        let panel_types = vec![PanelType {
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
        }];

        let time_types = vec![TimeType {
            uid: "time-type-split".to_string(),
            prefix: "SPLIT".to_string(),
            kind: "Page split".to_string(),
        }];

        #[cfg(feature = "xlsx_export")]
        {
            let worksheet = export_panel_types_worksheet(&panel_types, &time_types).unwrap();
            // Test would need to verify worksheet content
        }
    }
}
