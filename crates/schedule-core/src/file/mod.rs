/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

pub mod display_export;
pub mod full_export;
use std::path::Path;

use anyhow::{Context, Result};

use crate::data::schedule::Schedule;
use crate::edit::context::EditContext;
use crate::edit::history::EditHistory;

/// Combines a `Schedule` with its `EditHistory` for unified JSON persistence.
///
/// The JSON format is the schedule's normal fields plus an optional top-level
/// `"changeLog"` key (omitted when history is empty).  Format version is `8`
/// for full files; display files written by `display_export` stay at `7`.
pub struct ScheduleFile {
    pub schedule: Schedule,
    pub history: EditHistory,
}

impl ScheduleFile {
    /// Wrap a schedule with a fresh (empty) history.
    pub fn new(schedule: Schedule) -> Self {
        Self {
            schedule,
            history: EditHistory::new(),
        }
    }

    /// Load a JSON schedule file.  Any `"changeLog"` key in the file is
    /// deserialized into the history; all other keys are the schedule data.
    /// Uses FullSchedule for v10 format with proper relationship handling.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let raw: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON from {}", path.display()))?;

        let history = if let Some(cl) = raw.get("changeLog") {
            serde_json::from_value(cl.clone()).unwrap_or_else(|_| EditHistory::new())
        } else {
            EditHistory::new()
        };

        // Deserialize as FullSchedule (v10 format)
        let full_schedule: crate::file::full_export::FullSchedule = serde_json::from_value(raw)
            .with_context(|| {
                format!("Failed to deserialize FullSchedule from {}", path.display())
            })?;

        // Convert FullSchedule to Schedule with proper relationships
        let schedule = full_schedule.to_schedule().with_context(|| {
            format!(
                "Failed to convert FullSchedule to Schedule from {}",
                path.display()
            )
        })?;

        Ok(Self { schedule, history })
    }

    /// Borrow the schedule and history together as an `EditContext`.
    pub fn edit_context(&mut self) -> EditContext<'_> {
        EditContext::new(&mut self.schedule, &mut self.history)
    }

    /// Save to a JSON file using the schedule's save_json method.
    pub fn save_json(&mut self, path: &Path) -> Result<()> {
        self.schedule.save_json(path, &self.history)
    }
}

impl Clone for ScheduleFile {
    fn clone(&self) -> Self {
        Self {
            schedule: self.schedule.clone(),
            history: self.history.clone(),
        }
    }
}
