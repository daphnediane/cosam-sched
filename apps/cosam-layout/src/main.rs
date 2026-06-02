/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `cosam-layout` — CLI for generating Typst/PDF print layouts from a
//! cosam schedule file (widget JSON or internal `.schedule` binary) and
//! brand config.

mod cli;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use schedule_layout::{
    brand::BrandConfig,
    color::ColorMode,
    formats,
    grid::{LayoutConfig, LayoutFilter, LayoutFormat, Orientation, PaperSize, SplitMode},
    model::ScheduleData,
};

use cli::{Args, ColorModeArg, FormatArg, LayoutJob, OrientationArg, PaperArg, SplitArg};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    if args.dump_sample_brand {
        print!("{}", include_str!("../../../config/brand.sample.toml"));
        return Ok(());
    }

    let jobs = cli::parse_layout_jobs(&args.layout_args)?;

    let data = load_schedule_data(&args.input, &args.brand_config)?;

    let brand = load_brand(&args.brand_config);

    let color_mode = match args.color_mode {
        ColorModeArg::Color => ColorMode::Color,
        ColorModeArg::Bw => ColorMode::Bw,
    };

    fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("creating output dir {:?}", args.output_dir))?;

    for job in &jobs {
        run_job(
            &data,
            &brand,
            color_mode,
            job,
            &args.output_dir,
            args.typ,
            args.no_compile,
        )?;
    }

    Ok(())
}

/// Load schedule data from either a widget JSON file or an internal `.schedule`
/// binary, detected by file extension.
fn load_schedule_data(input: &Path, brand_config: &Path) -> Result<ScheduleData> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "schedule" {
        let bytes =
            fs::read(input).with_context(|| format!("reading schedule file {:?}", input))?;
        let schedule = schedule_core::schedule::Schedule::load_from_file(&bytes)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("loading internal schedule {:?}", input))?;
        let title = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Schedule");
        ScheduleData::from_schedule(&schedule, title)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| "building layout data from schedule")
    } else {
        let _ = brand_config;
        let json =
            fs::read_to_string(input).with_context(|| format!("reading JSON file {:?}", input))?;
        ScheduleData::from_json(&json).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

/// Load brand config, warning and falling back to defaults if the file is
/// missing or invalid.
fn load_brand(path: &Path) -> BrandConfig {
    match BrandConfig::load(path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("warning: brand config {:?}: {e}; using defaults", path);
            BrandConfig::default()
        }
    }
}

fn run_job(
    data: &ScheduleData,
    brand: &BrandConfig,
    color_mode: ColorMode,
    job: &LayoutJob,
    output_dir: &Path,
    write_typ: bool,
    no_compile: bool,
) -> Result<()> {
    let config = LayoutConfig {
        paper: map_paper(job.paper),
        format: map_format(job.format),
        split_by: map_split(job.split),
        filter: build_filter(job),
        orientation: map_orientation(job.orientation),
        base_font_pt: None,
        grid_font_pt: None,
    };

    let outputs: Vec<(String, String)> = match job.format {
        FormatArg::Schedule => formats::schedule::generate(data, brand, &config, color_mode),
        FormatArg::WorkshopsListing => {
            formats::workshops_listing::generate(data, brand, &config, color_mode)
        }
        FormatArg::RoomSigns => formats::room_signs::generate(data, brand, &config, color_mode),
        FormatArg::GuestPostcards => {
            formats::guest_postcards::generate(data, brand, &config, color_mode)
        }
        FormatArg::Descriptions => {
            formats::descriptions::generate(data, brand, &config, color_mode)
        }
        FormatArg::Flyer => formats::flyer::generate(data, brand, &config, color_mode),
    };

    if outputs.is_empty() {
        eprintln!(
            "warning: format {:?} produced no output (stub or no matching panels)",
            job.format
        );
        return Ok(());
    }

    // Determine base stem: explicit --stem > output_override file stem > format+paper default.
    let base_stem: String = job
        .stem
        .clone()
        .or_else(|| {
            job.output_override
                .as_ref()
                .filter(|p| p.extension().is_some())
                .and_then(|p| p.file_stem())
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| default_stem(job.format));

    for (qualifier, typ_src) in &outputs {
        let file_stem = [
            base_stem.as_str(),
            config.paper.dir_name(),
            qualifier.as_str(),
        ]
        .iter()
        .copied()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

        let typ_path = output_dir.join(format!("{file_stem}.typ"));
        let pdf_path = job
            .output_override
            .as_ref()
            .filter(|p| p.extension().is_some() && outputs.len() == 1)
            .cloned()
            .unwrap_or_else(|| output_dir.join(format!("{file_stem}.pdf")));

        if write_typ || no_compile {
            fs::write(&typ_path, typ_src).with_context(|| format!("writing {:?}", typ_path))?;
            eprintln!("wrote {}", typ_path.display());
        }

        if !no_compile {
            compile_typst(typ_src, &typ_path, &pdf_path, brand)?;
        }
    }

    Ok(())
}

/// Invoke the Typst compiler to produce a PDF.
///
/// Writes `typ_src` to `typ_path` (creating parent directories as needed),
/// then calls `typst compile` to produce `pdf_path`.
fn compile_typst(
    typ_src: &str,
    typ_path: &Path,
    pdf_path: &Path,
    brand: &BrandConfig,
) -> Result<()> {
    use std::process::Command;

    if let Some(parent) = typ_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {:?}", parent))?;
    }
    fs::write(typ_path, typ_src).with_context(|| format!("writing {:?}", typ_path))?;

    let font_args: Vec<String> = brand
        .fonts
        .font_dir
        .as_ref()
        .and_then(|d| d.to_str())
        .map(|d| vec!["--font-path".to_string(), d.to_string()])
        .unwrap_or_default();

    let status = Command::new("typst")
        .arg("compile")
        .arg("--root")
        .arg("/")
        .args(&font_args)
        .arg(typ_path)
        .arg(pdf_path)
        .status()
        .with_context(|| "invoking `typst compile` (is typst installed?)")?;

    if !status.success() {
        anyhow::bail!("typst compile failed for {:?}", typ_path);
    }

    eprintln!("compiled {}", pdf_path.display());
    Ok(())
}

fn map_paper(p: PaperArg) -> PaperSize {
    match p {
        PaperArg::Letter => PaperSize::Letter,
        PaperArg::Legal => PaperSize::Legal,
        PaperArg::Tabloid => PaperSize::Tabloid,
        PaperArg::SuperB => PaperSize::SuperB,
        PaperArg::Poster => PaperSize::Poster,
        PaperArg::Postcard4x6 => PaperSize::Postcard4x6,
    }
}

fn map_format(f: FormatArg) -> LayoutFormat {
    match f {
        FormatArg::Schedule => LayoutFormat::Schedule,
        FormatArg::WorkshopsListing => LayoutFormat::WorkshopsListing,
        FormatArg::RoomSigns => LayoutFormat::RoomSigns,
        FormatArg::GuestPostcards => LayoutFormat::GuestPostcards,
        FormatArg::Descriptions => LayoutFormat::Descriptions,
        FormatArg::Flyer => LayoutFormat::Flyer,
    }
}

fn map_split(s: SplitArg) -> SplitMode {
    match s {
        SplitArg::Day => SplitMode::Day,
        SplitArg::HalfDay => SplitMode::HalfDay,
    }
}

fn map_orientation(o: OrientationArg) -> Orientation {
    match o {
        OrientationArg::Landscape => Orientation::Landscape,
        OrientationArg::Portrait => Orientation::Portrait,
    }
}

/// Derive a default filename stem from format when the caller provides none.
/// Paper size and split qualifier are appended automatically during filename assembly.
fn default_stem(format: FormatArg) -> String {
    match format {
        FormatArg::Schedule => "schedule",
        FormatArg::WorkshopsListing => "workshops",
        FormatArg::RoomSigns => "room-signs",
        FormatArg::GuestPostcards => "postcards",
        FormatArg::Descriptions => "desc",
        FormatArg::Flyer => "flyer",
    }
    .to_string()
}

fn build_filter(job: &LayoutJob) -> LayoutFilter {
    LayoutFilter {
        premium_only: job.filter_premium,
        room_uid: job.filter_room.map(i64::from),
        guest_name: job.filter_guest.clone(),
    }
}
