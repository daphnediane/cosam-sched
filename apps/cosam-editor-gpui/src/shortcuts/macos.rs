/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::{App, KeyBinding};

pub(super) fn bind_platform_shortcuts(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-h", crate::HideApp, None),
        KeyBinding::new("cmd-alt-h", crate::HideOtherApps, None),
    ]);
}
