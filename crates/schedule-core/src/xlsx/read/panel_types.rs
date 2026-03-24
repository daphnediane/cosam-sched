/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use anyhow::Result;
use indexmap::IndexMap;
use umya_spreadsheet::Spreadsheet;

use crate::data::panel_type::PanelType;
use crate::data::source_info::{ChangeState, SourceInfo};

use crate::xlsx::columns::panel_types as pt;

use super::{
    build_column_map, collect_extra_metadata, find_data_range, get_field_def, is_truthy, row_to_map,
};

pub(super) fn read_panel_types(
    book: &Spreadsheet,
    preferred: &str,
    file_path: &str,
) -> Result<IndexMap<String, PanelType>> {
    let range = match find_data_range(book, preferred, &["Prefix", "PanelTypes"]) {
        Some(r) => r,
        None => return Ok(IndexMap::new()),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok(IndexMap::new());
    }

    let (raw_headers, canonical_headers, _col_map) = build_column_map(ws, &range);
    let mut types = IndexMap::new();

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

        types.insert(
            prefix.clone(),
            PanelType {
                prefix: prefix.clone(),
                kind,
                colors,
                is_break,
                is_cafe,
                is_workshop,
                is_hidden,
                is_room_hours,
                is_timeline,
                is_private,
                metadata,
                source: Some(SourceInfo {
                    file_path: Some(file_path.to_string()),
                    sheet_name: Some(range.sheet_name.clone()),
                    row_index: Some(row),
                }),
                change_state: ChangeState::Unchanged,
            },
        );
    }

    Ok(types)
}
