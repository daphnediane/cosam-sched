/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use super::panel::ExtraFields;
use super::source_info::{ChangeState, SourceInfo};
use super::time::{deserialize_optional_datetime, serialize_optional_datetime};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntry {
    pub id: String,
    #[serde(
        serialize_with = "serialize_optional_datetime",
        deserialize_with = "deserialize_optional_datetime"
    )]
    pub start_time: Option<NaiveDateTime>,
    pub description: String,
    #[serde(default, alias = "timeType")]
    pub panel_type: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExtraFields>,
    #[serde(default, skip_serializing)]
    pub source: Option<SourceInfo>,
    #[serde(default, skip_serializing)]
    pub change_state: ChangeState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_entry_deserialize() {
        let json = r##"{
            "id": "SPLIT01",
            "startTime": "2026-06-26T09:00:00",
            "description": "Thursday Morning",
            "panelType": "SPLIT",
            "note": "Opening ceremonies"
        }"##;
        let entry: TimelineEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.id, "SPLIT01");
        assert_eq!(entry.description, "Thursday Morning");
        assert_eq!(entry.panel_type, Some("SPLIT".into()));
    }

    #[test]
    fn test_timeline_entry_roundtrip() {
        let entry = TimelineEntry {
            id: "SPLIT01".to_string(),
            start_time: Some(
                chrono::NaiveDateTime::parse_from_str("2026-06-26T09:00:00", "%Y-%m-%dT%H:%M:%S")
                    .unwrap(),
            ),
            description: "Thursday Morning".to_string(),
            panel_type: Some("SPLIT".to_string()),
            note: Some("Opening ceremonies".to_string()),
            metadata: None,
            source: None,
            change_state: ChangeState::Unchanged,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let entry2: TimelineEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, entry2);
    }
}
