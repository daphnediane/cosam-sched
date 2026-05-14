/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CSV import for schedule data.

use std::path::Path;

use anyhow::Result;

use crate::schedule::Schedule;

use super::CsvImportOptions;

/// Import CSV files from a directory and return a populated [`Schedule`].
///
/// This function now forwards to the XLSX import functionality, which supports
/// both XLSX files and CSV directories. When given a directory path, it scans
/// for CSV/TXT files and imports them as if they were sheets in an XLSX file.
///
/// Read order:
/// 1. PanelTypes — so panel-type lookups work during schedule import.
/// 2. Hotels — creates HotelRoom entities with richer metadata (optional).
/// 3. Rooms — so room lookups work during schedule import.
///    Links event rooms to hotel rooms from Hotels file or inline Hotel Room column.
/// 4. People — establishes presenter rank/flags before the Schedule file
///    creates presenter entities from column headers.
/// 5. Timeline — creates Timeline entities separately from panels (optional).
/// 6. Schedule — panels, timing, rooms, panel type, and presenter edges.
///    Timeline rows (is_timeline panel type) are skipped if Timeline file was processed.
///
/// The returned `Schedule` is a clean slate — all entities and edges are
/// freshly created. No existing CRDT state is preserved or merged.
pub fn import_csv(dir_path: &Path, options: &CsvImportOptions) -> Result<Schedule> {
    if !dir_path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", dir_path.display());
    }

    // Forward to XLSX import, which now handles CSV directories
    crate::xlsx::read::import_xlsx(dir_path, options)
}
