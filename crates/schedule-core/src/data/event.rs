/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use super::source_info::{ChangeState, SourceInfo};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventConflict {
    #[serde(rename = "type")]
    pub conflict_type: String,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub conflict_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub duration: u32,
    #[serde(default)]
    pub room_id: Option<u32>,
    pub panel_type: Option<String>,
    #[serde(default)]
    pub cost: Option<String>,
    #[serde(default)]
    pub capacity: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub prereq: Option<String>,
    #[serde(default)]
    pub ticket_url: Option<String>,
    #[serde(default)]
    pub presenters: Vec<String>,
    #[serde(default)]
    pub credits: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<EventConflict>,
    #[serde(default)]
    pub is_free: bool,
    #[serde(default)]
    pub is_full: bool,
    #[serde(default)]
    pub is_kids: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_panelist: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_panelist: Option<String>,
    #[serde(default, skip_serializing)]
    pub source: Option<SourceInfo>,
    #[serde(default, skip_serializing)]
    pub change_state: ChangeState,
}

impl Event {
    #[must_use]
    pub fn date(&self) -> chrono::NaiveDate {
        self.start_time.date()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_deserialize_converter_format() {
        let json = r##"{
            "id": "GP002",
            "name": "Cosplay Contest Misconceptions",
            "description": "A deep-dive into competition issues.",
            "startTime": "2026-06-26T14:00:00",
            "endTime": "2026-06-26T15:00:00",
            "duration": 60,
            "roomId": 10,
            "kind": "Guest Panel",
            "panelType": "panel-type-gp",
            "cost": null,
            "capacity": null,
            "difficulty": null,
            "note": null,
            "prereq": null,
            "ticketUrl": null,
            "presenters": ["December Wynn", "Pro"],
            "credits": ["December Wynn", "Pros and Cons Cosplay"],
            "conflicts": [],
            "isFree": true,
            "isFull": false,
            "isKids": false
        }"##;
        let event: Event = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, "GP002");
        assert_eq!(event.panel_type, Some("panel-type-gp".into()));
        assert_eq!(event.credits.len(), 2);
        assert!(event.conflicts.is_empty());
        assert!(event.is_free);
    }

    #[test]
    fn test_event_deserialize_minimal() {
        let json = r#"{
            "id": "BRK001",
            "name": "Lunch Break",
            "startTime": "2026-06-26T12:00:00",
            "endTime": "2026-06-26T13:00:00",
            "duration": 60,
            "panelType": "panel-type-brk"
        }"#;
        let event: Event = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, "BRK001");
        assert_eq!(event.room_id, None);
        assert!(event.presenters.is_empty());
        assert!(event.credits.is_empty());
        assert_eq!(event.description, None);
    }

    #[test]
    fn test_event_roundtrip() {
        let json = r##"{
            "id": "GW001",
            "name": "Foam Armor Basics",
            "description": "Learn foam armor construction.",
            "startTime": "2026-06-26T14:00:00",
            "endTime": "2026-06-26T16:00:00",
            "duration": 120,
            "roomId": 0,
            "kind": "Guest Workshop",
            "panelType": "panel-type-gw",
            "cost": "$20.00",
            "capacity": "15",
            "difficulty": "Beginner",
            "note": "Materials provided",
            "prereq": null,
            "ticketUrl": null,
            "presenters": ["Sayakat Cosplay"],
            "credits": ["Sayakat Cosplay"],
            "conflicts": [],
            "isBreak": false,
            "isFree": false,
            "isFull": false,
            "isKids": false,
            "isWorkshop": true
        }"##;
        let event: Event = serde_json::from_str(json).unwrap();
        let reserialized = serde_json::to_string(&event).unwrap();
        let event2: Event = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(event, event2);
    }

    #[test]
    fn test_event_date() {
        let json = r#"{
            "id": "GP001",
            "name": "Test",
            "startTime": "2026-06-26T14:00:00",
            "endTime": "2026-06-26T15:00:00",
            "duration": 60,
            "panelType": "panel-type-gp"
        }"#;
        let event: Event = serde_json::from_str(json).unwrap();
        assert_eq!(
            event.date(),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 26).unwrap()
        );
    }
}
