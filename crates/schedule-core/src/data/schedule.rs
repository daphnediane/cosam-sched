/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::event::Event;
use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::room::Room;
use super::source_info::{ChangeState, ImportedSheetPresence};
use super::timeline::{TimeType, TimelineEntry};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    pub generated: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
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
    pub events: Vec<Event>,
    pub rooms: Vec<Room>,
    pub panel_types: Vec<PanelType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub time_types: Vec<TimeType>,
    pub presenters: Vec<Presenter>,
    #[serde(default, skip_serializing)]
    pub imported_sheets: ImportedSheetPresence,
}

impl Schedule {
    #[must_use]
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let mut schedule: Schedule = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON from {}", path.display()))?;

        // Auto-migrate to v4 if needed
        schedule.migrate_to_v4();

        Ok(schedule)
    }

    pub fn load_auto(path: &Path, options: &super::xlsx_import::XlsxImportOptions) -> Result<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("xlsx") => {
                super::xlsx_import::import_xlsx(path, options)
            }
            Some(ext) if ext.eq_ignore_ascii_case("json") => Self::load(path),
            Some(ext) => anyhow::bail!("Unsupported file format: .{ext}"),
            None => Self::load(path),
        }
    }

    pub fn save_json(&self, path: &Path) -> Result<()> {
        self.clone().save_json_to_file(path)
    }

    pub fn save_json_with_mode(&self, path: &Path, mode: super::JsonExportMode) -> Result<()> {
        // Get the schedule to save (filtered if needed)
        let mut schedule_to_save = if mode == super::JsonExportMode::Public {
            self.filter_for_public_export()
        } else {
            self.clone()
        };
        
        // Save the filtered/unfiltered schedule
        schedule_to_save.save_json_to_file(path)
    }
    
    fn save_json_to_file(&mut self, path: &Path) -> Result<()> {
        self.meta.generated = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        self.meta.version = Some(4);
        self.meta.generator = Some(format!("cosam-editor {}", env!("CARGO_PKG_VERSION")));

        super::post_process::apply_schedule_parity(self);

        // Calculate start and end times if not present
        if self.meta.start_time.is_none() || self.meta.end_time.is_none() {
            self.calculate_schedule_bounds();
        }

        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize schedule to JSON")?;
        std::fs::write(path, json.as_bytes())
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }

    fn filter_for_public_export(&self) -> Schedule {
        let mut schedule = self.clone();

        let hidden_type_uids: std::collections::HashSet<String> = schedule
            .panel_types
            .iter()
            .filter(|panel_type| panel_type.is_hidden)
            .map(|panel_type| panel_type.effective_uid())
            .collect();

        if !hidden_type_uids.is_empty() {
            schedule.events.retain(|event| {
                event
                    .panel_type
                    .as_ref()
                    .map(|panel_type_uid| !hidden_type_uids.contains(panel_type_uid))
                    .unwrap_or(true)
            });
            schedule.panel_types.retain(|panel_type| {
                !panel_type.is_hidden
            });
        }

        schedule
    }

    /// Calculate schedule start and end times from events and timeline entries
    pub fn calculate_schedule_bounds(&mut self) {
        let mut min_time: Option<chrono::NaiveDateTime> = None;
        let mut max_time: Option<chrono::NaiveDateTime> = None;

        // Check events
        for event in &self.events {
            if min_time.is_none() || Some(event.start_time) < min_time {
                min_time = Some(event.start_time);
            }
            if max_time.is_none() || Some(event.end_time) > max_time {
                max_time = Some(event.end_time);
            }
        }

        // Check timeline entries
        for timeline_entry in &self.timeline {
            if let Ok(start_time) = chrono::NaiveDateTime::parse_from_str(
                &timeline_entry.start_time,
                "%Y-%m-%dT%H:%M:%S",
            ) {
                if min_time.is_none() || Some(start_time) < min_time {
                    min_time = Some(start_time);
                }
                // Timeline entries have implicit 30-minute duration
                let end_time = start_time + chrono::Duration::minutes(30);
                if max_time.is_none() || Some(end_time) > max_time {
                    max_time = Some(end_time);
                }
            }
        }

        // Set meta fields
        if let Some(min_time) = min_time {
            self.meta.start_time = Some(min_time.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
        if let Some(max_time) = max_time {
            self.meta.end_time = Some(max_time.format("%Y-%m-%dT%H:%M:%SZ").to_string());
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

    /// Migrate v3 format to v4 format
    pub fn migrate_to_v4(&mut self) {
        if self.meta.version.unwrap_or(1) >= 4 {
            return; // Already v4 or higher
        }

        // Convert split panel types to time types and timeline
        let (panel_types, time_types, timeline) = self.convert_split_panel_types();

        // Update schedule
        self.panel_types = panel_types;
        self.time_types = time_types;
        self.timeline = timeline;

        // Calculate schedule bounds
        self.calculate_schedule_bounds();

        // Update version
        self.meta.version = Some(4);
    }

    /// Convert split panel types to time types and timeline entries
    fn convert_split_panel_types(
        &self,
    ) -> (
        Vec<super::panel_type::PanelType>,
        Vec<super::timeline::TimeType>,
        Vec<super::timeline::TimelineEntry>,
    ) {
        let mut time_types = Vec::new();
        let mut timeline = Vec::new();
        let mut filtered_panel_types = Vec::new();

        // Find split panel types and convert them
        for panel_type in &self.panel_types {
            if panel_type.prefix.to_uppercase() == "SPLIT"
                || panel_type.prefix.to_uppercase().starts_with("SP")
                || panel_type.prefix.to_uppercase().starts_with("SPLIT")
            {
                // Create time type
                let time_type = super::timeline::TimeType {
                    uid: super::timeline::TimeType::uid_from_prefix(&panel_type.prefix),
                    prefix: panel_type.prefix.clone(),
                    kind: panel_type.kind.clone(),
                    source: None,
                    change_state: ChangeState::Converted,
                };
                time_types.push(time_type);

                // Find events with this panel type and convert to timeline entries
                let split_events: Vec<_> = self
                    .events
                    .iter()
                    .filter(|e| {
                        e.panel_type
                            .as_ref()
                            .map(|pt| pt == &panel_type.effective_uid())
                            .unwrap_or(false)
                    })
                    .collect();

                for (i, event) in split_events.iter().enumerate() {
                    let timeline_entry = super::timeline::TimelineEntry {
                        id: format!("{}{:02}", panel_type.prefix, i + 1),
                        start_time: event.start_time.format("%Y-%m-%dT%H:%M:%S").to_string(),
                        description: event.name.clone(),
                        time_type: Some(super::timeline::TimeType::uid_from_prefix(
                            &panel_type.prefix,
                        )),
                        note: event.note.clone(),
                        source: None,
                        change_state: ChangeState::Converted,
                    };
                    timeline.push(timeline_entry);
                }
            } else {
                // Keep non-split panel types
                filtered_panel_types.push(panel_type.clone());
            }
        }

        (filtered_panel_types, time_types, timeline)
    }

    #[must_use]
    pub fn days(&self) -> Vec<NaiveDate> {
        let dates: BTreeSet<NaiveDate> = self.events.iter().map(|e| e.date()).collect();
        dates.into_iter().collect()
    }

    #[must_use]
    pub fn events_for_day(&self, day: &NaiveDate) -> Vec<&Event> {
        self.events.iter().filter(|e| &e.date() == day).collect()
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
        self.panel_types.iter().find(|pt| pt.prefix == prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn reference_data_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("widget")
            .join("2025.json")
    }

    #[test]
    fn test_load_reference_data() {
        let path = reference_data_path();
        if !path.exists() {
            eprintln!("Skipping test: {} not found", path.display());
            return;
        }
        let schedule = Schedule::load(&path).expect("Failed to load 2025.json");
        assert!(!schedule.events.is_empty());
        assert!(!schedule.rooms.is_empty());
        assert!(!schedule.presenters.is_empty());
        assert!(!schedule.meta.title.is_empty());
    }

    #[test]
    fn test_days_extraction() {
        let path = reference_data_path();
        if !path.exists() {
            return;
        }
        let schedule = Schedule::load(&path).unwrap();
        let days = schedule.days();
        assert!(!days.is_empty());
        // Days should be sorted
        for window in days.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn test_events_for_day() {
        let path = reference_data_path();
        if !path.exists() {
            return;
        }
        let schedule = Schedule::load(&path).unwrap();
        let days = schedule.days();
        for day in &days {
            let day_events = schedule.events_for_day(day);
            assert!(
                !day_events.is_empty(),
                "Day {day} should have at least one event"
            );
            for event in &day_events {
                assert_eq!(&event.date(), day);
            }
        }
    }

    #[test]
    fn test_room_by_id() {
        let path = reference_data_path();
        if !path.exists() {
            return;
        }
        let schedule = Schedule::load(&path).unwrap();
        assert!(!schedule.rooms.is_empty());
        let first_room = &schedule.rooms[0];
        let found = schedule.room_by_id(first_room.uid);
        assert!(found.is_some());
        assert_eq!(schedule.room_by_id(99999), None);
    }

    #[test]
    fn test_sorted_rooms() {
        let path = reference_data_path();
        if !path.exists() {
            return;
        }
        let schedule = Schedule::load(&path).unwrap();
        let rooms = schedule.sorted_rooms();
        for window in rooms.windows(2) {
            assert!(window[0].sort_key <= window[1].sort_key);
        }
    }

    #[test]
    fn test_panel_type_by_prefix() {
        let path = reference_data_path();
        if !path.exists() {
            return;
        }
        let schedule = Schedule::load(&path).unwrap();
        assert_eq!(schedule.panel_type_by_prefix("NONEXISTENT"), None);
    }

    #[test]
    fn test_roundtrip_serialization() {
        let path = reference_data_path();
        if !path.exists() {
            return;
        }
        let schedule = Schedule::load(&path).unwrap();
        let json = serde_json::to_string_pretty(&schedule).unwrap();
        let schedule2: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(schedule, schedule2);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = Schedule::load(Path::new("/nonexistent/path.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_malformed_json() {
        let dir = std::env::temp_dir();
        let path = dir.join("cosam_test_malformed.json");
        std::fs::write(&path, "{ not valid json }").unwrap();
        let result = Schedule::load(&path);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_json_export_mode_filtering() {
        let mut schedule = Schedule {
            conflicts: Vec::new(),
            meta: super::Meta {
                title: "Test Schedule".to_string(),
                generated: "2024-01-01T00:00:00Z".to_string(),
                version: Some(2),
                generator: None,
                start_time: None,
                end_time: None,
            },
            timeline: Vec::new(),
            events: Vec::new(),
            rooms: Vec::new(),
            panel_types: Vec::new(),
            time_types: Vec::new(),
            presenters: Vec::new(),
            imported_sheets: Default::default(),
        };
        
        // Add some panel types
        schedule.panel_types.push(super::PanelType {
            uid: Some("panel-type-public".to_string()),
            prefix: "PUB".to_string(),
            kind: "Public".to_string(),
            is_hidden: false,
            color: None,
            is_break: false,
            is_cafe: false,
            is_workshop: false,
            is_room_hours: false,
            bw_color: None,
            source: None,
            change_state: Default::default(),
        });
        
        schedule.panel_types.push(super::PanelType {
            uid: Some("panel-type-hidden".to_string()),
            prefix: "HID".to_string(),
            kind: "Hidden".to_string(),
            is_hidden: true,
            color: None,
            is_break: false,
            is_cafe: false,
            is_workshop: false,
            is_room_hours: false,
            bw_color: None,
            source: None,
            change_state: Default::default(),
        });
        
        schedule.panel_types.push(super::PanelType {
            uid: Some("panel-type-split".to_string()),
            prefix: "SPLIT".to_string(),
            kind: "Split".to_string(),
            is_hidden: true, // Even though hidden, splits should be handled normally
            color: None,
            is_break: false,
            is_cafe: false,
            is_workshop: false,
            is_room_hours: false,
            bw_color: None,
            source: None,
            change_state: Default::default(),
        });
        
        // Add events for each panel type
        let base_time = chrono::NaiveDateTime::parse_from_str("2024-01-01T10:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        
        schedule.events.push(super::Event {
            id: "event-public".to_string(),
            name: "Public Event".to_string(),
            description: None,
            start_time: base_time,
            end_time: base_time + chrono::Duration::hours(1),
            duration: 60,
            room_id: None,
            panel_type: Some("panel-type-public".to_string()),
            cost: None,
            capacity: None,
            difficulty: None,
            note: None,
            prereq: None,
            ticket_url: None,
            presenters: Vec::new(),
            credits: Vec::new(),
            conflicts: Vec::new(),
            is_free: false,
            is_full: false,
            is_kids: false,
            hide_panelist: false,
            alt_panelist: None,
            source: None,
            change_state: Default::default(),
        });
        
        schedule.events.push(super::Event {
            id: "event-hidden".to_string(),
            name: "Hidden Event".to_string(),
            description: None,
            start_time: base_time + chrono::Duration::hours(2),
            end_time: base_time + chrono::Duration::hours(3),
            duration: 60,
            room_id: None,
            panel_type: Some("panel-type-hidden".to_string()),
            cost: None,
            capacity: None,
            difficulty: None,
            note: None,
            prereq: None,
            ticket_url: None,
            presenters: Vec::new(),
            credits: Vec::new(),
            conflicts: Vec::new(),
            is_free: false,
            is_full: false,
            is_kids: false,
            hide_panelist: false,
            alt_panelist: None,
            source: None,
            change_state: Default::default(),
        });
        
        schedule.events.push(super::Event {
            id: "event-split".to_string(),
            name: "Split Event".to_string(),
            description: None,
            start_time: base_time + chrono::Duration::hours(4),
            end_time: base_time + chrono::Duration::hours(5),
            duration: 60,
            room_id: None,
            panel_type: Some("panel-type-split".to_string()),
            cost: None,
            capacity: None,
            difficulty: None,
            note: None,
            prereq: None,
            ticket_url: None,
            presenters: Vec::new(),
            credits: Vec::new(),
            conflicts: Vec::new(),
            is_free: false,
            is_full: false,
            is_kids: false,
            hide_panelist: false,
            alt_panelist: None,
            source: None,
            change_state: Default::default(),
        });
        
        // Test filtering directly
        let filtered_schedule = schedule.filter_for_public_export();
        assert_eq!(filtered_schedule.panel_types.len(), 1); // Only public panel type
        assert_eq!(filtered_schedule.events.len(), 1); // Only event with public panel type
        
        // Verify the correct panel type remains
        assert_eq!(filtered_schedule.panel_types[0].uid, Some("panel-type-public".to_string()));
        assert_eq!(filtered_schedule.events[0].panel_type, Some("panel-type-public".to_string()));
        
        // Test that hidden panel types are filtered out
        assert!(!filtered_schedule.panel_types.iter().any(|pt| pt.is_hidden));
        assert!(!filtered_schedule.events.iter().any(|e| {
            e.panel_type.as_ref().map_or(false, |uid| {
                uid == "panel-type-hidden" || uid == "panel-type-split"
            })
        }));
    }
}
