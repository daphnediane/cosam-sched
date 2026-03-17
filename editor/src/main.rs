mod data;
mod ui;

use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Focusable, KeyBinding, Menu, MenuItem, SystemMenuType, WindowBounds,
    WindowOptions, actions, px, size,
};

use data::{Schedule, XlsxImportOptions};
use ui::ScheduleEditor;
use ui::editor::{FileExportPublicJson, FileOpen, FileSave, FileSaveAs};

actions!(
    main,
    [
        Quit,
        HideApp,
        HideOtherApps,
        ShowAllApps,
        NewWindow,
        CloseWindow
    ]
);

struct CliArgs {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    title: String,
    staff_mode: bool,
    schedule_table: String,
    roommap_table: String,
    prefix_table: String,
    headless: bool,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut cli = CliArgs {
        input: None,
        output: None,
        title: String::new(),
        staff_mode: false,
        schedule_table: "Schedule".to_string(),
        roommap_table: "RoomMap".to_string(),
        prefix_table: "Prefix".to_string(),
        headless: false,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "-i" => {
                i += 1;
                if i < args.len() {
                    cli.input = Some(PathBuf::from(&args[i]));
                }
            }
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    cli.output = Some(PathBuf::from(&args[i]));
                }
            }
            "--title" | "-t" => {
                i += 1;
                if i < args.len() {
                    cli.title = args[i].clone();
                }
            }
            "--staff" => {
                cli.staff_mode = true;
            }
            "--schedule-table" => {
                i += 1;
                if i < args.len() {
                    cli.schedule_table = args[i].clone();
                }
            }
            "--roommap-table" => {
                i += 1;
                if i < args.len() {
                    cli.roommap_table = args[i].clone();
                }
            }
            "--prefix-table" => {
                i += 1;
                if i < args.len() {
                    cli.prefix_table = args[i].clone();
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            arg if !arg.starts_with('-') && cli.input.is_none() => {
                cli.input = Some(PathBuf::from(arg));
            }
            other => {
                eprintln!("Unknown argument: {other}");
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    if cli.input.is_some() && cli.output.is_some() {
        cli.headless = true;
    }

    cli
}

fn print_usage() {
    eprintln!(
        "Usage: cosam-editor [options] [<file.json|file.xlsx>]\n\
         \n\
         Options:\n\
         \x20 --input, -i <file>        Input file (.json or .xlsx)\n\
         \x20 --output, -o <file>       Output JSON file (enables headless convert mode)\n\
         \x20 --title, -t <string>      Event title (for XLSX import)\n\
         \x20 --staff                   Include staff/hidden events\n\
         \x20 --schedule-table <name>   Sheet name for schedule data (default: Schedule)\n\
         \x20 --roommap-table <name>    Sheet name for room mapping (default: RoomMap)\n\
         \x20 --prefix-table <name>     Sheet name for panel types (default: Prefix)\n\
         \x20 --help, -h                Show this help message\n\
         \n\
         Without --output, opens the GUI editor.\n\
         With both --input and --output, converts without opening the GUI."
    );
}

fn build_import_options(cli: &CliArgs) -> XlsxImportOptions {
    XlsxImportOptions {
        title: if cli.title.is_empty() {
            "Event Schedule".to_string()
        } else {
            cli.title.clone()
        },
        staff_mode: cli.staff_mode,
        schedule_sheet: cli.schedule_table.clone(),
        rooms_sheet: cli.roommap_table.clone(),
        panel_types_sheet: cli.prefix_table.clone(),
    }
}

fn resolve_input(cli: &CliArgs) -> Option<PathBuf> {
    if let Some(ref path) = cli.input {
        return Some(path.clone());
    }

    None
}

fn set_app_menus(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "App".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Hide cosam-editor", HideApp),
                MenuItem::action("Hide Others", HideOtherApps),
                MenuItem::action("Show All", ShowAllApps),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Window", NewWindow),
                MenuItem::separator(),
                MenuItem::action("Open...", FileOpen),
                MenuItem::action("Save", FileSave),
                MenuItem::action("Save As...", FileSaveAs),
                MenuItem::action("Export Public JSON...", FileExportPublicJson),
                MenuItem::separator(),
                MenuItem::action("Close Window", CloseWindow),
            ],
        },
    ]);
}

fn quit(_: &Quit, cx: &mut App) {
    cx.quit();
}

fn hide_app(_: &HideApp, cx: &mut App) {
    cx.hide();
}

fn hide_other_apps(_: &HideOtherApps, cx: &mut App) {
    cx.hide_other_apps();
}

fn show_all_apps(_: &ShowAllApps, cx: &mut App) {
    cx.unhide_other_apps();
}

fn close_window(_: &CloseWindow, cx: &mut App) {
    if let Some(active_window) = cx.active_window() {
        let _ = active_window.update(cx, |_, window, _cx| {
            window.remove_window();
        });
    }
}

fn open_editor_window(
    initial_schedule: Option<Schedule>,
    input_path: Option<PathBuf>,
    staff_mode: bool,
    cx: &mut App,
) -> anyhow::Result<()> {
    let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |window, cx| {
            let editor = cx.new(|cx| {
                ScheduleEditor::new(initial_schedule.clone(), input_path.clone(), staff_mode, cx)
            });
            window.focus(&editor.focus_handle(cx));
            editor
        },
    )?;
    Ok(())
}

fn main() {
    let cli = parse_args();
    let import_options = build_import_options(&cli);

    if cli.headless {
        let input = cli.input.as_ref().expect("headless mode requires --input");
        let output = cli
            .output
            .as_ref()
            .expect("headless mode requires --output");

        eprintln!("Reading: {}", input.display());
        let mut schedule = match Schedule::load_auto(input, &import_options) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error loading schedule: {e}");
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
            schedule.presenters.len(),
        );

        match schedule.save_json(output) {
            Ok(()) => eprintln!("Written: {}", output.display()),
            Err(e) => {
                eprintln!("Error saving: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    let input_path = resolve_input(&cli);
    let initial_schedule = match &input_path {
        Some(path) => match Schedule::load_auto(path, &import_options) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("Error loading schedule: {e}");
                std::process::exit(1);
            }
        },
        None => None,
    };

    let staff_mode = cli.staff_mode;

    Application::new().run(move |cx: &mut App| {
        // Register app-level handlers and keybindings
        cx.on_action(quit);
        cx.on_action(hide_app);
        cx.on_action(hide_other_apps);
        cx.on_action(show_all_apps);
        cx.on_action(close_window);

        let initial_schedule_for_new_window = initial_schedule.clone();
        let input_path_for_new_window = input_path.clone();
        cx.on_action(move |_: &NewWindow, cx| {
            let _ = open_editor_window(
                initial_schedule_for_new_window.clone(),
                input_path_for_new_window.clone(),
                staff_mode,
                cx,
            );
        });

        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.bind_keys([
            KeyBinding::new("cmd-h", HideApp, None),
            KeyBinding::new("cmd-alt-h", HideOtherApps, None),
            KeyBinding::new("cmd-n", NewWindow, None),
            KeyBinding::new("cmd-w", CloseWindow, None),
        ]);

        // Set up menus globally
        set_app_menus(cx);
        open_editor_window(initial_schedule.clone(), input_path.clone(), staff_mode, cx)
            .expect("Failed to open window");
    });
}
