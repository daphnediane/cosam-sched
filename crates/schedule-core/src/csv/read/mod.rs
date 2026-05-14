/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CSV import: shared utilities and the top-level [`import_csv`] entry point.
//!
//! CSV import now forwards to the XLSX import functionality, which supports
//! both XLSX files and CSV directories. This module provides CSV format
//! detection utilities used by the XLSX import when importing CSV files.

mod schedule;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};

pub use crate::xlsx::read::{TableImportMode, TableImportOptions};
pub use schedule::import_csv;

// ── CSV format detection ─────────────────────────────────────────────────────

/// CSV format variants for import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CsvFormat {
    /// Standard UTF-8 comma-delimited CSV (default)
    #[default]
    Utf8Comma,
    /// UTF-8 tab-delimited (legacy format)
    Utf8Tab,
    /// UTF-16 tab-delimited with CRLF (legacy UnicodeText format)
    Utf16Tab,
}

/// Detect CSV format from file extension and content.
pub fn detect_csv_format(path: &Path) -> Result<CsvFormat> {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Check file extension first
    if filename.ends_with(".txt") {
        // Legacy UnicodeText format - try to detect UTF-16
        let mut file =
            File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
        let mut buffer = [0u8; 2];
        file.read_exact(&mut buffer)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        // Check for UTF-16 BOM (FE FF or FF FE)
        if (buffer[0] == 0xFE && buffer[1] == 0xFF) || (buffer[0] == 0xFF && buffer[1] == 0xFE) {
            return Ok(CsvFormat::Utf16Tab);
        }
        // Assume UTF-8 tab-delimited for .txt files without BOM
        return Ok(CsvFormat::Utf8Tab);
    }

    // Default to UTF-8 comma-delimited for .csv and other files
    Ok(CsvFormat::Utf8Comma)
}

// ── Import options ────────────────────────────────────────────────────────────

/// Type alias for CSV import options (uses common TableImportOptions).
pub type CsvImportOptions = TableImportOptions;

impl CsvImportOptions {
    /// Get the effective filename for a given table, or None if skipped.
    pub fn effective_filename(&self, table: &str) -> Option<String> {
        let mode = match table {
            "schedule" => &self.schedule,
            "rooms" => &self.rooms,
            "panel_types" => &self.panel_types,
            "people" => &self.people,
            "hotel_rooms" => &self.hotel_rooms,
            "timeline" => &self.timeline,
            _ => return None,
        };
        let name = mode.effective_name(table)?;
        Some(format!("{name}.csv"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    use std::fs::File;

    #[test]
    fn test_csv_format_detection_csv_file() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_csv_format.csv");
        let mut file = File::create(&temp_file).unwrap();
        writeln!(file, "name,value").unwrap();
        writeln!(file, "test,123").unwrap();

        let format = detect_csv_format(&temp_file).unwrap();
        assert_eq!(format, CsvFormat::Utf8Comma);

        fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_csv_format_detection_txt_file_no_bom() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_csv_format.txt");
        let mut file = File::create(&temp_file).unwrap();
        writeln!(file, "name\tvalue").unwrap();
        writeln!(file, "test\t123").unwrap();

        let format = detect_csv_format(&temp_file).unwrap();
        assert_eq!(format, CsvFormat::Utf8Tab);

        fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_csv_format_detection_txt_file_utf16le_bom() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_utf16le.txt");
        let mut file = File::create(&temp_file).unwrap();
        // UTF-16 LE BOM
        file.write_all(&[0xFF, 0xFE]).unwrap();
        // Write some UTF-16 LE encoded data
        file.write_all(&[0x74, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74, 0x00])
            .unwrap();

        let format = detect_csv_format(&temp_file).unwrap();
        assert_eq!(format, CsvFormat::Utf16Tab);

        fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_csv_format_detection_txt_file_utf16be_bom() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_utf16be.txt");
        let mut file = File::create(&temp_file).unwrap();
        // UTF-16 BE BOM
        file.write_all(&[0xFE, 0xFF]).unwrap();
        // Write some UTF-16 BE encoded data
        file.write_all(&[0x00, 0x74, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74])
            .unwrap();

        let format = detect_csv_format(&temp_file).unwrap();
        assert_eq!(format, CsvFormat::Utf16Tab);

        fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_csv_import_options_effective_filename() {
        let opts = CsvImportOptions::default();
        assert_eq!(
            opts.effective_filename("schedule"),
            Some("schedule.csv".to_string())
        );
        assert_eq!(
            opts.effective_filename("rooms"),
            Some("rooms.csv".to_string())
        );
        assert_eq!(
            opts.effective_filename("panel_types"),
            Some("panel_types.csv".to_string())
        );
        assert_eq!(
            opts.effective_filename("people"),
            Some("people.csv".to_string())
        );
        assert_eq!(
            opts.effective_filename("hotel_rooms"),
            Some("hotel_rooms.csv".to_string())
        );
        assert_eq!(
            opts.effective_filename("timeline"),
            Some("timeline.csv".to_string())
        );
        assert_eq!(opts.effective_filename("unknown"), None);
    }

    #[test]
    fn test_csv_import_options_skip() {
        let opts = CsvImportOptions {
            schedule: TableImportMode::Skip,
            ..Default::default()
        };

        assert_eq!(opts.effective_filename("schedule"), None);
        assert_eq!(
            opts.effective_filename("rooms"),
            Some("rooms.csv".to_string())
        );
    }

    #[test]
    fn test_csv_import_options_custom_filename() {
        let opts = CsvImportOptions {
            rooms: TableImportMode::ReadFrom("my_rooms".to_string()),
            ..Default::default()
        };

        assert_eq!(
            opts.effective_filename("rooms"),
            Some("my_rooms.csv".to_string())
        );
    }

    #[test]
    fn test_csv_import_options_from_xlsx() {
        use crate::xlsx::read::TableImportOptions;

        let opts = TableImportOptions {
            schedule: TableImportMode::Skip,
            rooms: TableImportMode::ReadFrom("custom_rooms".to_string()),
            ..Default::default()
        };

        assert_eq!(opts.schedule, TableImportMode::Skip);
        assert_eq!(
            opts.rooms,
            TableImportMode::ReadFrom("custom_rooms".to_string())
        );
        assert_eq!(opts.panel_types, TableImportMode::Process);
    }

    #[test]
    fn test_csv_format_default() {
        assert_eq!(CsvFormat::default(), CsvFormat::Utf8Comma);
    }
}
