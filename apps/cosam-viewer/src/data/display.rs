/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON display types, re-exported from schedule-core with convenience wrapper.

use schedule_core::widget_json::{load_from_json, WidgetExport};
use std::ops::Deref;

pub use schedule_core::widget_json::{WidgetPanelType, WidgetRoom};

// ---------------------------------------------------------------------------
// ScheduleDoc wrapper with helper methods
// ---------------------------------------------------------------------------

/// Wrapper around WidgetExport with viewer-specific helper methods.
#[derive(Debug, Clone)]
pub struct ScheduleDoc(pub WidgetExport);

impl Deref for ScheduleDoc {
    type Target = WidgetExport;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ScheduleDoc {
    /// Load from a JSON byte slice.
    pub fn from_json(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self(load_from_json(std::str::from_utf8(bytes)?)?))
    }

    /// Look up a room by uid.
    pub fn room_by_uid(&self, uid: i32) -> Option<&WidgetRoom> {
        self.0.rooms.iter().find(|r| r.uid == uid)
    }

    /// Return all non-break, non-hidden panel types sorted by kind key.
    pub fn visible_types(&self) -> Vec<(&String, &WidgetPanelType)> {
        let mut types: Vec<_> = self
            .panel_types
            .iter()
            .filter(|(_, pt)| !pt.is_hidden && !pt.is_break && !pt.is_timeline)
            .collect();
        types.sort_by_key(|(k, _)| k.as_str());
        types
    }

    /// Return all non-break rooms sorted by sort_key then long_name.
    pub fn visible_rooms(&self) -> Vec<&WidgetRoom> {
        let mut rooms: Vec<_> = self.rooms.iter().filter(|r| !r.is_break).collect();
        rooms.sort_by(|a, b| {
            a.sort_key
                .cmp(&b.sort_key)
                .then_with(|| a.long_name.cmp(&b.long_name))
        });
        rooms
    }
}
