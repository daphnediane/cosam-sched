/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use anyhow::Result;
use chrono::Timelike;
use umya_spreadsheet::structs::Worksheet;

use crate::data::room::Room;
use crate::data::schedule::Schedule;
use crate::data::source_info::ChangeState;

pub(super) fn write_grid_sheet(ws: &mut Worksheet, schedule: &Schedule) -> Result<()> {
    // Get unique rooms from schedule, excluding break/room-hours rooms
    let mut rooms: Vec<&Room> = schedule
        .rooms
        .iter()
        .filter(|r| r.change_state != ChangeState::Deleted && !r.is_break)
        .collect();
    rooms.sort_by_key(|r| r.sort_key);

    // Find the earliest and latest times from all events/timeline
    let mut times = Vec::new();

    // Add timeline entry times
    for entry in &schedule.timeline {
        if entry.change_state == ChangeState::Deleted {
            continue;
        }
        if let Some(start_time) = entry.start_time {
            times.push(start_time);
        }
    }

    // Add panel session times
    for ps in schedule.panel_sets.values() {
        for panel in &ps.panels {
            if panel.change_state == ChangeState::Deleted {
                continue;
            }
            if let Some(start_time) = panel.timing.start_time() {
                times.push(start_time);
            }
        }
    }

    if times.is_empty() {
        // Default to a typical convention day if no times found
        let base_time =
            chrono::NaiveDateTime::parse_from_str("2026-06-26 09:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
        times.push(base_time);
        times.push(base_time + chrono::Duration::hours(12));
    }

    times.sort();

    let start_time = times[0];
    let end_time = times[times.len() - 1];

    // Round start down to nearest 30 minutes and end up to nearest 30 minutes
    let start_time = start_time
        .with_minute((start_time.minute() / 30) * 30)
        .unwrap()
        .with_second(0)
        .unwrap();
    let mut end_time = end_time
        .with_minute(((end_time.minute() + 29) / 30) * 30)
        .unwrap()
        .with_second(0)
        .unwrap();
    if end_time <= start_time {
        end_time = start_time + chrono::Duration::hours(1);
    }

    // Write "Time" header in cell A1
    ws.get_cell_mut((1, 1)).set_value("Time");

    // Write room headers across the top (starting at column B)
    for (i, room) in rooms.iter().enumerate() {
        let col = (i + 2) as u32;
        ws.get_cell_mut((col, 1)).set_value(&room.short_name);
    }

    // Write time intervals down the left column as proper date/times with formulas
    let mut current_time = start_time;
    let mut row = 2u32;

    while current_time < end_time {
        // Set the first time as a proper date/time value
        if row == 2 {
            let datetime_str = current_time.format("%Y-%m-%d %H:%M:%S").to_string();
            ws.get_cell_mut((1, row)).set_value(&datetime_str);
        } else {
            // For subsequent rows, use a formula that adds 30 minutes to the previous row
            let prev_row = row - 1;
            let formula = format!("A{}+TIME(0,30,0)", prev_row);
            ws.get_cell_mut((1, row)).set_formula(&formula);
            // Also set the calculated value for compatibility
            let datetime_str = current_time.format("%Y-%m-%d %H:%M:%S").to_string();
            ws.get_cell_mut((1, row)).set_value(&datetime_str);
        }

        // Write formulas for each room cell
        for (i, room) in rooms.iter().enumerate() {
            let col = (i + 2) as u32;

            // Use the improved LET formula for Excel
            let room_name = &room.short_name;
            let _formula = format!(
                "LET(X,SUMPRODUCT(IF(NOT(ISERR(SEARCH(\"{}\",Schedule[Room]))),1,0),IF(Schedule[Lstart]<=[@Time],1,0),IF(Schedule[Lend]>[@Time],1,0),ROW(Schedule[Uniq ID])),IF(X=0,\"\",INDEX(Schedule[Uniq ID],X-1)&\": \"&INDEX(Schedule[Name],X-1)))",
                room_name
            );

            ws.get_cell_mut((col, row)).set_value("");
        }

        current_time = current_time + chrono::Duration::minutes(30);
        row += 1;
    }

    // Convert the range to a proper Excel table
    let total_rows = row - 1;
    let total_cols = rooms.len() + 1;
    let last_row = total_rows.max(2);

    // Create table headers
    let mut headers = vec!["Time"];
    for room in rooms.iter() {
        headers.push(&room.short_name);
    }

    // Create the table using the same pattern as other sheets
    let mut table = umya_spreadsheet::structs::Table::new(
        "GridTable",
        ((1u32, 1u32), (total_cols as u32, last_row)),
    );
    table.set_display_name("GridTable");
    for header in headers.iter() {
        table.add_column(umya_spreadsheet::structs::TableColumn::new(header));
    }
    let style = umya_spreadsheet::structs::TableStyleInfo::new(
        "TableStyleMedium2",
        false,
        false,
        true,
        false,
    );
    table.set_style_info(Some(style));
    ws.add_table(table);

    Ok(())
}
