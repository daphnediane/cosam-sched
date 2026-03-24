/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::{Menu, MenuItem, SystemMenuType};

pub(super) fn menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "App".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Hide cosam-editor", crate::HideApp),
                MenuItem::action("Hide Others", crate::HideOtherApps),
                MenuItem::action("Show All", crate::ShowAllApps),
                MenuItem::separator(),
                MenuItem::action("Quit", crate::Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: super::file_menu_items(false),
        },
        Menu {
            name: "Edit".into(),
            items: super::edit_menu_items(),
        },
    ]
}
