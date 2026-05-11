/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON data model deserialization.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Top-level schedule data, matching the widget JSON display format.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleData {
    pub meta: Meta,
    pub panels: Vec<Panel>,
    pub rooms: Vec<Room>,
    #[serde(default)]
    pub panel_types: HashMap<String, PanelType>,
    #[serde(default)]
    pub timeline: Vec<TimelineEntry>,
    #[serde(default)]
    pub presenters: Vec<Presenter>,
}

impl ScheduleData {
    /// Parse from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, ModelError> {
        Ok(serde_json::from_str(json)?)
    }

    /// Load from a JSON file.
    pub fn load(path: &std::path::Path) -> Result<Self, ModelError> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }

    /// Returns only non-break panels with a scheduled start time.
    pub fn scheduled_panels(&self) -> Vec<&Panel> {
        self.panels
            .iter()
            .filter(|p| {
                p.start_time.is_some()
                    && !self
                        .panel_types
                        .get(p.panel_type.as_deref().unwrap_or(""))
                        .map(|pt| pt.is_break)
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Returns break panels (implicit and overnight).
    pub fn break_panels(&self) -> Vec<&Panel> {
        self.panels
            .iter()
            .filter(|p| {
                self.panel_types
                    .get(p.panel_type.as_deref().unwrap_or(""))
                    .map(|pt| pt.is_break)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns rooms sorted by `sort_key`.
    pub fn sorted_rooms(&self) -> Vec<&Room> {
        let mut rooms: Vec<&Room> = self.rooms.iter().collect();
        rooms.sort_by_key(|r| r.sort_key);
        rooms
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub title: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub generator: String,
    #[serde(default)]
    pub generated: String,
    #[serde(default)]
    pub modified: String,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    pub id: String,
    #[serde(default)]
    pub base_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub panel_type: Option<String>,
    #[serde(default)]
    pub room_ids: Vec<i64>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub prereq: Option<String>,
    #[serde(default)]
    pub cost: Option<String>,
    #[serde(default)]
    pub capacity: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub ticket_url: Option<String>,
    #[serde(default)]
    pub is_premium: bool,
    #[serde(default)]
    pub is_full: bool,
    #[serde(default)]
    pub is_kids: bool,
    #[serde(default)]
    pub credits: Vec<String>,
    #[serde(default)]
    pub presenters: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub uid: i64,
    #[serde(default)]
    pub short_name: String,
    #[serde(default)]
    pub long_name: String,
    #[serde(default)]
    pub hotel_room: String,
    #[serde(default)]
    pub sort_key: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelType {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub colors: PanelTypeColors,
    #[serde(default)]
    pub is_break: bool,
    #[serde(default)]
    pub is_cafe: bool,
    #[serde(default)]
    pub is_workshop: bool,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub is_room_hours: bool,
    #[serde(default)]
    pub is_timeline: bool,
    #[serde(default)]
    pub is_private: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelTypeColors {
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub bw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntry {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub panel_type: Option<String>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Presenter {
    pub uid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub short_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_data_deserialize_minimal() {
        let json = r#"{
            "meta": {"title": "Test Schedule", "version": 0, "variant": "display", "generator": "test", "generated": "2026-01-01T00:00:00Z", "modified": "2026-01-01T00:00:00Z"},
            "panels": [],
            "rooms": [],
            "panelTypes": {}
        }"#;
        let data = ScheduleData::from_json(json).unwrap();
        assert_eq!(data.meta.title, "Test Schedule");
        assert!(data.panels.is_empty());
    }

    #[test]
    fn test_panel_type_colors_default() {
        let colors = PanelTypeColors::default();
        assert!(colors.color.is_none());
        assert!(colors.bw.is_none());
    }

    #[test]
    fn test_schedule_data_roundtrip() {
        let json = "{\"meta\":{\"title\":\"RT Test\",\"version\":0,\"variant\":\"display\",\"generator\":\"test\",\"generated\":\"2026-01-01T00:00:00Z\",\"modified\":\"2026-01-01T00:00:00Z\"},\"panels\":[{\"id\":\"GP001\",\"baseId\":\"GP001\",\"name\":\"Test Panel\",\"panelType\":\"GP\",\"roomIds\":[1],\"startTime\":\"2026-06-26T14:00:00\",\"endTime\":\"2026-06-26T15:00:00\",\"duration\":60,\"credits\":[],\"presenters\":[]}],\"rooms\":[{\"uid\":1,\"shortName\":\"Main\",\"longName\":\"Main Hall\",\"hotelRoom\":\"Ballroom A\",\"sortKey\":0}],\"panelTypes\":{\"GP\":{\"kind\":\"Guest Panel\",\"colors\":{\"color\":\"#E2F9D7\"},\"isBreak\":false,\"isCafe\":false,\"isWorkshop\":false,\"isHidden\":false,\"isRoomHours\":false,\"isTimeline\":false,\"isPrivate\":false}}}";
        let data = ScheduleData::from_json(json).unwrap();
        let re_json = serde_json::to_string(&data).unwrap();
        let data2 = ScheduleData::from_json(&re_json).unwrap();
        assert_eq!(data.meta.title, data2.meta.title);
        assert_eq!(data.panels.len(), data2.panels.len());
        assert_eq!(data.rooms.len(), data2.rooms.len());
    }
}
