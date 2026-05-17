/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the Rooms sheet → [`EventRoomEntityType`] + [`HotelRoomEntityType`] entities.

use anyhow::Result;

use crate::edit::builder::find_or_create_entity;
use crate::entity::{EntityType, EntityUuid};
use crate::field::set::FieldUpdate;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::event_room::{self, EventRoomEntityType, EventRoomId};
use crate::tables::hotel_room::{self, HotelRoomEntityType};
use crate::xlsx::columns::room_map;

use super::{
    build_column_map, find_data_range, get_field_def, known_field_key_set, route_extra_columns,
    row_to_map,
};

impl super::ImportContext<'_> {
    /// Read the Rooms sheet and populate the schedule with EventRoom and HotelRoom entities.
    ///
    /// Reads hotel rooms from `self.hotel_lookup` (populated by `read_hotel_rooms`) and
    /// populates `self.room_lookup` (lowercase name → `EventRoomId`) for use when reading
    /// the Schedule sheet.
    ///
    /// Accumulates seen `EventRoom` UUIDs into `self.seen_rooms` and hotel rooms
    /// discovered via the inline `Hotel Room` column into `self.seen_hotel_rooms`.
    pub(super) fn read_rooms(&mut self) -> Result<()> {
        let mode = self.options.rooms.clone();

        let range = match find_data_range(
            self.book,
            self.csv_map,
            &mode,
            &["RoomMap", "Rooms", "EventRooms"],
        ) {
            Some(r) => r,
            None => return Ok(()),
        };

        let ws = match self.book.get_sheet_by_name(&range.sheet_name) {
            Some(ws) => ws,
            None => return Ok(()),
        };

        if !range.has_data() {
            return Ok(());
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

            let name_key = room_name.to_lowercase();

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

            let room_id = match find_or_create_entity::<EventRoomEntityType>(
                self.schedule,
                &name_key,
                updates,
            ) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("xlsx import: skipping room {room_name:?}: {e}");
                    continue;
                }
            };
            let room_uuid = room_id.entity_uuid();
            self.seen_rooms.insert(room_uuid);
            self.schedule.sidecar_mut().set_origin(
                room_uuid,
                EntityOrigin::Xlsx(XlsxSourceInfo {
                    file_path: self.file_path.map(str::to_owned),
                    sheet_name: range.sheet_name.clone(),
                    row_index: row,
                    import_time: self.import_time,
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
                room_uuid,
                EventRoomEntityType::TYPE_NAME,
                self.schedule,
            );

            // Register under both room_name (lowercase) and long_name for lookup.
            self.room_lookup.insert(name_key, room_id);
            if let Some(ref ln) = long_name {
                self.room_lookup.entry(ln.to_lowercase()).or_insert(room_id);
            }

            // Create / find HotelRoom and link it.
            if let Some(ref hr_name) = hotel_room_name {
                let hr_key = hr_name.to_lowercase();
                let hr_id = if let Some(&existing) = self.hotel_lookup.get(&hr_key) {
                    self.seen_hotel_rooms.insert(existing.entity_uuid());
                    existing
                } else {
                    match find_or_create_entity::<HotelRoomEntityType>(
                        self.schedule,
                        &hr_key,
                        vec![FieldUpdate::set(
                            &hotel_room::FIELD_HOTEL_ROOM_NAME,
                            hr_name.as_str(),
                        )],
                    ) {
                        Ok(id) => {
                            let uuid = id.entity_uuid();
                            self.seen_hotel_rooms.insert(uuid);
                            self.schedule.sidecar_mut().set_origin(
                                uuid,
                                EntityOrigin::Xlsx(XlsxSourceInfo {
                                    file_path: self.file_path.map(str::to_owned),
                                    sheet_name: range.sheet_name.clone(),
                                    row_index: row,
                                    import_time: self.import_time,
                                }),
                            );
                            self.hotel_lookup.insert(hr_key, id);
                            id
                        }
                        Err(e) => {
                            eprintln!("xlsx import: skipping hotel room {hr_name:?}: {e}");
                            continue;
                        }
                    }
                };
                // Replace the hotel room link (edge_set replaces, edge_add would duplicate).
                let _ = self
                    .schedule
                    .edge_set(room_id, event_room::EDGE_HOTEL_ROOMS, [hr_id]);
            } else {
                // No hotel room in this row — clear any existing hotel room link.
                let _ = self.schedule.edge_set(
                    room_id,
                    event_room::EDGE_HOTEL_ROOMS,
                    std::iter::empty::<EventRoomId>(),
                );
            }
        }

        Ok(())
    }
}
