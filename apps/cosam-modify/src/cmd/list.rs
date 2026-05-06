/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `list` command — display entities and their fields. (CLI-092)

use anyhow::Result;
use schedule_core::edit::context::EditContext;

use crate::args::{OutputFormat, Stage};

pub fn run(_ctx: &mut EditContext, _stage: &Stage, _format: &OutputFormat) -> Result<()> {
    // Implemented in CLI-092
    Ok(())
}
