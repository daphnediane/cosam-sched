use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::event::Event;
use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::room::Room;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    pub generated: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
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
    pub events: Vec<Event>,
    pub rooms: Vec<Room>,
    pub panel_types: Vec<PanelType>,
    pub presenters: Vec<Presenter>,
}

impl Schedule {
    #[must_use]
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let schedule: Schedule = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON from {}", path.display()))?;
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

    pub fn save_json(&mut self, path: &Path) -> Result<()> {
        self.meta.generated = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        self.meta.version = Some(3);
        self.meta.generator = Some(format!("cosam-editor {}", env!("CARGO_PKG_VERSION")));
        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize schedule to JSON")?;
        std::fs::write(path, json.as_bytes())
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
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
}
