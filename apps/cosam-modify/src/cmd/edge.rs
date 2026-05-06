/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `add-edge` and `remove-edge` commands. (CLI-096)

use anyhow::Result;
use schedule_core::edit::context::EditContext;

use crate::args::Stage;

pub fn run_add(
    _ctx: &mut EditContext,
    _stage: &Stage,
    _edge_field: &str,
    _value: &str,
) -> Result<()> {
    // Implemented in CLI-096
    Ok(())
}

pub fn run_remove(
    _ctx: &mut EditContext,
    _stage: &Stage,
    _edge_field: &str,
    _value: &str,
) -> Result<()> {
    // Implemented in CLI-096
    Ok(())
}
