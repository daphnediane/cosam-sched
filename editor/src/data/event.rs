use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub kind: Option<String>,
    pub panel_type: String,
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
    pub color: Option<String>,
    #[serde(default)]
    pub presenters: Vec<String>,
    #[serde(default)]
    pub is_break: bool,
    #[serde(default)]
    pub is_free: bool,
    #[serde(default)]
    pub is_full: bool,
    #[serde(default)]
    pub is_kids: bool,
    #[serde(default)]
    pub is_workshop: bool,
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
    fn test_event_deserialize_full() {
        let json = r##"{
            "id": "GP002",
            "name": "Cosplay Contest Misconceptions",
            "description": "A deep-dive into competition issues.",
            "startTime": "2026-06-26T14:00:00",
            "endTime": "2026-06-26T15:00:00",
            "duration": 60,
            "roomId": 10,
            "kind": "Guest Panel",
            "panelType": "GP",
            "cost": null,
            "capacity": null,
            "difficulty": null,
            "note": null,
            "prereq": null,
            "ticketUrl": null,
            "color": "#E2F9D7",
            "presenters": ["December Wynn", "Pro"],
            "isBreak": false,
            "isFree": true,
            "isFull": false,
            "isKids": false,
            "isWorkshop": false
        }"##;
        let event: Event = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, "GP002");
        assert_eq!(event.name, "Cosplay Contest Misconceptions");
        assert_eq!(event.duration, 60);
        assert_eq!(event.room_id, Some(10));
        assert_eq!(event.presenters.len(), 2);
        assert!(event.is_free);
        assert!(!event.is_break);
    }

    #[test]
    fn test_event_deserialize_minimal() {
        let json = r#"{
            "id": "BRK001",
            "name": "Lunch Break",
            "startTime": "2026-06-26T12:00:00",
            "endTime": "2026-06-26T13:00:00",
            "duration": 60,
            "panelType": "BRK",
            "isBreak": true
        }"#;
        let event: Event = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, "BRK001");
        assert!(event.is_break);
        assert_eq!(event.room_id, None);
        assert!(event.presenters.is_empty());
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
            "panelType": "GW",
            "cost": "$20.00",
            "capacity": "15",
            "difficulty": "Beginner",
            "note": "Materials provided",
            "prereq": null,
            "ticketUrl": null,
            "color": "#FDEEB5",
            "presenters": ["Sayakat Cosplay"],
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
            "panelType": "GP"
        }"#;
        let event: Event = serde_json::from_str(json).unwrap();
        assert_eq!(
            event.date(),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 26).unwrap()
        );
    }
}
