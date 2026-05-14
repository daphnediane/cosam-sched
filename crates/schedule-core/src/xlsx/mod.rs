/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! XLSX spreadsheet import and export support.
//!
//! # Import (FEATURE-028)
//!
//! [`import_xlsx`] reads the standard Cosplay America XLSX spreadsheet format
//! (Schedule, Rooms, and PanelTypes sheets) into a fresh [`Schedule`].
//!
//! The import always starts from a clean slate — no existing CRDT state is
//! preserved or merged.  See IDEA-079 for future merge-import support.
//!
//! # Export (FEATURE-029)
//!
//! [`export_xlsx`] writes a [`Schedule`] back to the same XLSX format,
//! enabling round-trip with the import.

pub mod columns;
pub mod read;
pub mod write;

use std::path::Path;

use anyhow::Result;

use crate::schedule::Schedule;

pub use columns::FieldDef;
pub use read::{
    canonical_header, import_xlsx, TableImportMode, TableImportOptions, XlsxImportOptions,
};

/// Export a schedule to an XLSX file.
///
/// # Errors
///
/// Returns an error if the file cannot be created or written.
pub fn export_xlsx(schedule: &Schedule, path: &Path) -> Result<()> {
    write::export_xlsx(schedule, path)
}
