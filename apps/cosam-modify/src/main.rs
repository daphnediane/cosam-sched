/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

mod args;
mod cmd;
mod load;
mod output;

use args::parse_args;
use load::{load_schedule, save_schedule};
use schedule_core::edit::context::EditContext;

fn main() {
    std::process::exit(run_main());
}

/// Returns exit code: 0 = success, 1 = user error, 2 = I/O error.
fn run_main() -> i32 {
    let cli = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e:#}");
            return 1;
        }
    };

    let schedule = match load_schedule(&cli.file, cli.create_new) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e:#}");
            return 2;
        }
    };

    let mut ctx = EditContext::new(schedule);

    for stage in &cli.stages {
        if let Err(e) = cmd::run_stage(&mut ctx, stage, &cli.format) {
            eprintln!("error: {e:#}");
            return 1;
        }
    }

    if ctx.is_dirty() {
        ctx.schedule_mut().metadata.generator =
            concat!("cosam-modify ", env!("CARGO_PKG_VERSION")).to_string();
        if let Err(e) = save_schedule(ctx.schedule_mut(), &cli.file) {
            eprintln!("error: {e:#}");
            return 2;
        }
    }

    0
}
