/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::{App, Menu, MenuItem};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod windows;
#[cfg(not(target_os = "macos"))]
pub use windows::WindowsMenuBar;

pub(super) fn edit_menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem::action("Undo", crate::EditUndo),
        MenuItem::action("Redo", crate::EditRedo),
    ]
}

pub(super) fn view_menu_items() -> Vec<MenuItem> {
    vec![MenuItem::action("List View", crate::ToggleListView)]
}

pub(super) fn help_menu_items() -> Vec<MenuItem> {
    vec![MenuItem::action("About cosam-editor", crate::AboutApp)]
}

fn file_menu_items(include_exit: bool) -> Vec<MenuItem> {
    let mut items = vec![
        MenuItem::action("New Window", crate::NewWindow),
        MenuItem::action("New Schedule", crate::NewSchedule),
        MenuItem::separator(),
        MenuItem::action("Open...", crate::FileOpen),
        MenuItem::action("Save", crate::FileSave),
        MenuItem::action("Save As...", crate::FileSaveAs),
        MenuItem::submenu(Menu {
            name: "Export".into(),
            items: vec![
                MenuItem::action("Public JSON...", crate::FileExportPublicJson),
                MenuItem::action("Embedded Widget...", crate::FileExportEmbed),
                MenuItem::action("Test page...", crate::FileExportTest),
            ],
        }),
        MenuItem::separator(),
        MenuItem::action("Close Window", crate::CloseWindow),
    ];

    if include_exit {
        items.push(MenuItem::separator());
        items.push(MenuItem::action("Exit", crate::Quit));
    }

    items
}

pub fn set_app_menus(cx: &mut App) {
    #[cfg(target_os = "macos")]
    let menus: Vec<Menu> = macos::menus();

    #[cfg(not(target_os = "macos"))]
    let menus: Vec<Menu> = windows::menus();

    cx.set_menus(menus);
}
