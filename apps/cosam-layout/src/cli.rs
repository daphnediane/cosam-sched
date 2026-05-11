/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CLI argument definitions for `cosam-layout`.

use clap::{ArgAction, Parser, ValueEnum};
use std::path::PathBuf;

/// Generate Typst/PDF print layouts from a cosam schedule JSON.
#[derive(Debug, Parser)]
#[command(name = "cosam-layout", version, about)]
pub struct Args {
    /// Input schedule.json
    #[arg(short, long, value_name = "FILE")]
    pub input: PathBuf,

    /// Output directory
    #[arg(long, value_name = "DIR", default_value = "output/layout")]
    pub output_dir: PathBuf,

    /// Brand config TOML (missing file warns and falls back to defaults)
    #[arg(long, value_name = "FILE", default_value = "config/brand.toml")]
    pub brand_config: PathBuf,

    /// Also write .typ source files alongside PDFs
    #[arg(long, action = ArgAction::SetTrue)]
    pub typ: bool,

    /// Write .typ only; skip PDF compilation
    #[arg(long, action = ArgAction::SetTrue)]
    pub no_compile: bool,

    /// Print sample brand.toml to stdout and exit
    #[arg(long, action = ArgAction::SetTrue)]
    pub dump_sample_brand: bool,

    /// Color mode
    #[arg(long, value_enum, default_value_t = ColorModeArg::Color)]
    pub color_mode: ColorModeArg,

    /// Per-layout job specs (repeatable; separate multiple jobs with --)
    #[arg(last = true)]
    pub layout_args: Vec<String>,
}

/// Color mode option for clap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorModeArg {
    Color,
    Bw,
}

/// A single resolved layout job derived from `--layout_args`.
#[derive(Debug, Clone)]
pub struct LayoutJob {
    pub format: FormatArg,
    pub paper: PaperArg,
    pub split: SplitArg,
    pub filter_premium: bool,
    pub filter_room: Option<u32>,
    pub filter_guest: Option<String>,
    pub output_override: Option<PathBuf>,
}

impl Default for LayoutJob {
    fn default() -> Self {
        Self {
            format: FormatArg::Schedule,
            paper: PaperArg::Tabloid,
            split: SplitArg::Day,
            filter_premium: false,
            filter_room: None,
            filter_guest: None,
            output_override: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FormatArg {
    Schedule,
    WorkshopPoster,
    RoomSigns,
    GuestPostcards,
    Descriptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PaperArg {
    Legal,
    Tabloid,
    SuperB,
    Postcard4x6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SplitArg {
    Day,
    HalfDay,
}

/// Parse the trailing `layout_args` into a list of `LayoutJob`s.
///
/// Jobs are delimited by bare `--` tokens. Each job resets to defaults before
/// applying its flags.
pub fn parse_layout_jobs(raw: &[String]) -> anyhow::Result<Vec<LayoutJob>> {
    let mut jobs: Vec<LayoutJob> = Vec::new();
    let mut current = LayoutJob::default();
    let mut has_content = false;

    let mut iter = raw.iter().peekable();
    while let Some(token) = iter.next() {
        if token == "--" {
            if has_content {
                jobs.push(current.clone());
            }
            current = LayoutJob::default();
            has_content = false;
            continue;
        }
        has_content = true;
        match token.as_str() {
            "--format" => {
                let val = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--format requires a value"))?;
                current.format = match val.as_str() {
                    "schedule" => FormatArg::Schedule,
                    "workshop-poster" => FormatArg::WorkshopPoster,
                    "room-signs" => FormatArg::RoomSigns,
                    "guest-postcards" => FormatArg::GuestPostcards,
                    "descriptions" => FormatArg::Descriptions,
                    other => anyhow::bail!("unknown --format value: {}", other),
                };
            }
            "--paper" => {
                let val = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--paper requires a value"))?;
                current.paper = match val.as_str() {
                    "legal" => PaperArg::Legal,
                    "tabloid" => PaperArg::Tabloid,
                    "super-b" => PaperArg::SuperB,
                    "postcard-4x6" => PaperArg::Postcard4x6,
                    other => anyhow::bail!("unknown --paper value: {}", other),
                };
            }
            "--split" => {
                let val = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--split requires a value"))?;
                current.split = match val.as_str() {
                    "day" => SplitArg::Day,
                    "half-day" => SplitArg::HalfDay,
                    other => anyhow::bail!("unknown --split value: {}", other),
                };
            }
            "--filter-premium" => {
                current.filter_premium = true;
            }
            "--filter-room" => {
                let val = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--filter-room requires a value"))?;
                current.filter_room =
                    Some(val.parse::<u32>().map_err(|_| {
                        anyhow::anyhow!("--filter-room must be a room UID integer")
                    })?);
            }
            "--filter-guest" => {
                let val = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--filter-guest requires a value"))?;
                current.filter_guest = Some(val.clone());
            }
            "--output" => {
                let val = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--output requires a value"))?;
                current.output_override = Some(PathBuf::from(val));
            }
            other => anyhow::bail!("unknown layout arg: {}", other),
        }
    }

    if has_content {
        jobs.push(current);
    }

    // Default: single schedule job if nothing was specified
    if jobs.is_empty() {
        jobs.push(LayoutJob::default());
    }

    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_layout_jobs_defaults() {
        let jobs = parse_layout_jobs(&[]).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].format, FormatArg::Schedule);
        assert_eq!(jobs[0].paper, PaperArg::Tabloid);
        assert_eq!(jobs[0].split, SplitArg::Day);
    }

    #[test]
    fn test_parse_layout_jobs_single() {
        let args: Vec<String> = vec![
            "--format",
            "room-signs",
            "--paper",
            "super-b",
            "--split",
            "day",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let jobs = parse_layout_jobs(&args).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].format, FormatArg::RoomSigns);
        assert_eq!(jobs[0].paper, PaperArg::SuperB);
    }

    #[test]
    fn test_parse_layout_jobs_two_jobs() {
        let args: Vec<String> = vec!["--format", "schedule", "--", "--format", "descriptions"]
            .into_iter()
            .map(String::from)
            .collect();
        let jobs = parse_layout_jobs(&args).unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].format, FormatArg::Schedule);
        assert_eq!(jobs[1].format, FormatArg::Descriptions);
    }

    #[test]
    fn test_parse_layout_jobs_filter_room() {
        let args: Vec<String> = vec!["--format", "room-signs", "--filter-room", "42"]
            .into_iter()
            .map(String::from)
            .collect();
        let jobs = parse_layout_jobs(&args).unwrap();
        assert_eq!(jobs[0].filter_room, Some(42));
    }
}
