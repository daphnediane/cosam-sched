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
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = parse_args()?;

    let schedule = load_schedule(&cli.file, cli.create_new)?;
    let mut ctx = EditContext::new(schedule);

    for stage in &cli.stages {
        cmd::run_stage(&mut ctx, stage, &cli.format)?;
    }

    if ctx.is_dirty() {
        ctx.schedule_mut().metadata.generator =
            concat!("cosam-modify ", env!("CARGO_PKG_VERSION")).to_string();
        save_schedule(ctx.schedule_mut(), &cli.file)?;
    }

    Ok(())
}
