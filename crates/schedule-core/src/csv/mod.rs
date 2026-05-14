/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CSV file import and export support.
//!
//! # Import
//!
//! [`import_csv`] reads CSV files from a directory into a fresh [`Schedule`].
//! Each CSV file corresponds to an entity type (schedule.csv, timeline.csv, etc.).
//!
//! The import always starts from a clean slate — no existing CRDT state is
//! preserved or merged.
//!
//! # Export
//!
//! [`export_csv`] writes a [`Schedule`] to CSV files in a directory,
//! enabling round-trip with the import.

pub mod read;
pub mod write;

use std::path::Path;

use anyhow::Result;

use crate::schedule::Schedule;

pub use read::{import_csv, CsvImportOptions};

/// Export a schedule to CSV files in a directory.
///
/// Creates a directory (if it doesn't exist) and writes one CSV file per
/// entity type: schedule.csv, timeline.csv, rooms.csv, hotel.csv, panel_types.csv, people.csv.
///
/// # Errors
///
/// Returns an error if the directory cannot be created or files cannot be written.
pub fn export_csv(schedule: &Schedule, dir_path: &Path) -> Result<()> {
    write::export_csv(schedule, dir_path)
}
