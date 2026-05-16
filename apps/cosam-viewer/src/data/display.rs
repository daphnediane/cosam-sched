/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Serde types matching the cosam widget JSON format (docs/widget-json-format.md).

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize a field that may be either a JSON number or a quoted string
/// containing a number (e.g. `"15"` or `15`), yielding `Option<u32>`.
fn deserialize_opt_u32_or_string<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct OptU32OrString;

    impl<'de> Visitor<'de> for OptU32OrString {
        type Value = Option<u32>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a u32, a string containing a u32, or null")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as u32))
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v as u32))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            if v.is_empty() {
                return Ok(None);
            }
            v.trim().parse::<u32>().map(Some).map_err(de::Error::custom)
        }

        fn visit_some<D2: Deserializer<'de>>(self, d: D2) -> Result<Self::Value, D2::Error> {
            d.deserialize_any(self)
        }
    }

    deserializer.deserialize_option(OptU32OrString)
}

// ---------------------------------------------------------------------------
// Top-level document
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDoc {
    pub meta: Meta,
    pub panels: Vec<DisplayPanel>,
    pub rooms: Vec<Room>,
    pub panel_types: HashMap<String, PanelType>,
    #[serde(default)]
    pub timeline: Vec<TimelineEntry>,
    #[serde(default)]
    pub presenters: Vec<DisplayPresenter>,
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub title: String,
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub variant: Option<String>,
    #[serde(default)]
    pub generator: Option<String>,
    #[serde(default)]
    pub generated: Option<String>,
    #[serde(default)]
    pub modified: Option<String>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
}

// ---------------------------------------------------------------------------
// Panel
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayPanel {
    pub id: String,
    #[serde(default)]
    pub base_id: Option<String>,
    #[serde(default)]
    pub part_num: Option<u32>,
    #[serde(default)]
    pub session_num: Option<u32>,
    pub name: String,
    pub panel_type: String,
    /// Room UIDs (u32) assigned at export time.
    #[serde(default)]
    pub room_ids: Vec<u32>,
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
    #[serde(default, deserialize_with = "deserialize_opt_u32_or_string")]
    pub capacity: Option<u32>,
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

// ---------------------------------------------------------------------------
// Room
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub uid: u32,
    pub short_name: String,
    pub long_name: String,
    #[serde(default)]
    pub hotel_room: Option<String>,
    #[serde(default)]
    pub sort_key: Option<i64>,
    #[serde(default)]
    pub is_break: bool,
}

// ---------------------------------------------------------------------------
// Panel type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelType {
    pub kind: String,
    #[serde(default)]
    pub colors: Option<PanelTypeColors>,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelTypeColors {
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub bw: Option<String>,
}

// ---------------------------------------------------------------------------
// Timeline entry
//
// The actual format varies: the widget spec documents {kind, time, label} but
// the exporter currently emits panel-like objects {id, startTime, panelType,
// description, note}. Accept any JSON object and ignore the contents — the
// viewer uses the timeline array only as a length hint and does not render it
// directly (timeline panels are filtered out via is_timeline in panel_types).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimelineEntry(serde_json::Value);

// ---------------------------------------------------------------------------
// Presenter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayPresenter {
    pub name: String,
    #[serde(default)]
    pub rank: Option<String>,
    #[serde(default)]
    pub sort_key: Option<i32>,
    #[serde(default)]
    pub is_group: bool,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub panel_ids: Vec<String>,
    #[serde(default)]
    pub subsumes_members: bool,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl ScheduleDoc {
    /// Load from a JSON byte slice.
    pub fn from_json(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }

    /// Look up a room by uid.
    pub fn room_by_uid(&self, uid: u32) -> Option<&Room> {
        self.rooms.iter().find(|r| r.uid == uid)
    }

    /// Return all non-break, non-hidden panel types sorted by kind key.
    pub fn visible_types(&self) -> Vec<(&String, &PanelType)> {
        let mut types: Vec<_> = self
            .panel_types
            .iter()
            .filter(|(_, pt)| !pt.is_hidden && !pt.is_break && !pt.is_timeline)
            .collect();
        types.sort_by_key(|(k, _)| k.as_str());
        types
    }

    /// Return all non-break rooms sorted by sort_key then long_name.
    pub fn visible_rooms(&self) -> Vec<&Room> {
        let mut rooms: Vec<_> = self.rooms.iter().filter(|r| !r.is_break).collect();
        rooms.sort_by(|a, b| {
            a.sort_key
                .cmp(&b.sort_key)
                .then_with(|| a.long_name.cmp(&b.long_name))
        });
        rooms
    }
}
