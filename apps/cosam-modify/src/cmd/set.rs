/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `set` command — update a field on selected entities. (CLI-093)

use anyhow::Result;
use schedule_core::edit::context::EditContext;

use crate::args::Stage;

pub fn run(_ctx: &mut EditContext, _stage: &Stage, _field: &str, _value: &str) -> Result<()> {
    // Implemented in CLI-093
    Ok(())
}
