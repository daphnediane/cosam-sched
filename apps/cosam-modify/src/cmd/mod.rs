/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Command dispatch for `cosam-modify` stages.

pub mod create;
pub mod delete;
pub mod edge;
pub mod get;
pub mod list;
pub mod set;
pub mod undo;

use anyhow::Result;
use schedule_core::edit::context::EditContext;

use crate::args::{OutputFormat, Stage, StageCommand};

/// Execute a single stage (selector + command) against `ctx`.
pub fn run_stage(ctx: &mut EditContext, stage: &Stage, format: &OutputFormat) -> Result<()> {
    match &stage.command {
        StageCommand::List => list::run(ctx, stage, format),
        StageCommand::Get { query } => get::run(ctx, stage, query, format),
        StageCommand::Set { field, value } => set::run(ctx, stage, field, value),
        StageCommand::Create { fields } => create::run(ctx, stage, fields),
        StageCommand::Delete { query } => delete::run(ctx, stage, query),
        StageCommand::AddEdge { edge_field, value } => edge::run_add(ctx, stage, edge_field, value),
        StageCommand::RemoveEdge { edge_field, value } => {
            edge::run_remove(ctx, stage, edge_field, value)
        }
        StageCommand::Undo => undo::run_undo(ctx),
        StageCommand::Redo => undo::run_redo(ctx),
        StageCommand::ShowHistory => {
            undo::run_show_history(ctx, format);
            Ok(())
        }
    }
}
