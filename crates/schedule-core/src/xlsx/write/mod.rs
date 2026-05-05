/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! XLSX export (FEATURE-029).

mod common;
mod export;

use std::path::Path;

use anyhow::Result;

use crate::schedule::Schedule;

pub(super) fn export_xlsx(schedule: &Schedule, path: &Path) -> Result<()> {
    export::export_xlsx(schedule, path)
}
