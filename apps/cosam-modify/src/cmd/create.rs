/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `create` command — add a new entity. (CLI-094)

use anyhow::Result;
use schedule_core::edit::context::EditContext;

use crate::args::Stage;

pub fn run(_ctx: &mut EditContext, _stage: &Stage, _fields: &[(String, String)]) -> Result<()> {
    // Implemented in CLI-094
    Ok(())
}
