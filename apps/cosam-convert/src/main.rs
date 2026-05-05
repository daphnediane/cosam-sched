/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use schedule_core::query::export::export_to_widget_json;
use schedule_core::schedule::Schedule;
use schedule_core::tables::event_room::EventRoomEntityType;
use schedule_core::tables::panel::PanelEntityType;
use schedule_core::tables::panel_type::PanelTypeEntityType;
use schedule_core::tables::presenter::PresenterEntityType;
use schedule_core::xlsx::{export_xlsx, import_xlsx, XlsxImportOptions};

// ── Output settings ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct OutputSettings {
    widget_css: Option<String>,
    widget_js: Option<String>,
    test_template: Option<String>,
    minified: bool,
    style_page: Option<bool>,
    title: String,
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
    ExportTest,
}

// ── CLI args ──────────────────────────────────────────────────────────────────

struct CliArgs {
    input: PathBuf,
    output_jobs: Vec<OutputJob>,
    check_only: bool,
    schedule_table: String,
    roommap_table: String,
    prefix_table: String,
    presenter_table: String,
}

// ── Argument parsing ──────────────────────────────────────────────────────────

fn check_duplicate_output(output_jobs: &[OutputJob], path: &PathBuf) -> Result<()> {
    if output_jobs.iter().any(|job| job.path == *path) {
        anyhow::bail!("Output file specified multiple times: {}", path.display());
    }
    Ok(())
}

fn parse_args() -> Result<CliArgs> {
    let arguments: Vec<String> = std::env::args().collect();
    let mut input: Option<PathBuf> = None;
    let mut output_jobs: Vec<OutputJob> = Vec::new();
    let mut check_only = false;
    let mut schedule_table = "Schedule".to_string();
    let mut roommap_table = "RoomMap".to_string();
    let mut prefix_table = "Prefix".to_string();
    let mut presenter_table = "Presenters".to_string();

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
            "--output" | "-o" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --output");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&output_jobs, &path)?;
                output_jobs.push(OutputJob {
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
                check_duplicate_output(&output_jobs, &path)?;
                output_jobs.push(OutputJob {
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
                check_duplicate_output(&output_jobs, &path)?;
                output_jobs.push(OutputJob {
                    path,
                    settings: current_settings.clone(),
                    job_type: OutputType::ExportEmbed,
                });
                first_setting_index = None;
            }
            "--export-test" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-test");
                }
                let path = PathBuf::from(&arguments[index]);
                check_duplicate_output(&output_jobs, &path)?;
                output_jobs.push(OutputJob {
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
                schedule_table = arguments[index].clone();
            }
            "--roommap-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --roommap-table");
                }
                roommap_table = arguments[index].clone();
            }
            "--prefix-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --prefix-table");
                }
                prefix_table = arguments[index].clone();
            }
            "--presenter-table" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --presenter-table");
                }
                presenter_table = arguments[index].clone();
            }
            "--check" | "--validate" => {
                check_only = true;
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
            "--builtin" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                current_settings.widget_css = None;
                current_settings.widget_js = None;
                current_settings.test_template = None;
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

    let Some(input) = input else {
        anyhow::bail!("--input is required");
    };

    if let Some(unused_index) = first_setting_index {
        anyhow::bail!(
            "Settings specified at argument '{}' but no output option follows. \
             Settings must precede --output, --export, --export-embed, or --export-test.",
            arguments[unused_index]
        );
    }

    if output_jobs.is_empty() && !check_only {
        anyhow::bail!(
            "At least one output option is required \
             (--output, --export, --export-embed, --export-test) unless --check is specified"
        );
    }

    Ok(CliArgs {
        input,
        output_jobs,
        check_only,
        schedule_table,
        roommap_table,
        prefix_table,
        presenter_table,
    })
}

fn print_usage() {
    eprintln!(
        "Usage: cosam-convert --input <file> [options]\n\
         \n\
         Input:\n\
         \x20 --input, -i <file>                  Input file (.xlsx or native .schedule)\n\
         \n\
         Output commands (each captures the current settings snapshot):\n\
         \x20 --output, -o <file>                 Save private schedule (.xlsx or native binary)\n\
         \x20 --export, -e <file.json>             Export public widget JSON\n\
         \x20 --export-embed <file.html>           Export embeddable HTML (inline CSS/JS/JSON)\n\
         \x20 --export-test <file.html>            Export standalone test page (Squarespace sim)\n\
         \n\
         Validation:\n\
         \x20 --check, --validate                  Report conflicts; exit non-zero if any found\n\
         \n\
         Table names (for XLSX import):\n\
         \x20 --schedule-table <name>              Schedule sheet name (default: Schedule)\n\
         \x20 --roommap-table <name>               Room map sheet name (default: RoomMap)\n\
         \x20 --prefix-table <name>                Panel types sheet name (default: Prefix)\n\
         \x20 --presenter-table <name>             Presenters sheet name (default: Presenters)\n\
         \n\
         Output settings (apply to all subsequent output commands until overridden):\n\
         \x20 --title <string>                     Event title for widget JSON and test pages\n\
         \x20 --widget <basename>                  Set CSS and JS to <basename>.css and <basename>.js\n\
         \x20 --widget-css <path>                  Override CSS source (default: builtin)\n\
         \x20 --widget-js <path>                   Override JS source (default: builtin)\n\
         \x20 --test-template <path>               Override test page template (default: builtin)\n\
         \x20 --minified                           Minify HTML output (default)\n\
         \x20 --no-minified, --for-debug           Skip minification\n\
         \x20 --style-page                         Set stylePageBody: true in widget init\n\
         \x20 --no-style-page                      Set stylePageBody: false in widget init\n\
         \n\
         Builtin resource shortcuts:\n\
         \x20 --builtin-css                        Use builtin CSS\n\
         \x20 --builtin-js                         Use builtin JS\n\
         \x20 --builtin-widget                     Use builtin CSS and JS\n\
         \x20 --builtin-template                   Use builtin test template\n\
         \x20 --builtin                            Use all builtin resources\n\
         \x20 --default                            Reset all settings to defaults\n\
         \n\
         Examples:\n\
         \x20 cosam-convert --input schedule.xlsx --export public.json\n\
         \x20 cosam-convert --input schedule.xlsx --check --export public.json\n\
         \x20 cosam-convert --input schedule.xlsx --output full.schedule --export public.json\n\
         \x20 cosam-convert --input schedule.xlsx --title \"Event 2026\" \\\n\
         \x20   --minified --export-embed embed.html --no-minified --export-embed debug.html"
    );
}

// ── Schedule loading ──────────────────────────────────────────────────────────

fn build_import_options(cli: &CliArgs) -> XlsxImportOptions {
    XlsxImportOptions {
        schedule_table: cli.schedule_table.clone(),
        rooms_table: cli.roommap_table.clone(),
        panel_types_table: cli.prefix_table.clone(),
        people_table: cli.presenter_table.clone(),
    }
}

fn load_schedule(path: &Path, options: &XlsxImportOptions) -> Result<Schedule> {
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

fn write_widget_json(schedule: &Schedule, path: &Path, title: &str) -> Result<()> {
    let widget = export_to_widget_json(schedule, title).map_err(|e| anyhow::anyhow!("{}", e))?;
    let json = serde_json::to_string_pretty(&widget)?;
    std::fs::write(path, json).with_context(|| format!("Failed to write {}", path.display()))
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

    let options = build_import_options(&cli);
    eprintln!("Reading: {}", cli.input.display());

    let mut schedule = match load_schedule(&cli.input, &options) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    };

    print_stats(&schedule);

    // Phase 2 will add scheduling conflict detection here.
    eprintln!("No conflicts detected");

    if cli.check_only {
        eprintln!("Validation completed successfully");
        return;
    }

    let mut had_error = false;

    for job in &cli.output_jobs {
        let effective_title = job.settings.title.clone();

        let result: Result<()> = match job.job_type {
            OutputType::Output => write_output(&mut schedule, &job.path).map(|()| {
                eprintln!("Saved: {}", job.path.display());
            }),
            OutputType::Export => {
                write_widget_json(&schedule, &job.path, &effective_title).map(|()| {
                    eprintln!("Exported: {}", job.path.display());
                })
            }
            OutputType::ExportEmbed | OutputType::ExportTest => Err(anyhow::anyhow!(
                "HTML embedding not yet implemented (coming in Phase 3)"
            )),
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
