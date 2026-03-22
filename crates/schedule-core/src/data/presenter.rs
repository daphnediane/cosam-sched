/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Deserializer, Serialize};

use super::panel::ExtraFields;
use super::source_info::{ChangeState, SourceInfo};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Presenter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub name: String,
    pub rank: String,
    #[serde(default, deserialize_with = "deserialize_bool_or_int")]
    pub is_group: bool,
    #[serde(default)]
    pub members: std::collections::BTreeSet<String>,
    #[serde(default)]
    pub groups: std::collections::BTreeSet<String>,
    #[serde(default, deserialize_with = "deserialize_bool_or_int")]
    pub always_grouped: bool,
    #[serde(default, deserialize_with = "deserialize_bool_or_int")]
    pub always_shown: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExtraFields>,
    #[serde(default, skip_serializing)]
    pub source: Option<SourceInfo>,
    #[serde(default, skip_serializing)]
    pub change_state: ChangeState,
}

fn deserialize_bool_or_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => Ok(n.as_i64().unwrap_or(0) != 0),
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presenter_deserialize_minimal() {
        let json = r#"{"name": "Yaya Han", "rank": "guest"}"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "Yaya Han");
        assert_eq!(p.rank, "guest");
        assert!(!p.is_group);
        assert_eq!(p.members, std::collections::BTreeSet::new());
        assert_eq!(p.groups, std::collections::BTreeSet::new());
        assert!(!p.always_grouped);
        assert!(!p.always_shown);
        assert_eq!(p.id, None);
    }

    #[test]
    fn test_presenter_deserialize_full() {
        let json = r#"{
            "name": "Pros and Cons Cosplay",
            "rank": "guest",
            "is_group": true,
            "members": ["Pro", "Con"],
            "groups": [],
            "always_grouped": false
        }"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert!(p.is_group);
        let mut expected_members = std::collections::BTreeSet::new();
        expected_members.insert("Pro".to_string());
        expected_members.insert("Con".to_string());
        assert_eq!(p.members, expected_members);
    }

    #[test]
    fn test_presenter_deserialize_int_is_group() {
        let json = r#"{"name": "UNC Staff", "rank": "guest", "is_group": 1}"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert!(p.is_group);
    }

    #[test]
    fn test_presenter_with_groups() {
        let json = r#"{
            "name": "Con",
            "rank": "guest",
            "is_group": false,
            "members": [],
            "groups": ["Pros and Cons Cosplay"],
            "always_grouped": false
        }"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        let mut expected_groups = std::collections::BTreeSet::new();
        expected_groups.insert("Pros and Cons Cosplay".to_string());
        assert_eq!(p.groups, expected_groups);
        assert!(!p.is_group);
    }

    #[test]
    fn test_presenter_roundtrip() {
        let p = Presenter {
            id: Some(5),
            name: "Sayakat Cosplay".into(),
            rank: "fan_panelist".into(),
            is_group: false,
            members: std::collections::BTreeSet::new(),
            groups: std::collections::BTreeSet::new(),
            always_grouped: false,
            always_shown: false,
            metadata: None,
            source: None,
            change_state: ChangeState::Unchanged,
        };
        let json = serde_json::to_string(&p).unwrap();
        let p2: Presenter = serde_json::from_str(&json).unwrap();
        assert_eq!(p, p2);
    }
}
