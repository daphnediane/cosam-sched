/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule file load and save helpers.

use std::path::Path;

use anyhow::{Context, Result};
use schedule_core::edit::context::EditContext;
use schedule_core::schedule::{Schedule, FILE_MAGIC};
use schedule_core::xlsx::{import_xlsx, update_schedule_from_xlsx, XlsxImportOptions};

/// Load a schedule from `path`, or create a new one.
///
/// - If `create_new` is `true` or the file does not exist: returns `Schedule::new()`.
/// - If the file starts with `COSAM\x00`: decoded as native binary.
/// - Otherwise: imported as xlsx using default table names.
pub fn load_schedule(path: &Path, create_new: bool) -> Result<Schedule> {
    if create_new || !path.exists() {
        return Ok(Schedule::new());
    }

    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;

    if bytes.starts_with(FILE_MAGIC) {
        Schedule::load_from_file(&bytes)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("failed to load schedule from {}", path.display()))
    } else {
        let opts = XlsxImportOptions::default();
        import_xlsx(path, &opts)
            .with_context(|| format!("failed to import xlsx from {}", path.display()))
    }
}

/// Merge an XLSX spreadsheet into the schedule held by `ctx` as a single
/// undoable checkpoint.
///
/// Uses upsert semantics: entities present in the XLSX are created or updated,
/// entities previously imported but absent from the XLSX are soft-deleted.
/// If the XLSX produces no CRDT changes, no undo entry is pushed and the
/// dirty counter is not incremented.
pub fn merge_xlsx_into(ctx: &mut EditContext, xlsx_path: &Path) -> Result<()> {
    let label = format!(
        "Import {}",
        xlsx_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("xlsx")
    );
    let opts = XlsxImportOptions::default();
    ctx.run_checkpoint(label, |schedule| {
        update_schedule_from_xlsx(schedule, xlsx_path, &opts)
            .with_context(|| format!("failed to merge xlsx from {}", xlsx_path.display()))
    })
}

/// Save `schedule` to `path` as a native binary file.
pub fn save_schedule(schedule: &mut Schedule, path: &Path) -> Result<()> {
    let bytes = schedule.save_to_file();
    std::fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))
}
