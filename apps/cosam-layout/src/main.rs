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
    config::{
        ContentMode, FooterMode, LayoutConfig, Orientation, PanelFilter, PaperSize, SectionSplit,
        TimeSplit,
    },
    document,
    model::ScheduleData,
};

use cli::{
    Args, ColorModeArg, ContentArg, FooterArg, LayoutJob, OrientationArg, PanelFilterArg, PaperArg,
    SplitArg,
};

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
        content: build_content(job.content, job.split),
        panel_filter: map_panel_filter(job.panel_filter),
        orientation: map_orientation(job.orientation),
        color_mode,
        columns: job.columns,
        footer: map_footer(job.footer),
        double_sided: job.double_sided,
        header_text: job.header_text.clone(),
        base_font_pt: None,
        grid_font_pt: None,
    };

    let outputs: Vec<(String, String)> = document::generate(data, brand, &config);

    if outputs.is_empty() {
        eprintln!(
            "warning: content {:?} produced no output (no matching panels)",
            job.content
        );
        return Ok(());
    }

    // Determine base stem: explicit --stem > output_override file stem > content+paper default.
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
        .unwrap_or_else(|| default_stem(job.content));

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

fn map_section_split(s: SplitArg) -> Option<SectionSplit> {
    match s {
        SplitArg::Room | SplitArg::RoomDay => Some(SectionSplit::Room),
        SplitArg::Presenter | SplitArg::PresenterDay => Some(SectionSplit::Presenter),
        _ => None,
    }
}

fn map_time_split_required(s: SplitArg) -> TimeSplit {
    match s {
        SplitArg::HalfDay => TimeSplit::HalfDay,
        _ => TimeSplit::Day,
    }
}

fn map_time_split_optional(s: SplitArg) -> Option<TimeSplit> {
    match s {
        SplitArg::None => None,
        SplitArg::HalfDay => Some(TimeSplit::HalfDay),
        _ => Some(TimeSplit::Day),
    }
}

/// Combine the content choice with its split into a `ContentMode`.
fn build_content(c: ContentArg, split: SplitArg) -> ContentMode {
    let section = map_section_split(split);
    match c {
        ContentArg::Both => ContentMode::Both {
            section,
            time: map_time_split_required(split),
        },
        ContentArg::GridOnly => ContentMode::GridOnly {
            section,
            time: map_time_split_required(split),
        },
        ContentArg::DescriptionOnly => ContentMode::DescriptionOnly {
            section,
            time: map_time_split_optional(split),
        },
        ContentArg::PanelList => ContentMode::PanelList {
            section,
            time: map_time_split_optional(split),
        },
    }
}

fn map_panel_filter(f: PanelFilterArg) -> PanelFilter {
    match f {
        PanelFilterArg::All => PanelFilter::All,
        PanelFilterArg::Workshops => PanelFilter::Workshops,
        PanelFilterArg::Premium => PanelFilter::Premium,
    }
}

fn map_footer(f: FooterArg) -> FooterMode {
    match f {
        FooterArg::Full => FooterMode::Full,
        FooterArg::TimestampOnly => FooterMode::TimestampOnly,
        FooterArg::None => FooterMode::None,
    }
}

fn map_orientation(o: OrientationArg) -> Orientation {
    match o {
        OrientationArg::Landscape => Orientation::Landscape,
        OrientationArg::Portrait => Orientation::Portrait,
    }
}

/// Derive a default filename stem from content when the caller provides none.
/// Paper size is appended automatically during filename assembly.
fn default_stem(content: ContentArg) -> String {
    match content {
        ContentArg::Both => "flyer",
        ContentArg::GridOnly => "schedule",
        ContentArg::DescriptionOnly => "desc",
        ContentArg::PanelList => "list",
    }
    .to_string()
}
