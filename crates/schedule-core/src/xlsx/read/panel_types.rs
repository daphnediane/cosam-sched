/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use anyhow::Result;
use indexmap::IndexMap;
use umya_spreadsheet::Spreadsheet;

use crate::data::source_info::{ChangeState, SourceInfo};
use crate::edit::context::EditContext;
use crate::edit::find::PanelTypeOptions;

use crate::xlsx::columns::panel_types as pt;

use super::{
    build_column_map, collect_extra_metadata, find_data_range, get_field_def, is_truthy, row_to_map,
};

/// Read panel types from the workbook and populate
/// `ctx.schedule.panel_types` via `find_or_create_panel_type`.
pub(super) fn read_panel_types_into(
    book: &Spreadsheet,
    preferred: &str,
    file_path: &str,
    ctx: &mut EditContext<'_>,
) -> Result<()> {
    let range = match find_data_range(book, preferred, &["Prefix", "PanelTypes"]) {
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

    for row in (range.header_row + 1)..=range.end_row {
        let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        let prefix = match get_field_def(&data, &pt::PREFIX) {
            Some(p) if !p.is_empty() => p.to_uppercase(),
            _ => continue,
        };

        let kind = get_field_def(&data, &pt::PANEL_KIND)
            .cloned()
            .unwrap_or_else(|| prefix.clone());

        let is_break = get_field_def(&data, &pt::IS_BREAK)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| kind.to_lowercase().starts_with("br"));

        let is_cafe = get_field_def(&data, &pt::IS_CAFE)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| {
                let lower = kind.to_lowercase();
                lower == "cafe" || lower == "café"
            });

        let is_workshop = get_field_def(&data, &pt::IS_WORKSHOP)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix.len() == 2 && prefix.ends_with('W'));

        let is_room_hours = get_field_def(&data, &pt::IS_ROOM_HOURS)
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let mut colors = IndexMap::new();
        if let Some(c) = get_field_def(&data, &pt::COLOR).cloned() {
            if !c.is_empty() {
                colors.insert("color".to_string(), c);
            }
        }
        if let Some(bw) = get_field_def(&data, &pt::BW_COLOR).cloned() {
            if !bw.is_empty() {
                colors.insert("bw".to_string(), bw);
            }
        }

        let is_hidden = get_field_def(&data, &pt::HIDDEN)
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        let is_timeline = get_field_def(&data, &pt::IS_TIMELINE)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix == "SPLIT" || prefix.starts_with("SP"));

        let is_private = get_field_def(&data, &pt::IS_PRIVATE)
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix == "SM" || prefix == "ZZ");

        let metadata = collect_extra_metadata(&data, &raw_headers, pt::ALL);

        ctx.find_or_create_panel_type(
            &prefix,
            &PanelTypeOptions {
                kind: Some(kind),
                colors: if colors.is_empty() {
                    None
                } else {
                    Some(colors)
                },
                is_break: Some(is_break),
                is_cafe: Some(is_cafe),
                is_workshop: Some(is_workshop),
                is_hidden: Some(is_hidden),
                is_room_hours: Some(is_room_hours),
                is_timeline: Some(is_timeline),
                is_private: Some(is_private),
                metadata,
                source: Some(SourceInfo {
                    file_path: Some(file_path.to_string()),
                    sheet_name: Some(range.sheet_name.clone()),
                    row_index: Some(row),
                }),
                change_state: Some(ChangeState::Unchanged),
                ..Default::default()
            },
        );
    }

    Ok(())
}
