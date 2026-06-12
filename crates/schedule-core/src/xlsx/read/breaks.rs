/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the optional Breaks sheet → [`BreakEntityType`] entities.
//!
//! Breaks are also extracted from break-type rows on the main Schedule sheet
//! (see [`super::schedule`]); this reader handles a dedicated `Breaks` sheet
//! mirroring the Timeline sheet, but with a duration (start + end/duration).

use anyhow::Result;

use crate::edit::builder::find_or_create_entity;
use crate::entity::{EntityType, EntityUuid};
use crate::field::set::FieldUpdate;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::breaks::{self, BreakEntityType, BreakId};
use crate::value::time::{parse_datetime, parse_duration};
use crate::value::uniq_id::PanelUniqId;
use crate::xlsx::columns::breaks as br_cols;

use super::{
    build_column_map, find_data_range, get_cell_str, get_field_def, known_field_key_set,
    parse_old_codes, route_extra_columns, row_to_map,
};

impl super::ImportContext<'_> {
    /// Read the optional Breaks sheet and create Break entities.
    ///
    /// Should be called after panel types are read so the type prefix resolves.
    /// Accumulates seen Break UUIDs into `self.seen_breaks`.
    pub(super) fn read_breaks(&mut self) -> Result<()> {
        let mode = self.options.breaks.clone();

        let range = match find_data_range(self.book, self.csv_map, &mode, &["Breaks", "Break"]) {
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
        // `READ_ONLY` columns (e.g. END_TIME) are accepted as input but never
        // exported; include them so they aren't routed to the sidecar.
        let known_keys = known_field_key_set(br_cols::ALL, br_cols::READ_ONLY);

        let start_col = col_map.get(br_cols::START_TIME.canonical).copied();
        let end_col = col_map.get(br_cols::END_TIME.canonical).copied();
        let dur_col = col_map.get(br_cols::DURATION.canonical).copied();

        for row in (range.header_row + 1)..=range.end_row {
            let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

            // Require a Name field.
            let name = match get_field_def(&data, &br_cols::NAME) {
                Some(n) if !n.is_empty() => n.clone(),
                _ => continue,
            };

            // A `*` anywhere on the Uniq ID marks the entry as *unscheduled*: it
            // still imports (the code is not required), but with no time.
            let raw_uniq_id = get_field_def(&data, &br_cols::UNIQ_ID).cloned();
            let force_unscheduled = raw_uniq_id.as_deref().is_some_and(|s| s.contains('*'));
            let uniq_id_str = raw_uniq_id
                .map(|s| s.replace('*', "").trim().to_string())
                .filter(|s| !s.is_empty());

            // Parse timing (start cleared for unscheduled entries).
            let start = if force_unscheduled {
                None
            } else {
                start_col
                    .and_then(|c| get_cell_str(ws, c, row))
                    .and_then(|s| parse_datetime(&s))
            };
            let end = end_col
                .and_then(|c| get_cell_str(ws, c, row))
                .and_then(|s| parse_datetime(&s));
            let duration = dur_col
                .and_then(|c| get_cell_str(ws, c, row))
                .and_then(|s| parse_duration(&s));

            // A break's panel type is always derived from its Uniq ID prefix
            // (e.g. `BREAK001` → `BR`); there is no Panel Types column.
            let parsed_code = uniq_id_str.as_deref().and_then(PanelUniqId::parse);

            // Determine Uniq ID string (synthesize row-based ID if missing).
            let code_str = uniq_id_str.unwrap_or_else(|| format!("BREAK{row:03}"));
            let upsert_name = parsed_code
                .as_ref()
                .map(|c| c.full_id())
                .unwrap_or_else(|| code_str.to_uppercase());

            let mut updates: Vec<FieldUpdate<BreakEntityType>> = vec![
                FieldUpdate::set(&breaks::FIELD_CODE, code_str.as_str()),
                FieldUpdate::set(&breaks::FIELD_NAME, name.as_str()),
            ];

            if let Some(ref d) = get_field_def(&data, &br_cols::DESCRIPTION).cloned() {
                updates.push(FieldUpdate::set(&breaks::FIELD_DESCRIPTION, d.as_str()));
            }
            if let Some(ref n) = get_field_def(&data, &br_cols::NOTE).cloned() {
                updates.push(FieldUpdate::set(&breaks::FIELD_NOTE, n.as_str()));
            }
            if let Some(st) = start {
                updates.push(FieldUpdate::set(&breaks::FIELD_START_TIME, st));
            }
            // End takes precedence over duration when both are present
            // (matching the Schedule-sheet timing resolution).
            if let Some(et) = end {
                updates.push(FieldUpdate::set(&breaks::FIELD_END_TIME, et));
            } else if let Some(dur) = duration {
                updates.push(FieldUpdate::set(&breaks::FIELD_DURATION, dur));
            }
            let old_codes = parse_old_codes(&data, &br_cols::OLD_UNIQ_ID);
            if !old_codes.is_empty() {
                updates.push(FieldUpdate::set(&breaks::FIELD_OLD_CODES, old_codes));
            }

            let break_id: BreakId = match find_or_create_entity::<BreakEntityType>(
                self.schedule,
                &upsert_name,
                &self.seen_breaks,
                false,
                updates,
            ) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("xlsx import: skipping break {code_str:?}: {e}");
                    continue;
                }
            };

            let uuid = break_id.entity_uuid();
            self.seen_breaks.insert(uuid);
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
                BreakEntityType::TYPE_NAME,
                self.schedule,
            );
        }

        Ok(())
    }
}
