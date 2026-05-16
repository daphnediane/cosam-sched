/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use dioxus_desktop::muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

// Menu item IDs — matched in the event handler
pub const ID_FILE_NEW_WINDOW: &str = "file.new_window";
pub const ID_FILE_NEW_SCHEDULE: &str = "file.new_schedule";
pub const ID_FILE_OPEN: &str = "file.open";
pub const ID_FILE_SAVE: &str = "file.save";
pub const ID_FILE_SAVE_AS: &str = "file.save_as";
pub const ID_FILE_EXPORT_JSON: &str = "file.export_json";
pub const ID_FILE_EXPORT_EMBED: &str = "file.export_embed";
pub const ID_FILE_EXPORT_TEST: &str = "file.export_test";
pub const ID_FILE_CLOSE: &str = "file.close";
pub const ID_EDIT_UNDO: &str = "edit.undo";
pub const ID_EDIT_REDO: &str = "edit.redo";
pub const ID_VIEW_LIST: &str = "view.list";
pub const ID_HELP_ABOUT: &str = "help.about";

pub fn build_app_menu() -> Menu {
    let menu = Menu::new();

    // File menu
    let file_menu = Submenu::new("File", true);
    file_menu
        .append_items(&[
            &MenuItem::with_id(ID_FILE_NEW_WINDOW, "New Window", true, None),
            &MenuItem::with_id(ID_FILE_NEW_SCHEDULE, "New Schedule", true, None),
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(ID_FILE_OPEN, "Open...", true, None),
            &MenuItem::with_id(ID_FILE_SAVE, "Save", true, None),
            &MenuItem::with_id(ID_FILE_SAVE_AS, "Save As...", true, None),
        ])
        .ok();

    let export_menu = Submenu::new("Export", true);
    export_menu
        .append_items(&[
            &MenuItem::with_id(ID_FILE_EXPORT_JSON, "Public JSON...", true, None),
            &MenuItem::with_id(ID_FILE_EXPORT_EMBED, "Embedded Widget...", true, None),
            &MenuItem::with_id(ID_FILE_EXPORT_TEST, "Test page...", true, None),
        ])
        .ok();
    file_menu.append(&export_menu).ok();

    file_menu.append(&PredefinedMenuItem::separator()).ok();
    file_menu
        .append(&MenuItem::with_id(
            ID_FILE_CLOSE,
            "Close Window",
            true,
            None,
        ))
        .ok();

    // On Windows/Linux add Quit to File menu
    #[cfg(not(target_os = "macos"))]
    {
        file_menu.append(&PredefinedMenuItem::separator()).ok();
        file_menu
            .append(&PredefinedMenuItem::quit(Some("Exit")))
            .ok();
    }

    // Edit menu
    let edit_menu = Submenu::new("Edit", true);
    edit_menu
        .append_items(&[
            &MenuItem::with_id(ID_EDIT_UNDO, "Undo", true, None),
            &MenuItem::with_id(ID_EDIT_REDO, "Redo", true, None),
        ])
        .ok();

    // View menu
    let view_menu = Submenu::new("View", true);
    view_menu
        .append(&MenuItem::with_id(ID_VIEW_LIST, "List View", true, None))
        .ok();

    // Help menu
    let help_menu = Submenu::new("Help", true);
    help_menu
        .append(&MenuItem::with_id(
            ID_HELP_ABOUT,
            "About cosam-editor",
            true,
            None,
        ))
        .ok();

    // macOS app menu (Hide, Quit, etc.) handled by PredefinedMenuItem
    #[cfg(target_os = "macos")]
    {
        let app_menu = Submenu::new("App", true);
        app_menu
            .append_items(&[
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ])
            .ok();
        menu.append(&app_menu).ok();
    }

    menu.append_items(&[&file_menu, &edit_menu, &view_menu, &help_menu])
        .ok();

    menu
}
