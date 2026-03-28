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
use crate::data::time;
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

        let mut schedule: Schedule = serde_json::from_value(raw)
            .with_context(|| format!("Failed to deserialize schedule from {}", path.display()))?;

        schedule.build_relationships_from_presenters();

        Ok(Self { schedule, history })
    }

    /// Save to a JSON file.
    ///
    /// - Stamps `meta.generated`, `meta.generator`, `meta.version = 10` (full).
    /// - Syncs presenter struct fields from `RelationshipManager`.
    /// - Calls `apply_schedule_parity` and `calculate_schedule_bounds`.
    /// - Appends `"changeLog"` when history is non-empty.
    pub fn save_json(&mut self, path: &Path) -> Result<()> {
        self.schedule.meta.generated = time::format_storage_ts(chrono::Utc::now());

        self.schedule.meta.version = Some(10);
        if self.schedule.meta.variant.is_none() {
            self.schedule.meta.variant = Some("full".to_string());
        }

        self.schedule.meta.generator = Some(format!("cosam-sched {}", env!("CARGO_PKG_VERSION")));

        // Sync presenter struct fields from RelationshipManager so that
        // changes made via AddRelationship/RemoveRelationship are serialized.
        self.schedule.sync_presenters_from_relationships();

        crate::data::post_process::apply_schedule_parity(&mut self.schedule);

        if self.schedule.meta.start_time.is_none() || self.schedule.meta.end_time.is_none() {
            self.schedule.calculate_schedule_bounds();
        }

        let mut obj =
            serde_json::to_value(&self.schedule).context("Failed to serialize schedule to JSON")?;

        if !self.history.is_empty() {
            let cl =
                serde_json::to_value(&self.history).context("Failed to serialize change log")?;
            if let Some(map) = obj.as_object_mut() {
                map.insert("changeLog".to_string(), cl);
            }
        }

        let json = serde_json::to_string_pretty(&obj).context("Failed to format JSON")?;
        std::fs::write(path, json.as_bytes())
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }

    /// Borrow the schedule and history together as an `EditContext`.
    pub fn edit_context(&mut self) -> EditContext<'_> {
        EditContext::new(&mut self.schedule, &mut self.history)
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
