/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::PathBuf;

use schedule_core::data::{
    Schedule, WidgetSources, XlsxImportOptions, export_to_xlsx, write_embed_html, write_test_html,
};

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

#[derive(Debug)]
struct OutputJob {
    path: PathBuf,
    settings: OutputSettings,
    job_type: OutputType,
}

#[derive(Debug)]
enum OutputType {
    Export,
    ExportEmbed,
    ExportTest,
}

struct CliArgs {
    input: PathBuf,
    output_jobs: Vec<OutputJob>,
    check_only: bool,
    schedule_table: String,
    roommap_table: String,
    prefix_table: String,
    config_file: Option<PathBuf>,
    use_modified_as_generated: bool,
}

fn check_duplicate_output(output_jobs: &[OutputJob], path: &PathBuf) -> anyhow::Result<()> {
    if output_jobs.iter().any(|job| job.path == *path) {
        anyhow::bail!("Output file specified multiple times: {}", path.display());
    }
    Ok(())
}

fn parse_args() -> anyhow::Result<CliArgs> {
    let arguments: Vec<String> = std::env::args().collect();
    let mut input: Option<PathBuf> = None;
    let mut output_jobs: Vec<OutputJob> = Vec::new();
    let mut check_only = false;
    let mut schedule_table = "Schedule".to_string();
    let mut roommap_table = "RoomMap".to_string();
    let mut prefix_table = "Prefix".to_string();
    let mut config_file: Option<PathBuf> = None;
    let mut use_modified_as_generated = false;

    // Current settings that get cloned for each output
    let mut current_settings = OutputSettings::default();
    // Track first setting index that hasn't been consumed by an output
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
                    job_type: OutputType::Export,
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
            "--config" | "-c" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --config");
                }
                config_file = Some(PathBuf::from(&arguments[index]));
            }
            "--check" | "--validate" => {
                check_only = true;
            }
            "--use-modified-as-generated" => {
                use_modified_as_generated = true;
            }
            // New builtin options
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
            // Settings options that track first usage
            "--widget" => {
                if first_setting_index.is_none() {
                    first_setting_index = Some(index);
                }
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --widget");
                }
                let widget_value = arguments[index].clone();
                current_settings.widget_css = Some(widget_value.clone());
                current_settings.widget_js = Some(widget_value);
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

    // Check for unused settings
    if let Some(unused_index) = first_setting_index {
        anyhow::bail!(
            "Settings specified at argument {} but no output file specified after them. \
             Settings must be followed by an output option (--output, --export, --export-embed, --export-test)",
            arguments[unused_index]
        );
    }

    if output_jobs.is_empty() && !check_only {
        anyhow::bail!(
            "At least one output option is required (--output, --export, --export-embed, --export-test) unless --check is specified"
        );
    }

    Ok(CliArgs {
        input,
        output_jobs,
        check_only,
        schedule_table,
        roommap_table,
        prefix_table,
        config_file,
        use_modified_as_generated,
    })
}

fn print_usage() {
    eprintln!(
        "Usage: cosam-convert --input <file.xlsx|file.json> [options]\n\
         \n\
         Options:\n\
         \x20 --output, -o <file.json|file.xlsx>  Save private/full schedule (format by extension)\n\
         \x20 --export, -e <file.json>            Export public schedule JSON\n\
         \x20 --export-embed <file.html>           Export embeddable HTML (inline CSS/JS/JSON)\n\
         \x20 --export-test <file.html>            Export standalone test page (Squarespace sim)\n\
         \x20 --check, --validate                  Validate input and exit with error if conflicts found\n\
         \x20 --config, -c <file.yaml>            Reserved for future Google Sheets support\n\
         \x20 --schedule-table <name>             Sheet name for schedule data (default: Schedule)\n\
         \x20 --roommap-table <name>              Sheet name for room mapping (default: RoomMap)\n\
         \x20 --prefix-table <name>               Sheet name for panel types (default: Prefix)\n\
         \x20 --help, -h                          Show this help message\n\
         \n\
         Output settings (apply to subsequent outputs):\n\
         \x20 --title <string>                     Event title (for test pages)\n\
         \x20 --widget <basename>                  Set both CSS and JS to basename.css/.js\n\
         \x20 --widget-css <path>                  Override CSS source (default: builtin)\n\
         \x20 --widget-js <path>                   Override JS source (default: builtin)\n\
         \x20 --test-template <path>                Override test page template (default: builtin)\n\
         \x20 --minified                           Minify output (default)\n\
         \x20 --no-minified, --for-debug           Skip minification for debugging\n\
         \x20 --style-page                         Set stylePageBody: true in widget init\n\
         \x20 --no-style-page                      Set stylePageBody: false in widget init\n\
         \n\
         Builtin resource shortcuts:\n\
         \x20 --builtin-css                         Use builtin CSS\n\
         \x20 --builtin-js                          Use builtin JS\n\
         \x20 --builtin-widget                      Use builtin CSS and JS\n\
         \x20 --builtin-template                    Use builtin template\n\
         \x20 --builtin                             Use builtin CSS, JS, and template\n\
         \x20 --default                            Reset all settings to defaults\n\
         \n\
         Examples:\n\
         \x20 cosam-convert --input schedule.xlsx --export public.json\n\
         \x20 cosam-convert --input schedule.xlsx --minified --export-embed min.html --no-minified --export-embed max.html\n\
         \x20 cosam-convert --input schedule.xlsx --check  # Validate only\n\
         \x20 cosam-convert --input schedule.xlsx --check --export public.json  # Validate before exporting"
    );
}

fn build_import_options(cli: &CliArgs) -> XlsxImportOptions {
    XlsxImportOptions {
        title: "Event Schedule".to_string(),
        schedule_table: cli.schedule_table.clone(),
        rooms_table: cli.roommap_table.clone(),
        panel_types_table: cli.prefix_table.clone(),
        use_modified_as_generated: cli.use_modified_as_generated,
    }
}

fn save_output(schedule: &Schedule, path: &std::path::Path) -> anyhow::Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "xlsx" => export_to_xlsx(schedule, path),
        _ => schedule.save_json(path),
    }
}

fn print_conflicts(schedule: &Schedule) {
    if schedule.conflicts.is_empty() {
        eprintln!("No conflicts detected");
        return;
    }

    eprintln!("Conflicts found: {}", schedule.conflicts.len());

    let mut room_conflicts = 0;
    let mut presenter_conflicts = 0;
    let mut group_presenter_conflicts = 0;
    let mut title_conflicts = 0;

    for conflict in &schedule.conflicts {
        match conflict.conflict_type.as_str() {
            "room" => room_conflicts += 1,
            "presenter" => presenter_conflicts += 1,
            "group_presenter" => group_presenter_conflicts += 1,
            "title_id_mismatch" => title_conflicts += 1,
            _ => {}
        }
    }

    if room_conflicts > 0 {
        eprintln!("  Room conflicts: {}", room_conflicts);
    }
    if presenter_conflicts > 0 {
        eprintln!("  Presenter conflicts: {}", presenter_conflicts);
    }
    if group_presenter_conflicts > 0 {
        eprintln!("  Group presenter conflicts: {}", group_presenter_conflicts);
    }
    if title_conflicts > 0 {
        eprintln!("  Title/ID mismatches: {}", title_conflicts);
    }

    // Count panel sessions with conflicts
    let mut sessions_with_conflicts = 0;
    let mut total_session_conflicts = 0;

    for panel in schedule.panels.values() {
        for part in &panel.parts {
            for session in &part.sessions {
                if !session.conflicts.is_empty() {
                    sessions_with_conflicts += 1;
                    total_session_conflicts += session.conflicts.len();
                }
            }
        }
    }

    if sessions_with_conflicts > 0 {
        eprintln!(
            "Panel sessions with conflicts: {} (total conflicts: {})",
            sessions_with_conflicts, total_session_conflicts
        );
    }

    // Show first few conflicts as examples
    let max_examples = 5;
    for (i, conflict) in schedule.conflicts.iter().take(max_examples).enumerate() {
        eprintln!(
            "  {}. {} vs {} ({})",
            i + 1,
            conflict.event1.name,
            conflict.event2.name,
            conflict.conflict_type
        );
        if let Some(ref presenter) = conflict.presenter {
            eprintln!("     Presenter: {}", presenter);
        }
    }

    if schedule.conflicts.len() > max_examples {
        eprintln!(
            "  ... and {} more conflicts",
            schedule.conflicts.len() - max_examples
        );
    }
}

fn main() {
    let cli = match parse_args() {
        Ok(arguments) => arguments,
        Err(error) => {
            eprintln!("{error}");
            print_usage();
            std::process::exit(1);
        }
    };

    if cli.config_file.is_some() {
        eprintln!(
            "Warning: --config is reserved for future Google Sheets support and is not used yet"
        );
    }

    let import_options = build_import_options(&cli);
    eprintln!("Reading: {}", cli.input.display());

    let mut schedule = match Schedule::load_auto(&cli.input, &import_options) {
        Ok(schedule) => schedule,
        Err(error) => {
            eprintln!("Error loading schedule: {error}");
            std::process::exit(1);
        }
    };

    eprintln!(
        "Panels: {}, Rooms: {}, Panel types: {}, Presenters: {}",
        schedule.panels.len(),
        schedule.rooms.len(),
        schedule.panel_types.len(),
        schedule.presenters.len()
    );

    // Report conflicts
    print_conflicts(&schedule);

    // If check mode and there are conflicts, exit with error
    if cli.check_only && !schedule.conflicts.is_empty() {
        eprintln!(
            "Validation failed - {} conflicts detected",
            schedule.conflicts.len()
        );
        std::process::exit(1);
    }

    if cli.check_only {
        eprintln!("Validation completed successfully");
        return;
    }

    let mut had_error = false;

    // Process all output jobs
    for job in &cli.output_jobs {
        // Update schedule title if custom title is provided for this job
        let original_title = schedule.meta.title.clone();
        if !job.settings.title.is_empty() {
            schedule.meta.title = job.settings.title.clone();
        }

        let result = match job.job_type {
            OutputType::Export => match schedule.export_public(&job.path) {
                Ok(()) => {
                    eprintln!("Exported: {}", job.path.display());
                    Ok(())
                }
                Err(error) => {
                    eprintln!("Error exporting {}: {error}", job.path.display());
                    Err(error)
                }
            },
            OutputType::ExportEmbed | OutputType::ExportTest => {
                // Resolve widget sources for this job
                let sources = match WidgetSources::resolve(
                    job.settings.widget_css.as_deref(),
                    job.settings.widget_js.as_deref(),
                    job.settings.test_template.as_deref(),
                ) {
                    Ok(sources) => sources,
                    Err(error) => {
                        eprintln!("Error resolving widget sources: {error}");
                        had_error = true;
                        continue;
                    }
                };

                let json_data = match schedule.export_public_json_string() {
                    Ok(json) => json,
                    Err(error) => {
                        eprintln!("Error generating public JSON: {error}");
                        had_error = true;
                        continue;
                    }
                };

                match job.job_type {
                    OutputType::ExportEmbed => {
                        match write_embed_html(
                            &job.path,
                            &json_data,
                            &sources,
                            job.settings.minified,
                            job.settings.style_page,
                        ) {
                            Ok(()) => {
                                eprintln!("Written: {}", job.path.display());
                                Ok(())
                            }
                            Err(error) => {
                                eprintln!("Error writing embed HTML: {error}");
                                Err(error)
                            }
                        }
                    }
                    OutputType::ExportTest => {
                        let title = if job.settings.title.is_empty() {
                            &schedule.meta.title
                        } else {
                            &job.settings.title
                        };
                        match write_test_html(
                            &job.path,
                            &json_data,
                            title,
                            &sources,
                            job.settings.minified,
                            job.settings.style_page,
                        ) {
                            Ok(()) => {
                                eprintln!("Written: {}", job.path.display());
                                Ok(())
                            }
                            Err(error) => {
                                eprintln!("Error writing test HTML: {error}");
                                Err(error)
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
        };

        // Restore original title for next job
        schedule.meta.title = original_title;

        if let Err(error) = result {
            had_error = true;
        }
    }

    if had_error {
        std::process::exit(1);
    }
}
