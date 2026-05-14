/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the Timeline sheet → [`TimelineEntityType`] entities.

use std::collections::HashMap;

use anyhow::Result;

use crate::edit::builder::build_entity;
use crate::entity::{EntityType, EntityUuid, UuidPreference};
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::panel_type::PanelTypeId;
use crate::tables::timeline::{self, TimelineEntityType, TimelineId};
use crate::value::time::parse_datetime;
use crate::value::uniq_id::PanelUniqId;
use crate::xlsx::columns::timeline as tl_cols;

use super::{
    build_column_map, find_data_range, get_cell_str, get_field_def, known_field_key_set,
    route_extra_columns, row_to_map, TableImportMode,
};

/// Read the Timeline sheet and create Timeline entities.
///
/// Should be called before the Schedule sheet import so that panel types
/// are properly resolved.
pub(super) fn read_timeline_into(
    ctx: &mut super::ImportContext<'_>,
    mode: &TableImportMode,
    schedule: &mut Schedule,
    panel_type_lookup: &HashMap<String, PanelTypeId>,
) -> Result<()> {
    let range = match find_data_range(ctx, mode, &["Timeline", "KeyTimes"]) {
        Some(r) => r,
        None => return Ok(()),
    };

    let ws = match ctx.book.get_sheet_by_name(&range.sheet_name) {
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

        // Parse Uniq ID; skip soft-deleted rows (leading `*`).
        let raw_uniq_id = get_field_def(&data, &tl_cols::UNIQ_ID).cloned();
        let (uniq_id_str, is_deleted) = match raw_uniq_id {
            Some(ref s) if s.starts_with('*') => {
                (Some(s.trim_start_matches('*').to_string()), true)
            }
            other => (other, false),
        };
        if is_deleted {
            continue; // Soft-deleted rows are excluded from import.
        }

        // Parse time.
        let time = time_col
            .and_then(|c| get_cell_str(ws, c, row))
            .and_then(|s| parse_datetime(&s));

        // Determine panel type from prefix or Panel Types column.
        let parsed_code = uniq_id_str.as_deref().and_then(PanelUniqId::parse);
        let panel_type_id: Option<PanelTypeId> = parsed_code
            .as_ref()
            .and_then(|c| panel_type_lookup.get(&c.prefix))
            .copied()
            .or_else(|| {
                get_field_def(&data, &tl_cols::PANEL_TYPES).and_then(|pt| {
                    panel_type_lookup
                        .values()
                        .find(|&&pt_id| {
                            schedule
                                .get_internal::<crate::tables::panel_type::PanelTypeEntityType>(
                                    pt_id,
                                )
                                .map(|d| {
                                    d.data.prefix.to_lowercase() == pt.to_lowercase()
                                        || d.data.panel_kind.to_lowercase() == pt.to_lowercase()
                                })
                                .unwrap_or(false)
                        })
                        .copied()
                })
            });

        // Determine Uniq ID string (synthesize row-based ID if missing).
        let code_str = uniq_id_str.unwrap_or_else(|| format!("XX{row:03}"));

        // Build Timeline entity via field system.
        let uuid_pref = UuidPreference::PreferFromV5 {
            name: code_str.to_uppercase(),
        };
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

        let timeline_id: TimelineId =
            match build_entity::<TimelineEntityType>(schedule, uuid_pref, updates) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("xlsx import: skipping timeline {code_str:?}: {e}");
                    continue;
                }
            };

        schedule.sidecar_mut().set_origin(
            timeline_id.entity_uuid(),
            EntityOrigin::Xlsx(XlsxSourceInfo {
                file_path: ctx.file_path.map(str::to_owned),
                sheet_name: range.sheet_name.clone(),
                row_index: row,
                import_time: ctx.import_time,
            }),
        );

        // Wire panel type edge to timeline.
        if let Some(pt_id) = panel_type_id {
            let _ = schedule.edge_add(timeline_id, timeline::EDGE_PANEL_TYPES, [pt_id]);
        }

        route_extra_columns(
            ws,
            row,
            &range,
            &raw_headers,
            &canonical_headers,
            &known_keys,
            &[],
            &std::collections::HashSet::new(),
            timeline_id.entity_uuid(),
            TimelineEntityType::TYPE_NAME,
            schedule,
        );
    }

    Ok(())
}
