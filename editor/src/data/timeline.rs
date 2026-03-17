use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntry {
    pub id: String,
    pub start_time: String,
    pub description: String,
    #[serde(default)]
    pub time_type: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeType {
    pub uid: String,
    pub prefix: String,
    pub kind: String,
}

impl TimeType {
    pub fn uid_from_prefix(prefix: &str) -> String {
        let slug = prefix
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-");
        let slug = slug.trim_matches('-');
        format!("time-type-{slug}")
    }
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
            "timeType": "time-type-split",
            "note": "Opening ceremonies"
        }"##;
        let entry: TimelineEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.id, "SPLIT01");
        assert_eq!(entry.description, "Thursday Morning");
        assert_eq!(entry.time_type, Some("time-type-split".into()));
    }

    #[test]
    fn test_time_type_deserialize() {
        let json = r##"{
            "uid": "time-type-split",
            "prefix": "SPLIT",
            "kind": "Page split"
        }"##;
        let time_type: TimeType = serde_json::from_str(json).unwrap();
        assert_eq!(time_type.uid, "time-type-split");
        assert_eq!(time_type.prefix, "SPLIT");
        assert_eq!(time_type.kind, "Page split");
    }

    #[test]
    fn test_uid_from_prefix() {
        assert_eq!(TimeType::uid_from_prefix("SPLIT"), "time-type-split");
        assert_eq!(TimeType::uid_from_prefix("SPLITDAY"), "time-type-splitday");
        assert_eq!(TimeType::uid_from_prefix("GW"), "time-type-gw");
    }

    #[test]
    fn test_timeline_entry_roundtrip() {
        let entry = TimelineEntry {
            id: "SPLIT01".to_string(),
            start_time: "2026-06-26T09:00:00".to_string(),
            description: "Thursday Morning".to_string(),
            time_type: Some("time-type-split".to_string()),
            note: Some("Opening ceremonies".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let entry2: TimelineEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, entry2);
    }

    #[test]
    fn test_time_type_roundtrip() {
        let time_type = TimeType {
            uid: "time-type-split".to_string(),
            prefix: "SPLIT".to_string(),
            kind: "Page split".to_string(),
        };
        let json = serde_json::to_string(&time_type).unwrap();
        let time_type2: TimeType = serde_json::from_str(&json).unwrap();
        assert_eq!(time_type, time_type2);
    }
}
