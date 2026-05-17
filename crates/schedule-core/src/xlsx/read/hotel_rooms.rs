/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the Hotel/Hotel Rooms/HotelMap sheet → [`HotelRoomEntityType`] entities.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use uuid::NonNilUuid;

use crate::edit::builder::find_or_create_entity;
use crate::entity::{EntityType, EntityUuid};
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::hotel_room::{self, HotelRoomEntityType, HotelRoomId};
use crate::xlsx::columns::hotel_rooms;

use super::{
    build_column_map, find_data_range, get_field_def, known_field_key_set, route_extra_columns,
    row_to_map, TableImportMode,
};

/// Read the Hotel/Hotel Rooms/HotelMap sheet and populate the schedule with HotelRoom entities.
///
/// Returns a map from lowercase hotel room name → `HotelRoomId` for use when
/// reading the Rooms sheet (to link event rooms to hotel rooms), plus the set
/// of UUIDs seen during this import (for soft-delete of removed entries).
pub(super) fn read_hotel_rooms_into(
    ctx: &mut super::ImportContext<'_>,
    mode: &TableImportMode,
    schedule: &mut Schedule,
) -> Result<(HashMap<String, HotelRoomId>, HashSet<NonNilUuid>)> {
    let mut hotel_lookup: HashMap<String, HotelRoomId> = HashMap::new();
    let mut seen: HashSet<NonNilUuid> = HashSet::new();

    let range = match find_data_range(ctx, mode, &["Hotel", "Hotel Rooms", "HotelMap"]) {
        Some(r) => r,
        None => return Ok((hotel_lookup, seen)),
    };

    let ws = match ctx.book.get_sheet_by_name(&range.sheet_name) {
        Some(ws) => ws,
        None => return Ok((hotel_lookup, seen)),
    };

    if !range.has_data() {
        return Ok((hotel_lookup, seen));
    }

    let (raw_headers, canonical_headers, _col_map) = build_column_map(ws, &range);
    let known_keys = known_field_key_set(hotel_rooms::ALL, &[]);

    for row in (range.header_row + 1)..=range.end_row {
        let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        let hotel_room_name = match get_field_def(&data, &hotel_rooms::HOTEL_ROOM_NAME) {
            Some(n) if !n.is_empty() => n.clone(),
            _ => continue,
        };

        let long_name = get_field_def(&data, &hotel_rooms::LONG_NAME)
            .filter(|n| n != &"#ERROR!")
            .cloned();

        let sort_key = get_field_def(&data, &hotel_rooms::SORT_KEY)
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| f as i64);

        let name_key = hotel_room_name.to_lowercase();
        if hotel_lookup.contains_key(&name_key) {
            continue;
        }

        let mut updates: Vec<FieldUpdate<HotelRoomEntityType>> = vec![FieldUpdate::set(
            &hotel_room::FIELD_HOTEL_ROOM_NAME,
            hotel_room_name.as_str(),
        )];

        if let Some(ref ln) = long_name {
            updates.push(FieldUpdate::set(&hotel_room::FIELD_LONG_NAME, ln.as_str()));
        }
        if let Some(sk) = sort_key {
            updates.push(FieldUpdate::set(&hotel_room::FIELD_SORT_KEY, sk));
        }

        match find_or_create_entity::<HotelRoomEntityType>(schedule, &name_key, updates) {
            Ok(id) => {
                let uuid = id.entity_uuid();
                seen.insert(uuid);
                schedule.sidecar_mut().set_origin(
                    uuid,
                    EntityOrigin::Xlsx(XlsxSourceInfo {
                        file_path: ctx.file_path.map(str::to_owned),
                        sheet_name: range.sheet_name.clone(),
                        row_index: row,
                        import_time: ctx.import_time,
                    }),
                );

                route_extra_columns(
                    ws,
                    row,
                    &range,
                    &raw_headers,
                    &canonical_headers,
                    &known_keys,
                    &[],
                    &std::collections::HashSet::new(),
                    uuid,
                    HotelRoomEntityType::TYPE_NAME,
                    schedule,
                );

                hotel_lookup.insert(name_key, id);
            }
            Err(e) => {
                eprintln!("xlsx import: skipping hotel room {hotel_room_name:?}: {e}");
            }
        }
    }

    Ok((hotel_lookup, seen))
}
