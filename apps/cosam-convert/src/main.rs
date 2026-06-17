/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use schedule_core::csv::{export_csv, import_csv};
use schedule_core::schedule::Schedule;
use schedule_core::tables::event_room::EventRoomEntityType;
use schedule_core::tables::panel::PanelEntityType;
use schedule_core::tables::panel_type::PanelTypeEntityType;
use schedule_core::tables::presenter::PresenterEntityType;
use schedule_core::widget_json::{
    export_to_widget_json, import_from_widget_json, load_from_file, load_from_url, ScheduleConfig,
    WidgetExport, WidgetJsonError,
};
use schedule_core::xlsx::{
    export_xlsx, export_xlsx_grid, import_xlsx, TableImportMode, TableImportOptions,
};

#[cfg(feature = "layout")]
mod brand_bridge;
mod conflicts;
mod embed;
mod layout_config;
mod static_html;
#[cfg(feature = "layout")]
mod widget_config;

// ── Input type tracking ───────────────────────────────────────────────────────

#[derive(Debug)]
enum InputType {
    Schedule(Box<Schedule>),
    WidgetJson(WidgetExport),
    ScheduleFromWidget(Box<Schedule>, WidgetExport),
}

impl InputType {
    fn as_schedule(&mut self) -> Result<&mut Schedule> {
        match self {
            InputType::Schedule(ref mut sched) => Ok(sched),
            InputType::ScheduleFromWidget(ref mut sched, _) => Ok(sched),
            InputType::WidgetJson(ref widget) => {
                eprintln!(
                    "Converting widget JSON to Schedule (data loss may occur - see documentation)"
                );
                let sched = import_from_widget_json(widget).map_err(|e| {
                    anyhow::anyhow!("Failed to convert widget JSON to Schedule: {}", e)
                })?;
                let widget_clone = widget.clone();
                *self = InputType::ScheduleFromWidget(Box::new(sched), widget_clone);
                match self {
                    InputType::ScheduleFromWidget(ref mut sched, _) => Ok(sched),
                    _ => unreachable!(),
                }
            }
        }
    }

    /// Get widget JSON, converting from Schedule if needed.
    /// Returns error if conversion fails.
    fn as_widget(
        &mut self,
        title: &str,
        private_export: bool,
    ) -> Result<WidgetExport, WidgetJsonError> {
        match self {
            InputType::WidgetJson(ref widget) => {
                // Clone and update title
                let mut widget_clone = widget.clone();
                widget_clone.meta.title = title.to_string();
                Ok(widget_clone)
            }
            InputType::ScheduleFromWidget(_, ref widget) => {
                // Clone and update title
                let mut widget_clone = widget.clone();
                widget_clone.meta.title = title.to_string();
                Ok(widget_clone)
            }
            InputType::Schedule(ref sched) => {
                // Convert from Schedule
                export_to_widget_json(sched, title, private_export)
            }
        }
    }
}

/// Detect if a file is widget JSON format by checking extension and content
fn is_widget_json_file(path: &Path) -> bool {
    // Check extension first
    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        return true;
    }

    // Try to read and parse as widget JSON
    if let Ok(content) = std::fs::read_to_string(path) {
        // Quick heuristic: widget JSON has "meta" field with "title"
        if content.contains("\"meta\"") && content.contains("\"title\"") {
            return true;
        }
    }

    false
}

// ── Output settings ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct OutputSettings {
    widget_css: Option<String>,
    widget_js: Option<String>,
    test_template: Option<String>,
    minified: bool,
    style_page: Option<bool>,
    title: String,
    private_export: bool,
    embed_as_html: bool,
    #[cfg(feature = "layout")]
    brand_config: Option<PathBuf>,
    #[cfg(feature = "layout")]
    layout_config: Option<PathBuf>,
    /// A single layout job assembled from `--layout.<key>=<value>` flags. When
    /// set, `--export-layout` renders just this one job (using the export path as
    /// the output file name) instead of the jobs from the layout TOML. Cleared by
    /// `--default`, `--default-layouts`, and `--layout-config`.
    #[cfg(feature = "layout")]
    layout: Option<layout_config::JobConfig>,
    /// Test affordance: use the schedule's modified time as the generated time so
    /// layout output (the page footer) is reproducible across runs.
    #[cfg(feature = "layout")]
    stable_timestamps: bool,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            widget_css: None,
            widget_js: None,
            test_template: None,
            minified: true,
            style_page: None,
            title: String::new(),
            private_export: false,
            embed_as_html: true,
            #[cfg(feature = "layout")]
            brand_config: None,
            #[cfg(feature = "layout")]
            layout_config: None,
            #[cfg(feature = "layout")]
            layout: None,
            #[cfg(feature = "layout")]
            stable_timestamps: false,
        }
    }
}

// ── Output job ────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct OutputJob {
    path: PathBuf,
    settings: OutputSettings,
    job_type: OutputType,
}

#[derive(Debug)]
enum OutputType {
    Output,
    Export,
    ExportEmbed,
    ExportEmbedHead,
    ExportEmbedBody,
    ExportTest,
    ExportCsv,
    ExportXlsxGrid,
    #[cfg(feature = "layout")]
    ExportLayout,
}

// ── CLI args ──────────────────────────────────────────────────────────────────

#[derive(Default)]
struct CliArgs {
    input: Option<PathBuf>,
    input_url: Option<String>,
    output_jobs: Vec<OutputJob>,
    check_only: bool,
    table_options: TableImportOptions,
    /// Default IANA timezone, used only when the source supplies no timezone.
    default_timezone: Option<String>,
    /// Default schedule-window start, used only when the source supplies none.
    default_start_time: Option<chrono::NaiveDateTime>,
    /// Default schedule-window end, used only when the source supplies none.
    default_end_time: Option<chrono::NaiveDateTime>,
}

// ── Argument parsing ──────────────────────────────────────────────────────────

fn parse_table_mode(arg: &str) -> Result<TableImportMode> {
    if arg.is_empty() {
        anyhow::bail!("Sheet mode argument cannot be empty");
    }
    let arg_lower = arg.to_lowercase();
    match arg_lower.as_str() {
        "default" => Ok(TableImportMode::Process),
        "ignore" | "skip" => Ok(TableImportMode::Skip),
        _ => Ok(TableImportMode::ReadFrom(arg.to_string())),
    }
}

fn check_duplicate_output(output_jobs: &[OutputJob], path: &PathBuf) -> Result<()> {
    if output_jobs.iter().any(|job| job.path == *path) {
        anyhow::bail!("Output file specified multiple times: {}", path.display());
    }
    Ok(())
}

fn parse_args() -> Result<CliArgs> {
    let arguments: Vec<String> = std::env::args().collect();
    let mut args = CliArgs::default();

    let mut input: Option<PathBuf> = None;
    let mut input_url: Option<String> = None;
    let mut current_settings = OutputSettings::default();
    // Index of the first setting not yet consumed by an output command.
    let mut first_setting_index: Option<usize> = None;

    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--input" | "-i" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --input");
                }
                input = Some(PathBuf::from(&arguments[index]));
            }
            "--input-url" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --input-url");
                }
                input_url = Some(arguments[index].clone());
            }
            "--default-timezone" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --default-timezone");
                }
                let name = &arguments[index];
                let tz = schedule_core::value::timezone::parse_tz(name)
                    .with_context(|| format!("Unknown timezone: {name}"))?;
                args.default_timezone = Some(tz.name().to_string());
            }
            "--default-start-time" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --default-start-time");
                }
                let raw = &arguments[index];
                let dt = schedule_core::value::time::parse_datetime(raw)
                    .with_context(|| format!("Could not parse datetime: {raw}"))?;
                args.default_start_time = Some(dt);
            }
            "--default-end-time" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --default-end-time");
                }
                let raw = &arguments[index];
                let dt = schedule_core::value::time::parse_datetime(raw)
                    .with_context(|| format!("Could not parse datetime: {raw}"))?;
                args.default_end_time = Some(dt);
            }
            "--output" | "-o" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --output");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&args.output_jobs, &path)?;
                args.output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::Output,
                });
                first_setting_index = None;
            }
            "--export" | "-e" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&args.output_jobs, &path)?;
                args.output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::Export,
                });
                first_setting_index = None;
            }
            "--export-embed" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-embed");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&args.output_jobs, &path)?;
                args.output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::ExportEmbed,
                });
                first_setting_index = None;
            }
            "--export-embed-head" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-embed-head");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&args.output_jobs, &path)?;
                args.output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::ExportEmbedHead,
                });
                first_setting_index = None;
            }
            "--export-embed-body" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-embed-body");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&args.output_jobs, &path)?;
                args.output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::ExportEmbedBody,
                });
                first_setting_index = None;
            }
            "--export-test" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-test");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&args.output_jobs, &path)?;
                args.output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::ExportTest,
                });
                first_setting_index = None;
            }
            "--schedule-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --schedule-table");
                }
                args.table_options.schedule = parse_table_mode(&arguments[index])?;
            }
            "--roommap-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --roommap-table");
                }
                args.table_options.rooms = parse_table_mode(&arguments[index])?;
            }
            "--prefix-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --prefix-table");
                }
                args.table_options.panel_types = parse_table_mode(&arguments[index])?;
            }
            "--presenter-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --presenter-table");
                }
                args.table_options.people = parse_table_mode(&arguments[index])?;
            }
            "--hotel-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --hotel-table");
                }
                args.table_options.hotel_rooms = parse_table_mode(&arguments[index])?;
            }
            "--timeline-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --timeline-table");
                }
                args.table_options.timeline = parse_table_mode(&arguments[index])?;
            }
            "--export-csv-dir" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-csv-dir");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&args.output_jobs, &path)?;
                args.output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::ExportCsv,
                });
                first_setting_index = None;
            }
            "--export-xlsx-grid" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-xlsx-grid");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&args.output_jobs, &path)?;
                args.output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::ExportXlsxGrid,
                });
                first_setting_index = None;
            }
            #[cfg(feature = "layout")]
            "--export-layout" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-layout");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&args.output_jobs, &path)?;
                args.output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::ExportLayout,
                });
                first_setting_index = None;
            }
            #[cfg(feature = "layout")]
            "--brand-config" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --brand-config");
                }
                current_settings.brand_config = Some(PathBuf::from(&arguments[index]));
            }
            #[cfg(feature = "layout")]
            "--layout-config" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --layout-config");
                }
                current_settings.layout_config = Some(PathBuf::from(&arguments[index]));
                // Selecting a config file means "render its jobs", so discard any
                // command-line layout accumulated so far.
                current_settings.layout = None;
            }
            #[cfg(feature = "layout")]
            "--default-layouts" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                // Revert to rendering the jobs from the layout TOML.
                current_settings.layout = None;
            }
            #[cfg(feature = "layout")]
            arg if arg.starts_with("--layout.") => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                let rest = &arg["--layout.".len()..];
                let (key, value) = match rest.split_once('=') {
                    Some((k, v)) => (k, Some(v)),
                    None => (rest, None),
                };
                let job = current_settings.layout.get_or_insert_with(Default::default);
                layout_config::apply_layout_arg(job, key, value)?;
            }
            #[cfg(feature = "layout")]
            "--stable-timestamps" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.stable_timestamps = true;
            }
            "--check" | "--validate" => {
                args.check_only = true;
            }
            "--builtin-css" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.widget_css = None;
            }
            "--builtin-js" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.widget_js = None;
            }
            "--builtin-widget" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.widget_css = None;
                current_settings.widget_js = None;
            }
            "--builtin-template" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.test_template = None;
            }
            #[cfg(feature = "layout")]
            "--builtin-brand" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.brand_config = None;
            }
            "--builtin" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.widget_css = None;
                current_settings.widget_js = None;
                current_settings.test_template = None;
                #[cfg(feature = "layout")]
                {
                    current_settings.brand_config = None;
                }
            }
            "--default" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings = OutputSettings::default();
            }
            "--widget" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --widget");
                }
                let base = arguments[index].clone();
                current_settings.widget_css = Some(format!("{base}.css"));
                current_settings.widget_js = Some(format!("{base}.js"));
            }
            "--widget-css" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --widget-css");
                }
                current_settings.widget_css = Some(arguments[index].clone());
            }
            "--widget-js" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --widget-js");
                }
                current_settings.widget_js = Some(arguments[index].clone());
            }
            "--test-template" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --test-template");
                }
                current_settings.test_template = Some(arguments[index].clone());
            }
            "--title" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --title");
                }
                current_settings.title = arguments[index].clone();
            }
            "--minified" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.minified = true;
            }
            "--no-minified" | "--for-debug" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.minified = false;
            }
            "--embed-as-html" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.embed_as_html = true;
            }
            "--embed-as-json" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.embed_as_html = false;
            }
            "--style-page" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.style_page = Some(true);
            }
            "--no-style-page" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.style_page = Some(false);
            }
            "--private" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.private_export = true;
            }
            "--public" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.private_export = false;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            value if !value.starts_with('-') && input.is_none() => {
                input = Some(PathBuf::from(value));
            }
            other => {
                anyhow::bail!("Unknown argument: {other}");
            }
        }

        index += 1;
    }

    args.input = input;
    args.input_url = input_url;

    if args.input.is_none() && args.input_url.is_none() {
        anyhow::bail!("--input or --input-url is required");
    }

    if args.input.is_some() && args.input_url.is_some() {
        anyhow::bail!("Cannot specify both --input and --input-url");
    }

    if let Some(unused_index) = first_setting_index {
        anyhow::bail!(
            "Settings specified at argument '{}' but no output option follows. \
             Settings must precede --output, --export, --export-embed, or --export-test.",
            arguments[unused_index]
        );
    }

    if args.output_jobs.is_empty() && !args.check_only {
        anyhow::bail!(
            "At least one output option is required \
             (--output, --export, --export-embed, --export-test, --export-csv-dir, --export-xlsx-grid) unless --check is specified"
        );
    }

    Ok(args)
}

fn print_usage() {
    eprintln!(
        "Usage: cosam-convert --input <file> | --input-url <url> [options]\n\
         \n\
         Input:\n\
         \x20 --input, -i <file>                   Input file (.xlsx, .schedule, or CSV directory)\n\
         \x20 --input-url <url>                    Fetch embedded widget JSON from a webpage URL\n\
         \n\
         Output commands (each captures the current settings snapshot):\n\
         \x20 --output, -o <file>                  Save schedule (.xlsx or native binary)\n\
         \x20 --export, -e <file.json>             Export widget JSON\n\
         \x20 --export-embed <file.html>           Export embeddable HTML (inline CSS/JS + schedule data)\n\
         \x20 --export-embed-head <file.html>      Export engine half (CSS/JS + resident bootstrap) for site-wide Code Injection Header\n\
         \x20 --export-embed-body <file.html>      Export content half (root + data) for the page code block; pairs with --export-embed-head\n\
         \x20 --export-test <file.html>            Export standalone test page (Squarespace sim)\n\
         \x20 --export-csv-dir <dir>               Export CSV files to directory (UTF-8 comma-delimited)\n\
         \x20 --export-xlsx-grid <file.xlsx>       Export grid reference sheets only (no data tables)\n\
         \x20 --export-layout <dir|file>           Render print layouts via Typst (requires typst on PATH).\n\
         \x20                                      Without --layout.*, writes the layout-TOML jobs as PDFs under <dir>.\n\
         \x20                                      With --layout.*, renders that one job to <file>.\n\
         \n\
         Validation:\n\
         \x20 --check, --validate                  Report conflicts; exit non-zero if any found\n\
         \n\
         Table names (for XLSX import):\n\
         \x20 --schedule-table <mode|name>          Schedule table: 'default', 'skip', or custom name\n\
         \x20 --roommap-table <mode|name>           Room map table: 'default', 'skip', or custom name\n\
         \x20 --prefix-table <mode|name>            Panel types table: 'default', 'skip', or custom name\n\
         \x20 --presenter-table <mode|name>         Presenters table: 'default', 'skip', or custom name\n\
         \x20 --hotel-table <mode|name>             Hotel table: 'default', 'skip', or custom name\n\
         \x20 --timeline-table <mode|name>          Timeline table: 'default', 'skip', or custom name\n\
         \n\
         Timezone / schedule window (defaults; the source Meta sheet wins if present):\n\
         \x20 --default-timezone <name>            IANA name or abbreviation (e.g. America/New_York, EDT, UTC);\n\
         \x20                                      falls back to the system local zone when unset\n\
         \x20 --default-start-time <datetime>      Schedule-window start (extended by panels scheduled earlier)\n\
         \x20 --default-end-time <datetime>        Schedule-window end (extended by panels scheduled later)\n\
         \n\
         Output settings (apply to all subsequent output commands until overridden):\n\
         \x20 --title <string>                     Event title for widget JSON and test pages\n\
         \x20 --widget <basename>                  Set CSS and JS to <basename>.css and <basename>.js\n\
         \x20 --widget-css <path>                  Override CSS source (default: builtin)\n\
         \x20 --widget-js <path>                   Override JS source (default: builtin)\n\
         \x20 --test-template <path>               Override test page template (default: builtin)\n\
         \x20 --brand-config <file>                Brand config for layout (default: config/brand.toml)\n\
         \x20 --layout-config <file>               Use the jobs from <file> (reverts any --layout.* to TOML jobs)\n\
         \x20 --layout.<key>[=<value>]             Define a single layout job on the command line; --export-layout\n\
         \x20                                      writes it to its path. Keys mirror layout.toml fields, e.g.\n\
         \x20                                      --layout.import=flyer --layout.paper=letter --layout.cards.\n\
         \x20                                      Repeat --layout.import to stack presets.\n\
         \x20 --default-layouts                    Revert to rendering the jobs from the layout TOML\n\
         \x20 --stable-timestamps                  Use modified time as generated time (reproducible layout output)\n\
         \x20 --minified                           Minify HTML output (default)\n\
         \x20 --no-minified, --for-debug           Skip minification\n\
         \x20 --embed-as-json                      Embed schedule as gzip+base64 JSON\n\
         \x20 --embed-as-html                      Embed schedule as widget-html semantic HTML (default)\n\
         \x20 --style-page                         Set stylePageBody: true in widget init\n\
         \x20 --no-style-page                      Set stylePageBody: false in widget init\n\
         \x20 --public                             Exclude private panels, timeline, and uncredited presenters in export\n\
         \x20 --private                            Include private panels, timeline, and uncredited presenters in export\n\
         \n\
         Builtin resource shortcuts:\n\
         \x20 --builtin-css                        Use builtin CSS\n\
         \x20 --builtin-js                         Use builtin JS\n\
         \x20 --builtin-widget                     Use builtin CSS and JS\n\
         \x20 --builtin-template                   Use builtin test template\n\
         \x20 --builtin-brand                      Use builtin brand defaults (no brand.toml)\n\
         \x20 --builtin                            Use all builtin resources\n\
         \x20 --default                            Reset all settings to defaults\n\
         \n\
         Examples:\n\
         \x20 cosam-convert --input schedule.xlsx --export public.json\n\
         \x20 cosam-convert --input schedule.xlsx --check --export public.json\n\
         \x20 cosam-convert --input schedule.xlsx --output full.schedule --export public.json\n\
         \x20 cosam-convert --input schedule.xlsx --export public.json --export-layout output/layout\n\
         \x20 cosam-convert --input schedule.xlsx --layout.import=flyer --layout.paper=letter --export-layout output/flyer.pdf\n\
         \x20 cosam-convert --input schedule.xlsx --layout.format=idml --export-layout output/sched.idml  (requires --features idml)\n\
         \x20 cosam-convert --input csv_dir --export public.json\n\
         \x20 cosam-convert --input schedule.xlsx --export-csv-dir csv_output\n\
         \x20 cosam-convert --input-url https://example.com/schedule --export public.json\n\
         \x20 cosam-convert --input schedule.xlsx --title \"Event 2026\" \\\n\
         \x20   --minified --export-embed embed.html --no-minified --export-embed debug.html"
    );
}

// ── Timezone / window resolution ────────────────────────────────────────────

/// Fill in any schedule-window metadata the source didn't already provide.
///
/// The source (e.g. an XLSX `Meta` sheet) is authoritative; the CLI `--default-*`
/// options only apply to fields still unset after loading. The timezone always
/// ends up populated: CLI default → system local → `"UTC"`.
fn apply_timezone_defaults(schedule: &mut Schedule, cli: &CliArgs) {
    if schedule.metadata.timezone.is_none() {
        let resolved =
            schedule_core::value::timezone::resolve_timezone(&[cli.default_timezone.as_deref()]);
        schedule.metadata.timezone = Some(resolved);
    }
    if schedule.metadata.start_time.is_none() {
        schedule.metadata.start_time = cli.default_start_time;
    }
    if schedule.metadata.end_time.is_none() {
        schedule.metadata.end_time = cli.default_end_time;
    }
}

// ── Schedule loading ──────────────────────────────────────────────────────────

fn load_schedule(path: &Path, options: &TableImportOptions) -> Result<Schedule> {
    // Check if input is a directory (CSV import)
    if path.is_dir() {
        return import_csv(path, options)
            .with_context(|| format!("Failed to import CSV from {}", path.display()));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "xlsx" => import_xlsx(path, options)
            .with_context(|| format!("Failed to import {}", path.display())),
        _ => {
            let bytes = std::fs::read(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            Schedule::load_from_file(&bytes)
                .map_err(|e| anyhow::anyhow!("{}", e))
                .with_context(|| format!("Failed to load {}", path.display()))
        }
    }
}

// ── Output writing ────────────────────────────────────────────────────────────

fn write_output(schedule: &mut Schedule, path: &Path) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "xlsx" => export_xlsx(schedule, path)
            .with_context(|| format!("Failed to write XLSX {}", path.display())),
        _ => {
            let bytes = schedule.save_to_file();
            std::fs::write(path, &bytes)
                .with_context(|| format!("Failed to write {}", path.display()))
        }
    }
}

// ── Stats reporting ───────────────────────────────────────────────────────────

fn print_stats(schedule: &Schedule) {
    eprintln!(
        "Panels: {}, Rooms: {}, Panel types: {}, Presenters: {}",
        schedule.entity_count::<PanelEntityType>(),
        schedule.entity_count::<EventRoomEntityType>(),
        schedule.entity_count::<PanelTypeEntityType>(),
        schedule.entity_count::<PresenterEntityType>(),
    );
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cli = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("Error: {err}");
            print_usage();
            std::process::exit(1);
        }
    };

    // Load input from file or URL
    let mut input_type: InputType = if let Some(url) = &cli.input_url {
        eprintln!("Fetching: {}", url);
        let widget = match load_from_url(url) {
            Ok(w) => w,
            Err(err) => {
                eprintln!("Error: {err}");
                std::process::exit(1);
            }
        };
        InputType::WidgetJson(widget)
    } else if let Some(path) = &cli.input {
        eprintln!("Reading: {}", path.display());
        if is_widget_json_file(path) {
            // Load as widget JSON
            let widget = match load_from_file(path) {
                Ok(w) => w,
                Err(err) => {
                    eprintln!("Error loading widget JSON: {err}");
                    std::process::exit(1);
                }
            };
            InputType::WidgetJson(widget)
        } else {
            // Load as Schedule
            let mut sched = match load_schedule(path, &cli.table_options) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("Error: {err}");
                    std::process::exit(1);
                }
            };
            apply_timezone_defaults(&mut sched, &cli);
            InputType::Schedule(Box::new(sched))
        }
    } else {
        unreachable!()
    };

    // Print stats only if we have a full Schedule
    if let Ok(ref sched) = input_type.as_schedule() {
        print_stats(sched);

        let scheduling_conflicts = conflicts::detect_conflicts(sched);
        conflicts::print_conflicts(&scheduling_conflicts);

        if cli.check_only {
            if scheduling_conflicts.is_empty() {
                eprintln!("Validation completed successfully");
            } else {
                eprintln!(
                    "Validation failed — {} conflict(s) detected",
                    scheduling_conflicts.len()
                );
                std::process::exit(1);
            }
            return;
        }
    } else if cli.check_only {
        eprintln!("Validation is not supported for widget JSON input");
        std::process::exit(1);
    }

    let mut had_error = false;

    for job in &cli.output_jobs {
        let effective_title = job.settings.title.clone();

        let result: Result<()> = match job.job_type {
            OutputType::Output => {
                // Output needs Schedule - convert if needed
                match input_type.as_schedule() {
                    Ok(sched) => write_output(sched, &job.path).map(|()| {
                        eprintln!("Saved: {}", job.path.display());
                    }),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        had_error = true;
                        Ok(())
                    }
                }
            }
            OutputType::Export => {
                // Export uses widget JSON - get or generate it
                match input_type.as_widget(&effective_title, job.settings.private_export) {
                    Ok(widget) => {
                        let json = serde_json::to_string_pretty(&widget).unwrap_or_else(|e| {
                            eprintln!("Error: Failed to serialize widget JSON: {}", e);
                            had_error = true;
                            String::new()
                        });
                        if !had_error {
                            if let Err(e) = std::fs::write(&job.path, json) {
                                eprintln!("Error: Failed to write widget JSON: {}", e);
                                had_error = true;
                            } else {
                                eprintln!("Exported: {}", job.path.display());
                            }
                        }
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        had_error = true;
                        Ok(())
                    }
                }
            }
            OutputType::ExportCsv => {
                // ExportCsv needs Schedule
                match input_type.as_schedule() {
                    Ok(sched) => match export_csv(sched, &job.path) {
                        Ok(_) => {
                            eprintln!("Exported CSV: {}", job.path.display());
                        }
                        Err(err) => {
                            eprintln!("Error: {:#}", err);
                            had_error = true;
                        }
                    },
                    Err(e) => {
                        eprintln!("Error: {e}");
                        had_error = true;
                    }
                }
                Ok(())
            }
            OutputType::ExportXlsxGrid => {
                // ExportXlsxGrid needs Schedule
                match input_type.as_schedule() {
                    Ok(sched) => export_xlsx_grid(sched, &job.path).map(|()| {
                        eprintln!("Exported grid XLSX: {}", job.path.display());
                    }),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        had_error = true;
                        Ok(())
                    }
                }
            }
            #[cfg(feature = "layout")]
            OutputType::ExportLayout => {
                // ExportLayout needs Schedule
                match input_type.as_schedule() {
                    Ok(sched) => {
                        run_layout_export(sched, &effective_title, &job.path, &job.settings);
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        had_error = true;
                    }
                }
                Ok(())
            }
            OutputType::ExportEmbed
            | OutputType::ExportEmbedHead
            | OutputType::ExportEmbedBody
            | OutputType::ExportTest => {
                // ExportEmbed/ExportTest can use either Schedule or WidgetJson
                let sources = match embed::WidgetSources::resolve(
                    job.settings.widget_css.as_deref(),
                    job.settings.widget_js.as_deref(),
                    job.settings.test_template.as_deref(),
                ) {
                    Ok(s) => s,
                    Err(err) => {
                        eprintln!("Error resolving widget sources: {err}");
                        had_error = true;
                        continue;
                    }
                };

                let widget =
                    match input_type.as_widget(&effective_title, job.settings.private_export) {
                        Ok(w) => w,
                        Err(e) => {
                            eprintln!("Error: {e}");
                            had_error = true;
                            continue;
                        }
                    };

                // Build the presentation config (branding + print formats) so the
                // widget's print formats can match the printed house style.
                let config: Option<ScheduleConfig> = {
                    #[cfg(feature = "layout")]
                    {
                        let brand =
                            brand_bridge::load_widget_brand(job.settings.brand_config.as_deref());
                        let print_formats = widget_config::load_print_formats(None);
                        Some(ScheduleConfig {
                            version: 1,
                            brand,
                            print_formats,
                        })
                    }
                    #[cfg(not(feature = "layout"))]
                    {
                        None
                    }
                };

                if job.settings.embed_as_html {
                    match job.job_type {
                        OutputType::ExportEmbed => embed::write_embed_html_widget_html(
                            &job.path,
                            &widget,
                            &sources,
                            job.settings.minified,
                            job.settings.style_page,
                        ),
                        OutputType::ExportEmbedHead => embed::write_embed_head_widget_html(
                            &job.path,
                            config.as_ref(),
                            &sources,
                            job.settings.minified,
                            job.settings.style_page,
                        ),
                        OutputType::ExportEmbedBody => embed::write_embed_body_widget_html(
                            &job.path,
                            &widget,
                            job.settings.minified,
                        ),
                        OutputType::ExportTest => embed::write_test_html_widget_html(
                            &job.path,
                            &widget,
                            &effective_title,
                            &sources,
                            job.settings.minified,
                            job.settings.style_page,
                        ),
                        _ => unreachable!(),
                    }
                } else {
                    let json_data = serde_json::to_string_pretty(&widget).unwrap_or_else(|e| {
                        eprintln!("Error: Failed to serialize widget JSON: {e}");
                        had_error = true;
                        String::new()
                    });
                    match job.job_type {
                        OutputType::ExportEmbed => embed::write_embed_html(
                            &job.path,
                            &json_data,
                            &sources,
                            job.settings.minified,
                            job.settings.style_page,
                        ),
                        OutputType::ExportEmbedHead => embed::write_embed_head_json(
                            &job.path,
                            &sources,
                            job.settings.minified,
                            job.settings.style_page,
                        ),
                        OutputType::ExportEmbedBody => embed::write_embed_body_json(
                            &job.path,
                            &json_data,
                            job.settings.minified,
                        ),
                        OutputType::ExportTest => embed::write_test_html(
                            &job.path,
                            &json_data,
                            &effective_title,
                            &sources,
                            job.settings.minified,
                            job.settings.style_page,
                        ),
                        _ => unreachable!(),
                    }
                }
            }
        };

        if let Err(err) = result {
            eprintln!("Error writing {}: {err}", job.path.display());
            had_error = true;
        }
    }

    if had_error {
        std::process::exit(1);
    }
}

// ── Layout export ─────────────────────────────────────────────────────────────

#[cfg(feature = "layout")]
fn run_layout_export(
    schedule: &schedule_core::schedule::Schedule,
    title: &str,
    layout_dir: &Path,
    settings: &OutputSettings,
) {
    use schedule_layout::{
        brand::BrandConfig, config::LayoutConfig, document, from_schedule, model::ScheduleData,
    };
    use std::fs;
    use std::path::PathBuf;

    use crate::layout_config::LayoutDefaults;

    let brand_path = settings
        .brand_config
        .clone()
        .unwrap_or_else(|| PathBuf::from("config/brand.toml"));
    let brand = match BrandConfig::load(&brand_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "warning: brand config {:?}: {e}; using defaults",
                brand_path
            );
            BrandConfig::default()
        }
    };

    // Build a layout dataset at a given visibility. Break synthesis runs over
    // whichever panel set is visible, so the public and private views are each
    // internally consistent (the public view is byte-identical to before).
    let build_data = |private: bool| -> Option<ScheduleData> {
        match from_schedule(schedule, title, private) {
            Ok(mut d) => {
                // For reproducible test output, pin the generated time to the
                // (stable) modified time so the footer no longer varies per run.
                if settings.stable_timestamps {
                    d.meta.generated = d.meta.modified.clone();
                }
                Some(d)
            }
            Err(e) => {
                eprintln!("warning: building layout data (private={private}): {e}");
                None
            }
        }
    };

    let data = match build_data(false) {
        Some(d) => d,
        None => {
            eprintln!("skipping layout export");
            return;
        }
    };

    /// A single default layout job: layout config + output filename base stem.
    #[derive(Clone)]
    struct LayoutOutputJob {
        config: LayoutConfig,
        /// Base filename stem (no extension). Split qualifiers are appended with `-`.
        stem: String,
        /// Explicit output path from `--export-layout` when a command-line layout
        /// was configured. `None` for jobs from the layout TOML (which write into
        /// the export directory under per-paper subdirectories).
        output_override: Option<PathBuf>,
    }

    /// Convert resolved `JobConfig`s to `LayoutOutputJob`s.
    fn convert_jobs(
        jobs: &[(layout_config::JobConfig, Option<String>)],
        timelines: &std::collections::HashMap<String, layout_config::CustomTimeline>,
        schedule_range: Option<(&str, &str)>,
    ) -> Vec<(LayoutOutputJob, Option<String>)> {
        jobs.iter()
            .map(|(job, brand_override)| {
                let (config, stem) = job.to_layout_config(timelines, schedule_range);
                (
                    LayoutOutputJob {
                        config,
                        stem,
                        output_override: None,
                    },
                    brand_override.clone(),
                )
            })
            .collect()
    }

    // Load optional user overrides from config/layout.toml
    let layout_defaults_path = settings.layout_config.clone().unwrap_or_else(|| {
        settings
            .brand_config
            .as_ref()
            .and_then(|b| b.parent())
            .map(|p| p.join("layout.toml"))
            .unwrap_or_else(|| PathBuf::from("config/layout.toml"))
    });
    let user_defaults = LayoutDefaults::load(&layout_defaults_path).unwrap_or_default();

    let global_brand = || {
        settings
            .brand_config
            .clone()
            .map(|p| p.to_string_lossy().to_string())
    };

    // Schedule date range for resolving loose/recurring time expressions.
    // The widget format uses "" for an absent naive datetime; treat that as None.
    let sched_range: Option<(&str, &str)> = {
        let start = data.meta.start_time.as_str();
        let end = data.meta.end_time.as_str();
        (!start.is_empty() && !end.is_empty()).then_some((start, end))
    };

    // Determine which jobs to run:
    // - A command-line layout (`--layout.*`) renders a single job; the export
    //   path is used as the output file name. Imports resolve against the presets
    //   from both layout-default.toml and the user's layout.toml (user wins).
    // - Otherwise, if layout.toml defines jobs, use those (with preset resolution).
    // - Otherwise use the embedded default jobs from `default_layout()`.
    let jobs_to_run: Vec<(LayoutOutputJob, Option<String>)> =
        if let Some(cli_job) = settings.layout.clone() {
            let mut presets = LayoutDefaults::default_layout().presets;
            presets.extend(user_defaults.presets.clone());
            let resolved = match cli_job.resolve(&presets, &mut Vec::new()) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("warning: resolving --layout.* job: {e}; skipping layout export");
                    return;
                }
            };
            let brand = resolved.brand_config.clone().or_else(global_brand);
            let mut converted =
                convert_jobs(&[(resolved, brand)], &user_defaults.timelines, sched_range);
            // Use the export path as the output file name for this single job.
            if let Some((job, _)) = converted.first_mut() {
                job.output_override = Some(layout_dir.to_path_buf());
            }
            converted
        } else if user_defaults.jobs.is_empty() {
            // For default layout, use the global brand config
            convert_jobs(
                &LayoutDefaults::default_layout()
                    .resolve_jobs()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(j, b)| {
                        (
                            j,
                            b.or_else(|| {
                                settings
                                    .brand_config
                                    .clone()
                                    .map(|p| p.to_string_lossy().to_string())
                            }),
                        )
                    })
                    .collect::<Vec<_>>(),
                &user_defaults.timelines,
                sched_range,
            )
        } else {
            // Resolve jobs with presets and per-job brand configs
            let resolved = user_defaults.resolve_jobs().unwrap_or_default();
            // If a job doesn't have a brand_config, use the global one
            let with_global_fallback: Vec<_> = resolved
                .into_iter()
                .map(|(job, brand)| {
                    let brand = brand.or_else(|| {
                        settings
                            .brand_config
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                    });
                    (job, brand)
                })
                .collect();
            convert_jobs(&with_global_fallback, &user_defaults.timelines, sched_range)
        };

    // Build the private view only when a job asks for it. Jobs that request
    // private data fall back to the public view if the private build failed.
    let needs_private = jobs_to_run
        .iter()
        .any(|(job, _)| job.config.include_private);
    let data_private = if needs_private {
        build_data(true)
    } else {
        None
    };

    for (job, brand_override) in jobs_to_run {
        // Load brand config - use job-specific if provided, otherwise global
        let job_brand = if let Some(brand_path_str) = brand_override {
            let brand_path = PathBuf::from(brand_path_str);
            match BrandConfig::load(&brand_path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!(
                        "warning: job '{}' brand config {:?}: {e}; using defaults",
                        job.stem, brand_path
                    );
                    BrandConfig::default()
                }
            }
        } else {
            // Clone the global brand (we'll need to clone here since brand is used across iterations)
            brand.clone()
        };

        // Private jobs render the private view; fall back to public if it is
        // unavailable (private build failed).
        let job_data = if job.config.include_private {
            data_private.as_ref().unwrap_or(&data)
        } else {
            &data
        };

        // IDML export is a separate path: one package per job, no Typst compile.
        if matches!(
            job.config.format,
            schedule_layout::config::LayoutFormat::Idml
        ) {
            #[cfg(feature = "idml")]
            {
                let base_stem = match &job.output_override {
                    Some(path) => path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| job.stem.clone()),
                    None => job.stem.clone(),
                };
                // A command-line export path with an extension is written verbatim;
                // otherwise the package lands beside the (per-paper) PDF output.
                let out_path = match &job.output_override {
                    Some(path) if path.extension().is_some() => path.clone(),
                    Some(path) => path
                        .parent()
                        .filter(|p| !p.as_os_str().is_empty())
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(format!("{base_stem}-{}.idml", job.config.paper.dir_name())),
                    None => layout_dir
                        .join(job.config.paper.dir_name())
                        .join(format!("{base_stem}-{}.idml", job.config.paper.dir_name())),
                };
                if let Some(dir) = out_path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    if let Err(e) = fs::create_dir_all(dir) {
                        eprintln!("warning: creating {:?}: {e}; skipping IDML job", dir);
                        continue;
                    }
                }
                match schedule_layout::idml::generate_idml(job_data, &job_brand, &job.config) {
                    Ok(bytes) => match fs::write(&out_path, bytes) {
                        Ok(()) => eprintln!("wrote {}", out_path.display()),
                        Err(e) => eprintln!("warning: writing {:?}: {e}", out_path),
                    },
                    Err(e) => eprintln!("warning: IDML generation for '{}': {e}", job.stem),
                }
            }
            #[cfg(not(feature = "idml"))]
            {
                eprintln!(
                    "warning: job '{}' requests format=idml but cosam-convert was built \
                     without the `idml` feature; skipping (rebuild with --features idml)",
                    job.stem
                );
            }
            continue;
        }

        let outputs = document::generate(job_data, &job_brand, &job.config);

        let font_args: Vec<String> = job_brand
            .fonts
            .font_dir
            .as_ref()
            .and_then(|d| d.to_str())
            .map(|d| vec!["--font-path".to_string(), d.to_string()])
            .unwrap_or_default();

        // Determine the base filename stem and the .typ / .pdf directories.
        //
        // A command-line layout (`output_override` set) writes next to the
        // requested export path, using its file stem as the base. Jobs from the
        // layout TOML write into a per-paper-size subdirectory of the export dir,
        // with a shared `typ/` directory.
        let base_stem = match &job.output_override {
            Some(path) => path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| job.stem.clone()),
            None => job.stem.clone(),
        };
        let (typ_dir, pdf_dir): (PathBuf, PathBuf) = match &job.output_override {
            Some(path) => {
                let dir = path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."));
                (dir.clone(), dir)
            }
            None => (
                layout_dir.join("typ"),
                layout_dir.join(job.config.paper.dir_name()),
            ),
        };

        if let Err(e) = fs::create_dir_all(&pdf_dir) {
            eprintln!(
                "warning: creating {:?}: {e}; skipping {:?}",
                pdf_dir, job.config.content
            );
            continue;
        }
        if typ_dir != pdf_dir {
            if let Err(e) = fs::create_dir_all(&typ_dir) {
                eprintln!(
                    "warning: creating {:?}: {e}; .typ files may not be written",
                    typ_dir
                );
            }
        }

        let single_output = outputs.len() == 1;
        for (qualifier, typ_src) in &outputs {
            let file_stem = [
                base_stem.as_str(),
                job.config.paper.dir_name(),
                qualifier.as_str(),
            ]
            .iter()
            .copied()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
            let typ_path = typ_dir.join(format!("{file_stem}.typ"));
            // A command-line layout that produces a single output and was given an
            // explicit file path (with extension) writes that path verbatim.
            let pdf_path = job
                .output_override
                .as_ref()
                .filter(|p| p.extension().is_some() && single_output)
                .cloned()
                .unwrap_or_else(|| pdf_dir.join(format!("{file_stem}.pdf")));
            if let Err(e) = fs::write(&typ_path, typ_src) {
                eprintln!("warning: writing {:?}: {e}", typ_path);
                continue;
            }
            let status = std::process::Command::new("typst")
                .arg("compile")
                .arg("--root")
                .arg("/")
                .args(&font_args)
                .arg(&typ_path)
                .arg(&pdf_path)
                .status();
            match status {
                Ok(s) if s.success() => eprintln!("compiled {}", pdf_path.display()),
                Ok(s) => eprintln!("warning: typst exited {} for {:?}", s, typ_path),
                Err(e) => eprintln!("warning: typst compile {:?}: {e}", typ_path),
            }
        }
    }
}
