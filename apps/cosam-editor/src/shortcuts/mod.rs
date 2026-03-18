/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::{App, KeyBinding};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod windows;

pub fn bind_app_shortcuts(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-o", crate::FileOpen, None),
        KeyBinding::new("secondary-s", crate::FileSave, None),
        KeyBinding::new("secondary-shift-s", crate::FileSaveAs, None),
        KeyBinding::new("secondary-shift-e", crate::FileExportPublicJson, None),
        KeyBinding::new("secondary-n", crate::NewWindow, None),
        KeyBinding::new("secondary-w", crate::CloseWindow, None),
        KeyBinding::new("secondary-q", crate::Quit, None),
    ]);

    #[cfg(target_os = "macos")]
    macos::bind_platform_shortcuts(cx);

    #[cfg(not(target_os = "macos"))]
    windows::bind_platform_shortcuts(cx);
}
