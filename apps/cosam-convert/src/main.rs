/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::PathBuf;

use schedule_core::data::{Schedule, XlsxImportOptions, export_to_xlsx};

mod widget_embed;

struct CliArgs {
    input: PathBuf,
    output: Option<PathBuf>,
    export: Option<PathBuf>,
    export_embed: Option<PathBuf>,
    export_test: Option<PathBuf>,
    title: String,
    schedule_table: String,
    roommap_table: String,
    prefix_table: String,
    config_file: Option<PathBuf>,
    widget: Option<String>,
    widget_css: Option<String>,
    widget_js: Option<String>,
    test_template: Option<String>,
    minified: bool,
    style_page: Option<bool>,
}

fn parse_args() -> anyhow::Result<CliArgs> {
    let arguments: Vec<String> = std::env::args().collect();
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut export: Option<PathBuf> = None;
    let mut export_embed: Option<PathBuf> = None;
    let mut export_test: Option<PathBuf> = None;
    let mut title = String::new();
    let mut schedule_table = "Schedule".to_string();
    let mut roommap_table = "RoomMap".to_string();
    let mut prefix_table = "Prefix".to_string();
    let mut config_file: Option<PathBuf> = None;
    let mut widget: Option<String> = None;
    let mut widget_css: Option<String> = None;
    let mut widget_js: Option<String> = None;
    let mut test_template: Option<String> = None;
    let mut minified = true;
    let mut style_page: Option<bool> = None;

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
                output = Some(PathBuf::from(&arguments[index]));
            }
            "--export" | "-e" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export");
                }
                export = Some(PathBuf::from(&arguments[index]));
            }
            "--title" | "-t" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --title");
                }
                title = arguments[index].clone();
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
            "--config" | "-c" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --config");
                }
                config_file = Some(PathBuf::from(&arguments[index]));
            }
            "--export-embed" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-embed");
                }
                export_embed = Some(PathBuf::from(&arguments[index]));
            }
            "--export-test" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --export-test");
                }
                export_test = Some(PathBuf::from(&arguments[index]));
            }
            "--widget" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --widget");
                }
                widget = Some(arguments[index].clone());
            }
            "--widget-css" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --widget-css");
                }
                widget_css = Some(arguments[index].clone());
            }
            "--widget-js" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --widget-js");
                }
                widget_js = Some(arguments[index].clone());
            }
            "--test-template" => {
                index += 1;
                if index >= arguments.len() {
                    anyhow::bail!("Missing value for --test-template");
                }
                test_template = Some(arguments[index].clone());
            }
            "--minified" => {
                minified = true;
            }
            "--no-minified" | "--for-debug" => {
                minified = false;
            }
            "--style-page" => {
                style_page = Some(true);
            }
            "--no-style-page" => {
                style_page = Some(false);
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

    Ok(CliArgs {
        input,
        output,
        export,
        export_embed,
        export_test,
        title,
        schedule_table,
        roommap_table,
        prefix_table,
        config_file,
        widget,
        widget_css,
        widget_js,
        test_template,
        minified,
        style_page,
    })
}

fn print_usage() {
    eprintln!(
        "Usage: cosam-convert --input <file.xlsx|file.json> [options]\n\
         \n\
         Options:\n\
         \x20 --output, -o <file.json|file.xlsx>  Save private/full schedule (format by extension)\n\
         \x20 --export, -e <file.json>            Export public schedule JSON\n\
         \x20 --title, -t <string>                Event title (for XLSX import)\n\
         \x20 --config, -c <file.yaml>            Reserved for future Google Sheets support\n\
         \x20 --schedule-table <name>             Sheet name for schedule data (default: Schedule)\n\
         \x20 --roommap-table <name>              Sheet name for room mapping (default: RoomMap)\n\
         \x20 --prefix-table <name>               Sheet name for panel types (default: Prefix)\n\
         \x20 --help, -h                          Show this help message\n\
         \n\
         Widget / Embed options:\n\
         \x20 --export-embed <file.html>           Export embeddable HTML (inline CSS/JS/JSON)\n\
         \x20 --export-test <file.html>            Export standalone test page (Squarespace sim)\n\
         \x20 --widget <builtin|dir|basename>      Override both CSS and JS sources\n\
         \x20 --widget-css <builtin|path>          Override CSS source only\n\
         \x20 --widget-js <builtin|path>           Override JS source only\n\
         \x20 --test-template <builtin|file>       Override test page template\n\
         \x20 --minified                           Minify output (default)\n\
         \x20 --no-minified, --for-debug           Skip minification for debugging\n\
         \x20 --style-page                         Set stylePageBody: true in widget init\n\
         \x20 --no-style-page                      Set stylePageBody: false in widget init\n\
         \n\
         If neither --output nor --export is given, the input is parsed and summarized."
    );
}

fn build_import_options(cli: &CliArgs) -> XlsxImportOptions {
    XlsxImportOptions {
        title: if cli.title.is_empty() {
            "Event Schedule".to_string()
        } else {
            cli.title.clone()
        },
        schedule_table: cli.schedule_table.clone(),
        rooms_table: cli.roommap_table.clone(),
        panel_types_table: cli.prefix_table.clone(),
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

    if !cli.title.is_empty() {
        schedule.meta.title = cli.title.clone();
    }

    eprintln!(
        "Panels: {}, Rooms: {}, Panel types: {}, Presenters: {}",
        schedule.panels.len(),
        schedule.rooms.len(),
        schedule.panel_types.len(),
        schedule.presenters.len()
    );

    // Report conflicts
    print_conflicts(&schedule);

    let mut had_error = false;

    if let Some(ref output_path) = cli.output {
        match save_output(&schedule, output_path) {
            Ok(()) => eprintln!("Written: {}", output_path.display()),
            Err(error) => {
                eprintln!("Error writing {}: {error}", output_path.display());
                had_error = true;
            }
        }
    }

    if let Some(ref export_path) = cli.export {
        match schedule.export_public(export_path) {
            Ok(()) => eprintln!("Exported: {}", export_path.display()),
            Err(error) => {
                eprintln!("Error exporting {}: {error}", export_path.display());
                had_error = true;
            }
        }
    }

    if cli.export_embed.is_some() || cli.export_test.is_some() {
        let sources = match widget_embed::WidgetSources::resolve(
            cli.widget.as_deref(),
            cli.widget_css.as_deref(),
            cli.widget_js.as_deref(),
            cli.test_template.as_deref(),
        ) {
            Ok(sources) => sources,
            Err(error) => {
                eprintln!("Error resolving widget sources: {error}");
                std::process::exit(1);
            }
        };

        let json_data = match schedule.export_public_json_string() {
            Ok(json) => json,
            Err(error) => {
                eprintln!("Error generating public JSON: {error}");
                std::process::exit(1);
            }
        };

        if let Some(ref embed_path) = cli.export_embed {
            match widget_embed::write_embed_html(
                embed_path,
                &json_data,
                &sources,
                cli.minified,
                cli.style_page,
            ) {
                Ok(()) => {}
                Err(error) => {
                    eprintln!("Error writing embed HTML: {error}");
                    had_error = true;
                }
            }
        }

        if let Some(ref test_path) = cli.export_test {
            let title = if cli.title.is_empty() {
                &schedule.meta.title
            } else {
                &cli.title
            };
            match widget_embed::write_test_html(
                test_path,
                &json_data,
                title,
                &sources,
                cli.minified,
                cli.style_page,
            ) {
                Ok(()) => {}
                Err(error) => {
                    eprintln!("Error writing test HTML: {error}");
                    had_error = true;
                }
            }
        }
    }

    if had_error {
        std::process::exit(1);
    }
}
