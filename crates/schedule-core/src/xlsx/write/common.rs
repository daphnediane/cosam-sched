/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared helpers for XLSX export.

use umya_spreadsheet::structs::{Table, TableColumn, TableStyleInfo, Worksheet};

pub(super) fn set_str(ws: &mut Worksheet, col: u32, row: u32, value: &str) {
    ws.get_cell_mut((col, row)).set_value(value);
}

pub(super) fn set_opt(ws: &mut Worksheet, col: u32, row: u32, value: &Option<String>) {
    if let Some(v) = value {
        ws.get_cell_mut((col, row)).set_value(v.as_str());
    }
}

pub(super) fn set_headers(ws: &mut Worksheet, headers: &[&str]) {
    for (i, header) in headers.iter().enumerate() {
        ws.get_cell_mut((i as u32 + 1, 1)).set_value(*header);
    }
}

pub(super) fn add_table(ws: &mut Worksheet, name: &str, headers: &[&str], last_data_row: u32) {
    // Only add table if there's actual data (at least header + 1 data row)
    if last_data_row < 2 {
        return;
    }
    let num_cols = headers.len() as u32;
    let mut table = Table::new(name, ((1u32, 1u32), (num_cols, last_data_row)));
    table.set_display_name(name);
    // Excel requires unique column names; deduplicate by adding numeric suffixes
    let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for header in headers {
        let count = seen.entry(header.to_string()).or_insert(0);
        if *count > 0 {
            let unique_name = format!("{}{}", header, *count);
            table.add_column(TableColumn::new(&unique_name));
        } else {
            table.add_column(TableColumn::new(header));
        }
        *count += 1;
    }
    let style = TableStyleInfo::new("TableStyleMedium2", false, false, true, false);
    table.set_style_info(Some(style));
    ws.add_table(table);
}
