/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the Hotel/Hotel Rooms/HotelMap sheet → [`HotelRoomEntityType`] entities.

use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use umya_spreadsheet::Spreadsheet;

use crate::edit::builder::build_entity;
use crate::entity::{EntityType, EntityUuid, UuidPreference};
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
/// reading the Rooms sheet (to link event rooms to hotel rooms).
pub(super) fn read_hotel_rooms_into(
    book: &Spreadsheet,
    mode: &TableImportMode,
    schedule: &mut Schedule,
    file_path: Option<&str>,
    import_time: DateTime<Utc>,
) -> Result<HashMap<String, HotelRoomId>> {
    let mut hotel_lookup: HashMap<String, HotelRoomId> = HashMap::new();

    let range = match find_data_range(book, mode, &["Hotel", "Hotel Rooms", "HotelMap"]) {
        Some(r) => r,
        None => return Ok(hotel_lookup),
    };

    let ws = match book.get_sheet_by_name(&range.sheet_name) {
        Some(ws) => ws,
        None => return Ok(hotel_lookup),
    };

    if !range.has_data() {
        return Ok(hotel_lookup);
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

        // Skip if already exists (by name)
        let name_key = hotel_room_name.to_lowercase();
        if hotel_lookup.contains_key(&name_key) {
            continue;
        }

        // Build HotelRoom via field system.
        let uuid_pref = UuidPreference::PreferFromV5 {
            name: name_key.clone(),
        };
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

        let hotel_id = match build_entity::<HotelRoomEntityType>(schedule, uuid_pref, updates) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("xlsx import: skipping hotel room {hotel_room_name:?}: {e}");
                continue;
            }
        };

        schedule.sidecar_mut().set_origin(
            hotel_id.entity_uuid(),
            EntityOrigin::Xlsx(XlsxSourceInfo {
                file_path: file_path.map(str::to_owned),
                sheet_name: range.sheet_name.clone(),
                row_index: row,
                import_time,
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
            hotel_id.entity_uuid(),
            HotelRoomEntityType::TYPE_NAME,
            schedule,
        );

        hotel_lookup.insert(name_key, hotel_id);
    }

    Ok(hotel_lookup)
}
