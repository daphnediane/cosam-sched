/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::BTreeSet;

use chrono::NaiveDate;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::panel::Panel;
use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::room::Room;
use super::source_info::{ChangeState, ImportedSheetPresence};
use super::time;
use super::timeline::TimelineEntry;

/// Lightweight struct for displaying a panel session in the editor UI
#[derive(Debug, Clone)]
pub struct SessionDisplayInfo {
    pub session_id: String,
    pub base_id: String,
    pub name: String,
    pub panel_type: Option<String>,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub room_ids: Vec<u32>,
    pub presenters: Vec<String>,
    pub is_full: bool,
    pub change_state: ChangeState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    pub generated: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_presenter_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflictEventRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleConflict {
    pub event1: ConflictEventRef,
    pub event2: ConflictEventRef,
    #[serde(default)]
    pub presenter: Option<String>,
    #[serde(default)]
    pub room: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub conflict_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<ScheduleConflict>,
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timeline: Vec<TimelineEntry>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub panels: IndexMap<String, Panel>,
    pub rooms: Vec<Room>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub panel_types: IndexMap<String, PanelType>,
    pub presenters: Vec<Presenter>,
    #[serde(default, skip_serializing)]
    pub imported_sheets: ImportedSheetPresence,
}

impl Schedule {
    /// Calculate schedule start and end times from panels and timeline entries
    pub fn calculate_schedule_bounds(&mut self) {
        let mut min_time: Option<chrono::NaiveDateTime> = None;
        let mut max_time: Option<chrono::NaiveDateTime> = None;

        // Check panel sessions
        for panel in self.panels.values() {
            for part in &panel.parts {
                for session in &part.sessions {
                    if let Some(ref st) = session.start_time {
                        if let Some(start) = time::parse_storage(st) {
                            if min_time.is_none() || Some(start) < min_time {
                                min_time = Some(start);
                            }
                            let end = if let Some(ref et) = session.end_time {
                                time::parse_storage(et).unwrap_or(
                                    start + chrono::Duration::minutes(session.duration as i64),
                                )
                            } else {
                                start + chrono::Duration::minutes(session.duration as i64)
                            };
                            if max_time.is_none() || Some(end) > max_time {
                                max_time = Some(end);
                            }
                        }
                    }
                }
            }
        }

        // Check timeline entries
        for timeline_entry in &self.timeline {
            if let Some(start_time) = time::parse_storage(&timeline_entry.start_time) {
                if min_time.is_none() || Some(start_time) < min_time {
                    min_time = Some(start_time);
                }
                let end_time = start_time + chrono::Duration::minutes(30);
                if max_time.is_none() || Some(end_time) > max_time {
                    max_time = Some(end_time);
                }
            }
        }

        // Set meta fields
        if let Some(min_time) = min_time {
            self.meta.start_time = Some(time::format_storage_ts(min_time.and_utc()));
        }
        if let Some(max_time) = max_time {
            self.meta.end_time = Some(time::format_storage_ts(max_time.and_utc()));
        }

        // If still no times found, set reasonable defaults for Cosplay America
        // @todo assume the last weekend of the upcoming June.
        if self.meta.start_time.is_none() {
            self.meta.start_time = Some("2026-06-25T17:00:00Z".to_string()); // Thursday evening
        }
        if self.meta.end_time.is_none() {
            self.meta.end_time = Some("2026-06-28T18:00:00Z".to_string()); // Sunday evening
        }
    }

    #[must_use]
    pub fn days(&self) -> Vec<NaiveDate> {
        let mut dates: BTreeSet<NaiveDate> = BTreeSet::new();

        for panel in self.panels.values() {
            for part in &panel.parts {
                for session in &part.sessions {
                    if let Some(ref st) = session.start_time {
                        if let Some(dt) = time::parse_storage(st) {
                            dates.insert(dt.date());
                        }
                    }
                }
            }
        }

        dates.into_iter().collect()
    }

    /// Returns flattened session display info for a given day from the v5 panels hierarchy
    #[must_use]
    pub fn sessions_for_day(&self, day: &NaiveDate) -> Vec<SessionDisplayInfo> {
        let mut results = Vec::new();
        for panel in self.panels.values() {
            for part in &panel.parts {
                for session in &part.sessions {
                    let start_dt = session
                        .start_time
                        .as_ref()
                        .and_then(|st| time::parse_storage(st));
                    if let Some(start) = start_dt {
                        if &start.date() == day {
                            let end_dt = session
                                .end_time
                                .as_ref()
                                .and_then(|et| time::parse_storage(et))
                                .unwrap_or(
                                    start + chrono::Duration::minutes(session.duration as i64),
                                );

                            let presenters: Vec<String> = {
                                let mut all = panel.credited_presenters.clone();
                                for name in &part.credited_presenters {
                                    if !all.contains(name) {
                                        all.push(name.clone());
                                    }
                                }
                                for name in &session.credited_presenters {
                                    if !all.contains(name) {
                                        all.push(name.clone());
                                    }
                                }
                                all
                            };

                            results.push(SessionDisplayInfo {
                                session_id: session.id.clone(),
                                base_id: panel.id.clone(),
                                name: panel.name.clone(),
                                panel_type: panel.panel_type.clone(),
                                start_time: start,
                                end_time: end_dt,
                                room_ids: session.room_ids.clone(),
                                presenters,
                                is_full: session.is_full,
                                change_state: session.change_state,
                            });
                        }
                    }
                }
            }
        }
        results.sort_by_key(|s| s.start_time);
        results
    }

    #[must_use]
    pub fn room_by_id(&self, uid: u32) -> Option<&Room> {
        self.rooms.iter().find(|r| r.uid == uid)
    }

    #[must_use]
    pub fn sorted_rooms(&self) -> Vec<&Room> {
        let mut rooms: Vec<&Room> = self.rooms.iter().collect();
        rooms.sort_by_key(|r| r.sort_key);
        rooms
    }

    #[must_use]
    pub fn panel_type_by_prefix(&self, prefix: &str) -> Option<&PanelType> {
        self.panel_types.get(prefix)
    }

    pub fn populate_panel_type_prefixes(&mut self) {
        for (prefix, panel_type) in &mut self.panel_types {
            panel_type.prefix = prefix.clone();
        }
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            conflicts: Vec::new(),
            meta: Meta::default(),
            timeline: Vec::new(),
            panels: IndexMap::new(),
            rooms: Vec::new(),
            panel_types: IndexMap::new(),
            presenters: Vec::new(),
            imported_sheets: ImportedSheetPresence::default(),
        }
    }
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            title: "Event Schedule".to_string(),
            generated: time::format_storage_ts(chrono::Utc::now()),
            version: Some(7),
            variant: None,
            generator: Some("cosam-sched".to_string()),
            start_time: None,
            end_time: None,
            next_presenter_id: None,
            creator: None,
            last_modified_by: None,
            modified: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_json() -> &'static str {
        r#"{"meta":{"title":"Test","generated":"2026-01-01T00:00:00Z"},"rooms":[],"presenters":[]}"#
    }

    #[test]
    fn test_roundtrip_serialization() {
        let schedule: Schedule = serde_json::from_str(make_minimal_json()).unwrap();
        let json = serde_json::to_string_pretty(&schedule).unwrap();
        let schedule2: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(schedule, schedule2);
    }

    #[test]
    fn test_unknown_fields_ignored() {
        let json = r#"{"meta":{"title":"T","generated":"2026-01-01T00:00:00Z"},"rooms":[],"presenters":[],"changeLog":{"undoStack":[],"redoStack":[],"maxDepth":50}}"#;
        let schedule: Schedule = serde_json::from_str(json).unwrap();
        assert_eq!(schedule.meta.title, "T");
    }

    #[test]
    fn test_malformed_json_fails() {
        let result: Result<Schedule, _> = serde_json::from_str("{ not valid json }");
        assert!(result.is_err());
    }
}
