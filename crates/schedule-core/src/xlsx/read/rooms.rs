/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use anyhow::Result;
use umya_spreadsheet::Spreadsheet;

use crate::data::source_info::{ChangeState, SourceInfo};
use crate::edit::context::EditContext;
use crate::edit::find::RoomOptions;

use crate::xlsx::columns::room_map;

use super::{build_column_map, collect_extra_metadata, find_data_range, get_field_def, row_to_map};

/// Read rooms from the workbook and populate `ctx.schedule.rooms` via
/// `find_or_create_room`.  The schedule's room list is sorted by sort_key
/// before returning.
pub(super) fn read_rooms_into(
    book: &Spreadsheet,
    preferred: &str,
    file_path: &str,
    ctx: &mut EditContext<'_>,
) -> Result<()> {
    let range = match find_data_range(book, preferred, &["RoomMap", "Rooms"]) {
        Some(r) => r,
        None => return Ok(()),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok(());
    }

    let (raw_headers, canonical_headers, _col_map) = build_column_map(ws, &range);
    let mut next_uid: u32 = 1;

    for row in (range.header_row + 1)..=range.end_row {
        let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        let short_name = get_field_def(&data, &room_map::ROOM_NAME).cloned();
        let long_name_raw = get_field_def(&data, &room_map::LONG_NAME).cloned();
        let hotel_room = get_field_def(&data, &room_map::HOTEL_ROOM)
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

        let sort_key: u32 = get_field_def(&data, &room_map::SORT_KEY)
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| f as u32)
            .unwrap_or(999);

        let uid = next_uid;
        next_uid += 1;

        // Only the primary columns are "known"; EXTRA columns are not first-class
        // fields and intentionally flow through to room metadata for round tripping.
        let metadata = collect_extra_metadata(&data, &raw_headers, room_map::ALL);

        ctx.find_or_create_room(
            &short_name,
            &RoomOptions {
                long_name: Some(long_name),
                hotel_room: Some(hotel_room),
                sort_key: Some(sort_key),
                is_break: Some(false),
                metadata,
                uid: Some(uid),
                source: Some(SourceInfo {
                    file_path: Some(file_path.to_string()),
                    sheet_name: Some(range.sheet_name.clone()),
                    row_index: Some(row),
                }),
                change_state: Some(ChangeState::Unchanged),
            },
        );
    }

    ctx.schedule.rooms.sort_by_key(|r| r.sort_key);
    Ok(())
}
