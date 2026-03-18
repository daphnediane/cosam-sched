use std::path::PathBuf;

use schedule_core::data::{Schedule, XlsxImportOptions, JsonExportMode};

struct CliArgs {
    input: PathBuf,
    output: PathBuf,
    title: String,
    export_mode: JsonExportMode,
    schedule_table: String,
    roommap_table: String,
    prefix_table: String,
    config_file: Option<PathBuf>,
}

fn parse_args() -> anyhow::Result<CliArgs> {
    let arguments: Vec<String> = std::env::args().collect();
    let mut input: Option<PathBuf> = None;
    let mut output = PathBuf::from("schedule.json");
    let mut title = String::new();
    let mut export_mode = JsonExportMode::Public;
    let mut schedule_table = "Schedule".to_string();
    let mut roommap_table = "RoomMap".to_string();
    let mut prefix_table = "Prefix".to_string();
    let mut config_file: Option<PathBuf> = None;

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
                output = PathBuf::from(&arguments[index]);
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
            "--staff" => {
                export_mode = JsonExportMode::Staff;
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
        title,
        export_mode,
        schedule_table,
        roommap_table,
        prefix_table,
        config_file,
    })
}

fn print_usage() {
    eprintln!(
        "Usage: cosam-convert --input <file.xlsx|file.json> [options]\n\
         \n\
         Options:\n\
         \x20 --output, -o <file>       Output JSON file (default: schedule.json)\n\
         \x20 --title, -t <string>      Event title (for XLSX import)\n\
         \x20 --config, -c <file.yaml>  Reserved for future Google Sheets support\n\
         \x20 --staff                   Include staff/hidden events\n\
         \x20 --schedule-table <name>   Sheet name for schedule data (default: Schedule)\n\
         \x20 --roommap-table <name>    Sheet name for room mapping (default: RoomMap)\n\
         \x20 --prefix-table <name>     Sheet name for panel types (default: Prefix)\n\
         \x20 --help, -h                Show this help message"
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
        "Events: {}, Rooms: {}, Panel types: {}, Presenters: {}",
        schedule.events.len(),
        schedule.rooms.len(),
        schedule.panel_types.len(),
        schedule.presenters.len()
    );

    match schedule.save_json_with_mode(&cli.output, cli.export_mode) {
        Ok(()) => eprintln!("Written: {}", cli.output.display()),
        Err(error) => {
            eprintln!("Error saving: {error}");
            std::process::exit(1);
        }
    }
}
