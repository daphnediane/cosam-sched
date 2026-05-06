/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `delete` command — soft-delete an entity. (CLI-095)

use anyhow::Result;
use schedule_core::edit::context::EditContext;

use crate::args::Stage;

pub fn run(_ctx: &mut EditContext, _stage: &Stage, _query: &str) -> Result<()> {
    // Implemented in CLI-095
    Ok(())
}
