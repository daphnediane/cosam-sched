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
    let num_cols = headers.len() as u32;
    let last_row = last_data_row.max(2);
    let mut table = Table::new(name, ((1u32, 1u32), (num_cols, last_row)));
    table.set_display_name(name);
    for header in headers {
        table.add_column(TableColumn::new(header));
    }
    let style = TableStyleInfo::new("TableStyleMedium2", false, false, true, false);
    table.set_style_info(Some(style));
    ws.add_table(table);
}
