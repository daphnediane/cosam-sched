/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the Timeline sheet → [`TimelineEntityType`] entities.

use anyhow::Result;

use crate::edit::builder::find_or_create_entity;
use crate::entity::{EntityType, EntityUuid};
use crate::field::set::FieldUpdate;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::timeline::{self, TimelineEntityType, TimelineId};
use crate::value::time::parse_datetime;
use crate::value::uniq_id::PanelUniqId;
use crate::xlsx::columns::timeline as tl_cols;

use super::{
    build_column_map, find_data_range, get_cell_str, get_field_def, known_field_key_set,
    route_extra_columns, row_to_map,
};

impl super::ImportContext<'_> {
    /// Read the Timeline sheet and create Timeline entities.
    ///
    /// Should be called before the Schedule sheet import so that panel types
    /// are properly resolved.
    ///
    /// Accumulates seen Timeline UUIDs into `self.seen_timelines`.
    pub(super) fn read_timeline(&mut self) -> Result<()> {
        let mode = self.options.timeline.clone();

        let range = match find_data_range(self.book, self.csv_map, &mode, &["Timeline", "KeyTimes"])
        {
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

        let (raw_headers, canonical_headers, col_map) = build_column_map(ws, &range);
        let known_keys = known_field_key_set(tl_cols::ALL, &[]);

        let time_col = col_map.get(tl_cols::TIME.canonical).copied();

        for row in (range.header_row + 1)..=range.end_row {
            let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

            // Require a Name field.
            let name = match get_field_def(&data, &tl_cols::NAME) {
                Some(n) if !n.is_empty() => n.clone(),
                _ => continue,
            };

            // A `*` anywhere on the Uniq ID marks the entry as *unscheduled*: it
            // still imports (the code is not required), but with no time.
            // Deletion is done by removing the row from the sheet.
            let raw_uniq_id = get_field_def(&data, &tl_cols::UNIQ_ID).cloned();
            let force_unscheduled = raw_uniq_id.as_deref().is_some_and(|s| s.contains('*'));
            let uniq_id_str = raw_uniq_id
                .map(|s| s.replace('*', "").trim().to_string())
                .filter(|s| !s.is_empty());

            // Parse time (cleared for unscheduled entries).
            let time = if force_unscheduled {
                None
            } else {
                time_col
                    .and_then(|c| get_cell_str(ws, c, row))
                    .and_then(|s| parse_datetime(&s))
            };

            // A timeline's panel type is derived from its Uniq ID prefix.
            let parsed_code = uniq_id_str.as_deref().and_then(PanelUniqId::parse);

            // Determine Uniq ID string (synthesize row-based ID if missing).
            // Normalize via PanelUniqId so the upsert key matches what find_by_code
            // will read back from stored full_id() after parse+normalize.
            let code_str = uniq_id_str.unwrap_or_else(|| format!("XX{row:03}"));
            let upsert_name = parsed_code
                .as_ref()
                .map(|c| c.full_id())
                .unwrap_or_else(|| code_str.to_uppercase());

            let mut updates: Vec<FieldUpdate<TimelineEntityType>> = vec![
                FieldUpdate::set(&timeline::FIELD_CODE, code_str.as_str()),
                FieldUpdate::set(&timeline::FIELD_NAME, name.as_str()),
            ];

            if let Some(ref d) = get_field_def(&data, &tl_cols::DESCRIPTION).cloned() {
                updates.push(FieldUpdate::set(&timeline::FIELD_DESCRIPTION, d.as_str()));
            }
            if let Some(ref n) = get_field_def(&data, &tl_cols::NOTE).cloned() {
                updates.push(FieldUpdate::set(&timeline::FIELD_NOTE, n.as_str()));
            }
            if let Some(t) = time {
                updates.push(FieldUpdate::set(&timeline::FIELD_TIME, t));
            }

            let timeline_id: TimelineId = match find_or_create_entity::<TimelineEntityType>(
                self.schedule,
                &upsert_name,
                &self.seen_timelines,
                false,
                updates,
            ) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("xlsx import: skipping timeline {code_str:?}: {e}");
                    continue;
                }
            };

            let uuid = timeline_id.entity_uuid();
            self.seen_timelines.insert(uuid);
            self.schedule.sidecar_mut().set_origin(
                uuid,
                EntityOrigin::Xlsx(XlsxSourceInfo {
                    file_path: self.file_path.map(str::to_owned),
                    sheet_name: range.sheet_name.clone(),
                    row_index: row,
                    import_time: self.import_time,
                }),
            );

            // Panel type is derived from the Uniq ID prefix — no edge to set.

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
                TimelineEntityType::TYPE_NAME,
                self.schedule,
            );
        }

        Ok(())
    }
}
