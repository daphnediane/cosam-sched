/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CSV export implementation.
//!
//! Reuses the XLSX export logic by building the spreadsheet and then
//! exporting each sheet to CSV using umya-spreadsheet's CSV writer.

use std::fs;
use std::path::Path;

use anyhow::Result;
use umya_spreadsheet::structs::CsvWriterOption;

use crate::schedule::Schedule;
use crate::xlsx::write::build_spreadsheet;

/// Export schedule to CSV files by reusing XLSX export logic.
///
/// This function:
/// 1. Builds the XLSX spreadsheet using existing export logic
/// 2. Exports each sheet to a separate CSV file
/// 3. Handles encoding and formatting via umya-spreadsheet's CSV writer
pub fn export_csv(schedule: &Schedule, dir_path: &Path) -> Result<()> {
    // Create directory if it doesn't exist
    fs::create_dir_all(dir_path)?;

    // Build the XLSX spreadsheet (in-memory)
    let mut book = build_spreadsheet(schedule)?;

    // Export each sheet to CSV
    for sheet_index in 0..book.get_sheet_count() {
        let sheet = book
            .get_sheet(&sheet_index)
            .ok_or_else(|| anyhow::anyhow!("Cannot get sheet at index {}", sheet_index))?;

        let sheet_name = sheet.get_name();

        // Normalize sheet name for CSV filename
        let csv_name = format!("{}.csv", sheet_name.to_lowercase().replace(" ", "_"));
        let csv_path = dir_path.join(&csv_name);

        // Set as active sheet
        book.set_active_sheet(sheet_index as u32);

        // Export to CSV
        let mut option = CsvWriterOption::default();
        option.set_csv_encode_value(umya_spreadsheet::structs::CsvEncodeValues::Utf8);

        umya_spreadsheet::writer::csv::write(&book, &csv_path, Some(&option))
            .map_err(|e| anyhow::anyhow!("Failed to write CSV {}: {}", csv_path.display(), e))?;
    }

    Ok(())
}
