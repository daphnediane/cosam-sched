/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the Rooms sheet → [`EventRoomEntityType`] + [`HotelRoomEntityType`] entities.

use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use umya_spreadsheet::Spreadsheet;

use crate::edit::builder::build_entity;
use crate::entity::{EntityType, EntityUuid, UuidPreference};
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::event_room::{self, EventRoomEntityType, EventRoomId};
use crate::tables::hotel_room::{self, HotelRoomEntityType};
use crate::xlsx::columns::room_map;

use super::{
    build_column_map, find_data_range, get_field_def, known_field_key_set, route_extra_columns,
    row_to_map,
};

/// Read the Rooms sheet and populate the schedule with EventRoom and HotelRoom entities.
///
/// Returns a map from lowercase room name → `EventRoomId` for use when reading
/// the Schedule sheet.
///
/// The `hotel_lookup` parameter contains hotel rooms already created from the Hotels sheet.
/// Event rooms will be linked to these existing hotel rooms, and new hotel rooms will only
/// be created for hotel room names not found in `hotel_lookup`.
pub(super) fn read_rooms_into(
    book: &Spreadsheet,
    preferred: &str,
    schedule: &mut Schedule,
    file_path: Option<&str>,
    import_time: DateTime<Utc>,
    hotel_lookup: &HashMap<String, hotel_room::HotelRoomId>,
) -> Result<HashMap<String, EventRoomId>> {
    let mut room_lookup: HashMap<String, EventRoomId> = HashMap::new();
    // Clone the hotel_lookup so we can add new hotel rooms found in the Rooms sheet.
    let mut hotel_lookup: HashMap<String, hotel_room::HotelRoomId> = hotel_lookup.clone();

    let range = match find_data_range(book, preferred, &["RoomMap", "Rooms"]) {
        Some(r) => r,
        None => return Ok(room_lookup),
    };

    let ws = match book.get_sheet_by_name(&range.sheet_name) {
        Some(ws) => ws,
        None => return Ok(room_lookup),
    };

    if !range.has_data() {
        return Ok(room_lookup);
    }

    let (raw_headers, canonical_headers, _col_map) = build_column_map(ws, &range);
    let known_keys = known_field_key_set(room_map::ALL, &[]);

    for row in (range.header_row + 1)..=range.end_row {
        let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        let room_name = match get_field_def(&data, &room_map::ROOM_NAME) {
            Some(n) if !n.is_empty() => n.clone(),
            _ => continue,
        };

        let long_name = get_field_def(&data, &room_map::LONG_NAME)
            .filter(|n| n != &"#ERROR!")
            .cloned();

        let sort_key = get_field_def(&data, &room_map::SORT_KEY)
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| f as i64);

        let hotel_room_name = get_field_def(&data, &room_map::HOTEL_ROOM)
            .filter(|s| !s.is_empty())
            .cloned();

        let is_pseudo =
            get_field_def(&data, &room_map::IS_PSEUDO).is_some_and(|s| super::is_truthy(s));

        // Build EventRoom via field system.
        let uuid_pref = UuidPreference::PreferFromV5 {
            name: room_name.to_lowercase(),
        };
        let mut updates: Vec<FieldUpdate<EventRoomEntityType>> = vec![FieldUpdate::set(
            &event_room::FIELD_ROOM_NAME,
            room_name.as_str(),
        )];
        if let Some(ref ln) = long_name {
            updates.push(FieldUpdate::set(&event_room::FIELD_LONG_NAME, ln.as_str()));
        }
        if let Some(sk) = sort_key {
            updates.push(FieldUpdate::set(&event_room::FIELD_SORT_KEY, sk));
        }
        if is_pseudo {
            updates.push(FieldUpdate::set(&event_room::FIELD_IS_PSEUDO, true));
        }

        let room_id = match build_entity::<EventRoomEntityType>(schedule, uuid_pref, updates) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("xlsx import: skipping room {room_name:?}: {e}");
                continue;
            }
        };
        schedule.sidecar_mut().set_origin(
            room_id.entity_uuid(),
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
            room_id.entity_uuid(),
            EventRoomEntityType::TYPE_NAME,
            schedule,
        );

        // Register under both room_name (lowercase) and long_name for lookup.
        room_lookup.insert(room_name.to_lowercase(), room_id);
        if let Some(ref ln) = long_name {
            room_lookup.entry(ln.to_lowercase()).or_insert(room_id);
        }

        // Create / find HotelRoom and link it.
        if let Some(ref hr_name) = hotel_room_name {
            let hr_id = if let Some(&existing) = hotel_lookup.get(&hr_name.to_lowercase()) {
                existing
            } else {
                let hr_uuid = UuidPreference::PreferFromV5 {
                    name: hr_name.to_lowercase(),
                };
                match build_entity::<HotelRoomEntityType>(
                    schedule,
                    hr_uuid,
                    vec![FieldUpdate::set(
                        &hotel_room::FIELD_HOTEL_ROOM_NAME,
                        hr_name.as_str(),
                    )],
                ) {
                    Ok(id) => {
                        schedule.sidecar_mut().set_origin(
                            id.entity_uuid(),
                            EntityOrigin::Xlsx(XlsxSourceInfo {
                                file_path: file_path.map(str::to_owned),
                                sheet_name: range.sheet_name.clone(),
                                row_index: row,
                                import_time,
                            }),
                        );
                        hotel_lookup.insert(hr_name.to_lowercase(), id);
                        id
                    }
                    Err(e) => {
                        eprintln!("xlsx import: skipping hotel room {hr_name:?}: {e}");
                        continue;
                    }
                }
            };
            let _ = schedule.edge_add(room_id, event_room::EDGE_HOTEL_ROOMS, [hr_id]);
        }
    }

    Ok(room_lookup)
}
