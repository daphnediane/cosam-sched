/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `undo`, `redo`, and `show-history` commands. (CLI-097)

use anyhow::Result;
use schedule_core::edit::context::EditContext;

use crate::args::OutputFormat;

pub fn run_undo(_ctx: &mut EditContext) -> Result<()> {
    // Implemented in CLI-097
    Ok(())
}

pub fn run_redo(_ctx: &mut EditContext) -> Result<()> {
    // Implemented in CLI-097
    Ok(())
}

pub fn run_show_history(_ctx: &EditContext, _format: &OutputFormat) {
    // Implemented in CLI-097
}
