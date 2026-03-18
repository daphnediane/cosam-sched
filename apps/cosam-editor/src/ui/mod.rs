/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::Global;

pub mod day_tabs;
pub mod editor;
pub mod event_card;
pub mod sidebar;

pub use editor::ScheduleEditor;

#[derive(PartialEq)]
pub struct MenuState {
    pub schedule_loaded: bool,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            schedule_loaded: false,
        }
    }
}

impl Global for MenuState {}
