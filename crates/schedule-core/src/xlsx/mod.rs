/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

pub mod columns;
pub mod read;
pub mod write;

pub use read::{XlsxImportOptions, canonical_header, import_xlsx};
pub use write::{export_to_xlsx, post_save_cleanup, update_xlsx};

use std::path::Path;

use anyhow::Result;

use crate::file::ScheduleFile;

/// Load a schedule from either `.xlsx` or `.json` based on file extension.
pub fn load_auto(path: &Path, options: &read::XlsxImportOptions) -> Result<ScheduleFile> {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("xlsx") => read::import_xlsx(path, options),
        Some(ext) if ext.eq_ignore_ascii_case("json") => ScheduleFile::load(path),
        Some(ext) => anyhow::bail!("Unsupported file format: .{ext}"),
        None => ScheduleFile::load(path),
    }
}

/// Save a schedule to either `.xlsx` or `.json` based on file extension.
pub fn save_auto(sf: &mut ScheduleFile, path: &Path) -> Result<()> {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("xlsx") => write::update_xlsx(sf, path),
        _ => sf.save_json(path),
    }
}
