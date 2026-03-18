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

fn file_menu_items(include_exit: bool) -> Vec<MenuItem> {
    let mut file_items = vec![
        MenuItem::action("New Window", crate::NewWindow),
        MenuItem::separator(),
        MenuItem::action("Open...", crate::FileOpen),
        MenuItem::action("Save", crate::FileSave),
        MenuItem::action("Save As...", crate::FileSaveAs),
        MenuItem::action("Export Public JSON...", crate::FileExportPublicJson),
        MenuItem::separator(),
        MenuItem::action("Close Window", crate::CloseWindow),
    ];

    if include_exit {
        file_items.push(MenuItem::separator());
        file_items.push(MenuItem::action("Exit", crate::Quit));
    }

    file_items
}

pub fn set_app_menus(cx: &mut App) {
    #[cfg(target_os = "macos")]
    let menus: Vec<Menu> = macos::menus();

    #[cfg(not(target_os = "macos"))]
    let menus: Vec<Menu> = windows::menus();

    cx.set_menus(menus);
}
