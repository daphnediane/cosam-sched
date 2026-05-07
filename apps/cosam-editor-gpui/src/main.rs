/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::PathBuf;

mod menu;
mod shortcuts;
mod ui;

use gpui::prelude::*;
use gpui::{
    actions, px, size, App, Application, Bounds, Focusable, TitlebarOptions, WindowBounds,
    WindowOptions,
};
use gpui_component::Root;
use schedule_core::edit::context::EditContext;
use schedule_core::schedule::{Schedule, FILE_MAGIC};
use schedule_core::xlsx::{import_xlsx, XlsxImportOptions};
use ui::editor::{
    EditRedo, EditUndo, FileExportEmbed, FileExportPublicJson, FileExportTest, FileSave, FileSaveAs,
};
use ui::ScheduleEditor;

actions!(
    main,
    [
        Quit,
        HideApp,
        HideOtherApps,
        ShowAllApps,
        NewWindow,
        CloseWindow,
        FileOpen,
        NewSchedule,
        ToggleListView,
        AboutApp,
    ]
);

fn load_schedule(path: &PathBuf) -> anyhow::Result<Schedule> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(FILE_MAGIC) {
        Schedule::load_from_file(&bytes).map_err(|e| anyhow::anyhow!("{e}"))
    } else {
        import_xlsx(path, &XlsxImportOptions::default()).map_err(|e| anyhow::anyhow!("{e}"))
    }
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

fn file_open(_: &FileOpen, cx: &mut App) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Schedule files", &["cosam", "xlsx"])
        .add_filter("cosam schedule", &["cosam"])
        .add_filter("Excel Workbook", &["xlsx"])
        .add_filter("All files", &["*"])
        .pick_file()
    else {
        return;
    };

    match load_schedule(&path) {
        Ok(schedule) => {
            let ctx = EditContext::new(schedule);
            let _ = open_editor_window(Some(ctx), Some(path), cx);
        }
        Err(e) => {
            eprintln!("Error loading file: {e}");
        }
    }
}

fn new_schedule(_: &NewSchedule, cx: &mut App) {
    let ctx = EditContext::new(Schedule::new());
    let _ = open_editor_window(Some(ctx), None, cx);
}

fn open_editor_window(
    ctx: Option<EditContext>,
    path: Option<PathBuf>,
    cx: &mut App,
) -> anyhow::Result<()> {
    let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
    let mut window_options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
    };

    if cfg!(target_os = "windows") {
        window_options.titlebar = Some(TitlebarOptions {
            appears_transparent: false,
            ..Default::default()
        });
    }

    cx.open_window(window_options, move |window, cx| {
        let editor = cx.new(|cx| ScheduleEditor::new(ctx, path, cx));
        window.focus(&editor.focus_handle(cx));
        cx.new(|cx| Root::new(editor, window, cx))
    })?;
    Ok(())
}

fn main() {
    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);

        cx.on_action(quit);
        cx.on_action(hide_app);
        cx.on_action(hide_other_apps);
        cx.on_action(show_all_apps);
        cx.on_action(close_window);
        cx.on_action(file_open);
        cx.on_action(new_schedule);
        cx.on_action(|_: &NewWindow, cx| {
            let _ = open_editor_window(None, None, cx);
        });
        cx.on_action(|_: &ToggleListView, _cx| {});
        cx.on_action(|_: &AboutApp, _cx| {});

        shortcuts::bind_app_shortcuts(cx);
        menu::set_app_menus(cx);

        cx.dispatch_action(&FileOpen);
        cx.activate(true);
    });
}
