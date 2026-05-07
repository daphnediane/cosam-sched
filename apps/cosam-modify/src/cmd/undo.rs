/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `undo`, `redo`, and `show-history` commands. (CLI-097)

use anyhow::Result;
use schedule_core::edit::context::EditContext;

use crate::args::OutputFormat;

pub fn run_undo(ctx: &mut EditContext) -> Result<()> {
    ctx.undo().map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn run_redo(ctx: &mut EditContext) -> Result<()> {
    ctx.redo().map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn run_show_history(ctx: &EditContext, format: &OutputFormat) {
    let undo = ctx.undo_depth();
    let redo = ctx.redo_depth();
    match format {
        OutputFormat::Text => {
            println!("undo: {undo}");
            println!("redo: {redo}");
        }
        OutputFormat::Json => {
            println!(r#"{{"undo": {undo}, "redo": {redo}}}"#);
        }
        OutputFormat::Toml => {
            println!("undo = {undo}");
            println!("redo = {redo}");
        }
    }
}
