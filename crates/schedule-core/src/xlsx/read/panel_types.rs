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

use super::{
    build_column_map, collect_extra_metadata, find_data_range, get_field, is_truthy, row_to_map,
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

        let prefix = match get_field(&data, &["Prefix"]) {
            Some(p) if !p.is_empty() => p.to_uppercase(),
            _ => continue,
        };

        let kind = get_field(&data, &["Panel_Kind", "PanelKind", "Kind"])
            .cloned()
            .unwrap_or_else(|| prefix.clone());

        let is_break = get_field(&data, &["Is_Break"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| kind.to_lowercase().starts_with("br"));

        let is_cafe = get_field(&data, &["Is_Cafe", "Is_Café"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| {
                let lower = kind.to_lowercase();
                lower == "cafe" || lower == "café"
            });

        let is_workshop = get_field(&data, &["Is_Workshop"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix.len() == 2 && prefix.ends_with('W'));

        let is_room_hours = get_field(&data, &["Is_Room_Hours", "IsRoomHours"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let _is_split = get_field(&data, &["Is_Split"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| {
                prefix == "SPLIT"
                    || prefix.to_uppercase().starts_with("SP")
                    || prefix.to_uppercase().starts_with("SPLIT")
            });

        let mut colors = IndexMap::new();
        if let Some(c) = get_field(&data, &["Color"]).cloned() {
            if !c.is_empty() {
                colors.insert("color".to_string(), c);
            }
        }
        if let Some(bw) = get_field(&data, &["BW", "Bw"]).cloned() {
            if !bw.is_empty() {
                colors.insert("bw".to_string(), bw);
            }
        }

        let is_hidden = get_field(&data, &["Hidden"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        let is_timeline = get_field(&data, &["Is_TimeLine", "Is_Timeline", "IsTimeLine"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix == "SPLIT" || prefix.starts_with("SP"));

        let is_private = get_field(&data, &["Is_Private", "IsPrivate"])
            .map(|s| is_truthy(s))
            .unwrap_or_else(|| prefix == "SM" || prefix == "ZZ");

        const PANEL_TYPE_KNOWN: &[&str] = &[
            "Prefix",
            "Panel_Kind",
            "PanelKind",
            "Kind",
            "Color",
            "BW",
            "Bw",
            "Is_Break",
            "Is_Cafe",
            "Is_Café",
            "Is_Workshop",
            "Is_Room_Hours",
            "IsRoomHours",
            "Is_Split",
            "Hidden",
            "Is_TimeLine",
            "Is_Timeline",
            "IsTimeLine",
            "Is_Private",
            "IsPrivate",
        ];
        let metadata = collect_extra_metadata(&data, &raw_headers, PANEL_TYPE_KNOWN);

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
