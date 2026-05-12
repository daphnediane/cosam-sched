/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule data model for layout generation.
//!
//! [`ScheduleData`] can be built in-process from a
//! [`schedule_core::schedule::Schedule`] via [`ScheduleData::from_schedule`],
//! or deserialized from a widget-JSON file via [`ScheduleData::load`] for
//! standalone `cosam-layout` use.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("export error: {0}")]
    Export(String),
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
    /// Build directly from a [`schedule_core::schedule::Schedule`] with no
    /// JSON serialization round-trip. Uses the public-export view (no private
    /// panels/presenters).
    pub fn from_schedule(
        schedule: &schedule_core::schedule::Schedule,
        title: &str,
    ) -> Result<Self, ModelError> {
        use schedule_core::query::export::export_to_widget_json;
        let export = export_to_widget_json(schedule, title, false)
            .map_err(|e| ModelError::Export(e.to_string()))?;

        let meta = Meta {
            title: export.meta.title,
            version: export.meta.version as u32,
            variant: export.meta.variant,
            generator: export.meta.generator,
            generated: export.meta.generated,
            modified: export.meta.modified,
            start_time: Some(export.meta.start_time).filter(|s| !s.is_empty()),
            end_time: Some(export.meta.end_time).filter(|s| !s.is_empty()),
        };

        let panels = export
            .panels
            .into_iter()
            .map(|p| Panel {
                id: p.id,
                base_id: p.base_id,
                name: p.name,
                panel_type: p.panel_type,
                room_ids: p.room_ids.into_iter().map(i64::from).collect(),
                start_time: p.start_time,
                end_time: p.end_time,
                duration: p.duration.try_into().ok(),
                description: p.description,
                note: p.note,
                prereq: p.prereq,
                cost: p.cost,
                capacity: p.capacity,
                difficulty: p.difficulty,
                ticket_url: p.ticket_url,
                is_premium: p.is_premium,
                is_full: p.is_full,
                is_kids: p.is_kids,
                credits: p.credits,
                presenters: p.presenters,
            })
            .collect();

        let rooms = export
            .rooms
            .into_iter()
            .map(|r| Room {
                uid: i64::from(r.uid),
                short_name: r.short_name,
                long_name: r.long_name,
                hotel_room: r.hotel_room,
                sort_key: i64::from(r.sort_key),
            })
            .collect();

        let panel_types = export
            .panel_types
            .into_iter()
            .map(|(k, pt)| {
                let colors = PanelTypeColors {
                    color: pt.colors.get("color").cloned(),
                    bw: pt.colors.get("bw").cloned(),
                };
                (
                    k,
                    PanelType {
                        kind: pt.kind,
                        colors,
                        is_break: pt.is_break,
                        is_cafe: pt.is_cafe,
                        is_workshop: pt.is_workshop,
                        is_hidden: pt.is_hidden,
                        is_room_hours: pt.is_room_hours,
                        is_timeline: pt.is_timeline,
                        is_private: pt.is_private,
                    },
                )
            })
            .collect();

        let timeline = export
            .timeline
            .into_iter()
            .map(|t| TimelineEntry {
                id: t.id,
                name: t.description,
                panel_type: t.panel_type,
                start_time: Some(t.start_time),
                end_time: None,
            })
            .collect();

        let presenters = export
            .presenters
            .into_iter()
            .map(|p| Presenter {
                uid: p.name.clone(),
                name: p.name,
                short_name: None,
            })
            .collect();

        Ok(Self {
            meta,
            panels,
            rooms,
            panel_types,
            timeline,
            presenters,
        })
    }

    /// Parse from a widget-JSON string (for standalone `cosam-layout` use).
    pub fn from_json(json: &str) -> Result<Self, ModelError> {
        Ok(serde_json::from_str(json)?)
    }

    /// Load from a widget-JSON file (for standalone `cosam-layout` use).
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
