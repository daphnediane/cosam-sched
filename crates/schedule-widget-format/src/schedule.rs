/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule data structures.
//!
//! This module contains the core schedule data: metadata, panels, rooms,
//! panel types, timeline, and presenters. Presentation configuration
//! (branding, print formats) lives in the separate `config` module.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::WidgetFormatError;

/// Top-level metadata for a widget JSON document.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetMeta {
    pub title: String,
    /// Widget JSON format version. Consumers branch on this; bumped to `1` for
    /// the unified-DTO format (inline panel-type `prefix`, typed `colors`,
    /// timeline `name`).
    pub version: i32,
    pub generator: String,
    pub generated: String,
    pub modified: String,
    /// Schedule window start as Unix epoch seconds (FEATURE-154). Canonical,
    /// timezone-unambiguous time; combine with [`Self::timezone`] to recover the
    /// wall-clock. `0` when unknown.
    #[serde(default)]
    pub start_epoch: i64,
    /// Schedule window end as Unix epoch seconds. See [`Self::start_epoch`].
    #[serde(default)]
    pub end_epoch: i64,
    /// IANA timezone name the epoch times are displayed in (used to recover the
    /// wall-clock and to anchor `.ics` output). Empty when unknown.
    #[serde(default)]
    pub timezone: String,
    /// Precomputed iCalendar `VTIMEZONE` component for `timezone`, covering the
    /// schedule window, so the widget can emit correctly-anchored `.ics` files.
    /// Empty when there is no timezone or it needs no `VTIMEZONE` (e.g. UTC).
    #[serde(default)]
    pub vtimezone: String,
}

/// Panel entry (one schedulable session).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPanel {
    pub id: String,
    pub base_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_num: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_num: Option<i32>,
    /// Number of distinct parts in this panel's multi-part series, set only when
    /// the panel belongs to a series with more than one part. Drives "Part N of
    /// M" labeling and signals that a single cost covers every part. Absent for
    /// standalone panels and for plain multi-session reruns (where each session
    /// carries its own cost).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_parts: Option<i32>,
    /// True on the single "lead" instance of a multi-part series (lowest part
    /// number, then earliest start time — normally Part 1). The lead bears the
    /// shared series cost; continuation parts suppress the price to avoid
    /// implying a separate charge per part.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_series_lead: bool,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_ids: Vec<i32>,
    /// Start time as Unix epoch seconds (FEATURE-154). Canonical,
    /// timezone-unambiguous time; combine with the meta timezone to recover the
    /// wall-clock. Absent for unscheduled panels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_epoch: Option<i64>,
    /// End time as Unix epoch seconds. See [`Self::start_epoch`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_epoch: Option<i64>,
    pub duration: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prereq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_premium: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_full: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_kids: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credits: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presenters: Vec<String>,
}

impl WidgetPanel {
    /// A continuation part of a multi-part series (i.e. a member that is not the
    /// cost-bearing lead). The shared price is shown only on the lead, so these
    /// suppress the cost and display "Part N of M" instead.
    #[must_use]
    pub fn is_series_continuation(&self) -> bool {
        self.total_parts.is_some() && !self.is_series_lead
    }
}

/// Room entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetRoom {
    pub uid: i32,
    pub short_name: String,
    pub long_name: String,
    pub hotel_room: String,
    pub sort_key: i32,
    pub is_break: bool,
}

/// Panel-type colors. Named fields replace the former stringly-typed map so
/// consumers read `colors.color` / `colors.bw` directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPanelColors {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bw: Option<String>,
}

/// Panel-type entry. Keyed by `prefix` in [`WidgetExport::panel_types`]; the
/// `prefix` is also carried inline so list-oriented consumers don't have to
/// reconstruct identity from the map key.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPanelType {
    /// Type prefix (e.g. `"GP"`), matching this entry's key in `panelTypes`.
    #[serde(default)]
    pub prefix: String,
    pub kind: String,
    #[serde(default)]
    pub colors: WidgetPanelColors,
    pub is_break: bool,
    pub is_cafe: bool,
    pub is_workshop: bool,
    pub is_hidden: bool,
    pub is_room_hours: bool,
    pub is_timeline: bool,
    pub is_private: bool,
}

/// Timeline entry. `name` is the display label (was `description` pre-v1, kept
/// readable via the alias for older published JSON).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetTimeline {
    pub id: String,
    /// Start time as Unix epoch seconds (FEATURE-154). Canonical,
    /// timezone-unambiguous time; combine with the meta timezone to recover the
    /// wall-clock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_epoch: Option<i64>,
    #[serde(alias = "description")]
    pub name: String,
    pub panel_type: Option<String>,
    pub note: Option<String>,
}

/// Presenter entry (DisplayPresenter).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPresenter {
    pub name: String,
    pub rank: String,
    pub sort_key: i32,
    pub is_group: bool,
    pub members: Vec<String>,
    pub groups: Vec<String>,
    pub panel_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub subsumes_members: bool,
}

/// Complete schedule export (the core data structure).
///
/// This contains the schedule data itself: metadata, panels, rooms,
/// panel types, timeline, and presenters. Presentation configuration
/// (branding, print formats) lives in the separate `config` module as
/// [`crate::config::ScheduleConfig`] so the same schedule can be displayed with
/// different styling without modifying the core data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetExport {
    pub meta: WidgetMeta,
    pub panels: Vec<WidgetPanel>,
    pub rooms: Vec<WidgetRoom>,
    pub panel_types: BTreeMap<String, WidgetPanelType>,
    #[serde(default)]
    pub timeline: Vec<WidgetTimeline>,
    #[serde(default)]
    pub presenters: Vec<WidgetPresenter>,
}

impl WidgetExport {
    /// Parse from a widget-JSON string.
    pub fn from_json(json: &str) -> Result<Self, WidgetFormatError> {
        Ok(serde_json::from_str(json)?)
    }

    /// Load from a widget-JSON file.
    pub fn load(path: &std::path::Path) -> Result<Self, WidgetFormatError> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }

    /// True when `panel_type` resolves to a break-typed entry.
    fn is_break_type(&self, panel_type: Option<&str>) -> bool {
        self.panel_types
            .get(panel_type.unwrap_or(""))
            .map(|pt| pt.is_break)
            .unwrap_or(false)
    }

    /// Non-break panels with a scheduled start time.
    pub fn scheduled_panels(&self) -> Vec<&WidgetPanel> {
        self.panels
            .iter()
            .filter(|p| p.start_epoch.is_some() && !self.is_break_type(p.panel_type.as_deref()))
            .collect()
    }

    /// Break panels (implicit and overnight).
    pub fn break_panels(&self) -> Vec<&WidgetPanel> {
        self.panels
            .iter()
            .filter(|p| self.is_break_type(p.panel_type.as_deref()))
            .collect()
    }

    /// Rooms sorted by `sort_key`.
    pub fn sorted_rooms(&self) -> Vec<&WidgetRoom> {
        let mut rooms: Vec<&WidgetRoom> = self.rooms.iter().collect();
        rooms.sort_by_key(|r| r.sort_key);
        rooms
    }
}

fn is_false(b: &bool) -> bool {
    !b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_deserialize() {
        let json = r#"{
            "meta": {"title": "Test Schedule", "version": 1, "variant": "display", "generator": "test", "generated": "2026-01-01T00:00:00Z", "modified": "2026-01-01T00:00:00Z", "startTime": "", "endTime": ""},
            "panels": [],
            "rooms": [],
            "panelTypes": {}
        }"#;
        let data = WidgetExport::from_json(json).unwrap();
        assert_eq!(data.meta.title, "Test Schedule");
        assert_eq!(data.meta.version, 1);
        assert!(data.panels.is_empty());
    }

    #[test]
    fn test_panel_colors_default() {
        let colors = WidgetPanelColors::default();
        assert!(colors.color.is_none());
        assert!(colors.bw.is_none());
    }

    #[test]
    fn test_timeline_description_alias() {
        // Older published JSON used `description`; it must still parse into `name`.
        let json = r#"{"id":"TL1","startTime":"2026-06-26T09:00:00","description":"Friday Morning","panelType":null,"note":null}"#;
        let tl: WidgetTimeline = serde_json::from_str(json).unwrap();
        assert_eq!(tl.name, "Friday Morning");
    }

    #[test]
    fn test_roundtrip_with_typed_colors_and_prefix() {
        let json = r##"{"meta":{"title":"RT","version":2,"variant":"display","generator":"test","generated":"2026-01-01T00:00:00Z","modified":"2026-01-01T00:00:00Z","startEpoch":1782842400,"endEpoch":1782846000},"panels":[{"id":"GP001","baseId":"GP001","name":"Test Panel","panelType":"GP","roomIds":[1],"startEpoch":1782842400,"endEpoch":1782846000,"duration":60}],"rooms":[{"uid":1,"shortName":"Main","longName":"Main Hall","hotelRoom":"Ballroom A","sortKey":0,"isBreak":false}],"panelTypes":{"GP":{"prefix":"GP","kind":"Guest Panel","colors":{"color":"#E2F9D7"},"isBreak":false,"isCafe":false,"isWorkshop":false,"isHidden":false,"isRoomHours":false,"isTimeline":false,"isPrivate":false}}}"##;
        let data = WidgetExport::from_json(json).unwrap();
        assert_eq!(data.panel_types["GP"].prefix, "GP");
        assert_eq!(
            data.panel_types["GP"].colors.color.as_deref(),
            Some("#E2F9D7")
        );
        assert_eq!(data.scheduled_panels().len(), 1);

        let re = serde_json::to_string(&data).unwrap();
        let data2 = WidgetExport::from_json(&re).unwrap();
        assert_eq!(
            data2.panel_types["GP"].colors.color.as_deref(),
            Some("#E2F9D7")
        );
    }
}
