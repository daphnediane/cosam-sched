/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the PanelTypes sheet → [`PanelTypeEntityType`] entities.

use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use umya_spreadsheet::Spreadsheet;

use crate::edit::builder::build_entity;
use crate::entity::{EntityType, EntityUuid, UuidPreference};
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::panel_type::{self, PanelTypeEntityType, PanelTypeId};
use crate::xlsx::columns::panel_types as pt;

use super::{
    build_column_map, find_data_range, get_field_def, is_truthy, known_field_key_set,
    route_extra_columns, row_to_map,
};

/// Read the PanelTypes sheet and populate the schedule with PanelType entities.
///
/// Returns a map from uppercase 2-letter prefix → `PanelTypeId` for use when
/// reading the Schedule sheet.
pub(super) fn read_panel_types_into(
    book: &Spreadsheet,
    preferred: &str,
    schedule: &mut Schedule,
    file_path: Option<&str>,
    import_time: DateTime<Utc>,
) -> Result<HashMap<String, PanelTypeId>> {
    let mut lookup: HashMap<String, PanelTypeId> = HashMap::new();

    let range = match find_data_range(book, preferred, &["Prefix", "PanelTypes"]) {
        Some(r) => r,
        None => return Ok(lookup),
    };

    let ws = match book.get_sheet_by_name(&range.sheet_name) {
        Some(ws) => ws,
        None => return Ok(lookup),
    };

    if !range.has_data() {
        return Ok(lookup);
    }

    let (raw_headers, canonical_headers, _col_map) = build_column_map(ws, &range);
    let known_keys = known_field_key_set(pt::ALL, &[]);

    for row in (range.header_row + 1)..=range.end_row {
        let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        let prefix = match get_field_def(&data, &pt::PREFIX) {
            Some(p) if !p.is_empty() => p.to_uppercase(),
            _ => continue,
        };
        // Normalize to 2-letter prefix (matches PanelUniqId behavior).
        let prefix = if prefix.len() > 2 {
            prefix[..2].to_string()
        } else {
            prefix
        };

        let panel_kind = get_field_def(&data, &pt::PANEL_KIND)
            .cloned()
            .unwrap_or_else(|| prefix.clone());

        let is_break = get_field_def(&data, &pt::IS_BREAK)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| panel_kind.to_lowercase().starts_with("br"));

        let is_cafe = get_field_def(&data, &pt::IS_CAFE)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| {
                let lower = panel_kind.to_lowercase();
                lower == "cafe" || lower == "café"
            });

        let is_workshop = get_field_def(&data, &pt::IS_WORKSHOP)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix.len() == 2 && prefix.ends_with('W'));

        let is_room_hours = get_field_def(&data, &pt::IS_ROOM_HOURS)
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let is_timeline = get_field_def(&data, &pt::IS_TIMELINE)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix == "SP" || prefix.starts_with("SP"));

        let is_private = get_field_def(&data, &pt::IS_PRIVATE)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix == "SM" || prefix == "ZZ");

        let hidden = get_field_def(&data, &pt::HIDDEN)
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        // If is_private is true, ignore the hidden flag and set it to false
        let hidden = if is_private { false } else { hidden };

        let color = get_field_def(&data, &pt::COLOR)
            .filter(|s| !s.is_empty())
            .cloned();

        let bw = get_field_def(&data, &pt::BW_COLOR)
            .filter(|s| !s.is_empty())
            .cloned();

        let uuid_pref = UuidPreference::PreferFromV5 {
            name: prefix.clone(),
        };
        let mut updates: Vec<FieldUpdate<PanelTypeEntityType>> = vec![
            FieldUpdate::set(&panel_type::FIELD_PREFIX, prefix.as_str()),
            FieldUpdate::set(&panel_type::FIELD_PANEL_KIND, panel_kind.as_str()),
            FieldUpdate::set(&panel_type::FIELD_HIDDEN, hidden),
            FieldUpdate::set(&panel_type::FIELD_IS_BREAK, is_break),
            FieldUpdate::set(&panel_type::FIELD_IS_CAFE, is_cafe),
            FieldUpdate::set(&panel_type::FIELD_IS_WORKSHOP, is_workshop),
            FieldUpdate::set(&panel_type::FIELD_IS_ROOM_HOURS, is_room_hours),
            FieldUpdate::set(&panel_type::FIELD_IS_TIMELINE, is_timeline),
            FieldUpdate::set(&panel_type::FIELD_IS_PRIVATE, is_private),
        ];
        if let Some(ref c) = color {
            updates.push(FieldUpdate::set(&panel_type::FIELD_COLOR, c.as_str()));
        }
        if let Some(ref b) = bw {
            updates.push(FieldUpdate::set(&panel_type::FIELD_BW, b.as_str()));
        }

        match build_entity::<PanelTypeEntityType>(schedule, uuid_pref, updates) {
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
                route_extra_columns(
                    ws,
                    row,
                    &range,
                    &raw_headers,
                    &canonical_headers,
                    &known_keys,
                    &[],
                    id.entity_uuid(),
                    PanelTypeEntityType::TYPE_NAME,
                    schedule,
                );
                lookup.insert(prefix, id);
            }
            Err(e) => {
                eprintln!("xlsx import: skipping panel type {prefix:?}: {e}");
            }
        }
    }

    Ok(lookup)
}
