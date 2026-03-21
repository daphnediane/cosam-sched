/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Serialize};

use super::source_info::{ChangeState, SourceInfo};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelType {
    #[serde(default)]
    pub uid: Option<String>,
    pub prefix: String,
    pub kind: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub is_break: bool,
    #[serde(default)]
    pub is_cafe: bool,
    #[serde(default)]
    pub is_workshop: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_hidden: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_room_hours: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bw_color: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_implicit: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_overnight: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_private: bool,
    #[serde(default, skip_serializing)]
    pub source: Option<SourceInfo>,
    #[serde(default, skip_serializing)]
    pub change_state: ChangeState,
}

impl PanelType {
    pub fn uid_from_prefix(prefix: &str) -> String {
        let slug = prefix
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-");
        let slug = slug.trim_matches('-');
        format!("panel-type-{slug}")
    }

    pub fn effective_uid(&self) -> String {
        // For v7, uid is the prefix directly
        self.prefix.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_type_deserialize() {
        let json = r##"{
            "uid": "panel-type-gp",
            "prefix": "GP",
            "kind": "Guest Panel",
            "color": "#E2F9D7",
            "isBreak": false,
            "isCafe": false,
            "isWorkshop": false
        }"##;
        let pt: PanelType = serde_json::from_str(json).unwrap();
        assert_eq!(pt.prefix, "GP");
        assert_eq!(pt.kind, "Guest Panel");
        assert_eq!(pt.color, Some("#E2F9D7".into()));
        assert_eq!(pt.uid, Some("panel-type-gp".into()));
        assert!(!pt.is_break);
    }

    #[test]
    fn test_panel_type_no_uid() {
        let json = r##"{
            "prefix": "GP",
            "kind": "Guest Panel",
            "color": "#E2F9D7"
        }"##;
        let pt: PanelType = serde_json::from_str(json).unwrap();
        assert_eq!(pt.uid, None);
        assert_eq!(pt.effective_uid(), "panel-type-gp");
    }

    #[test]
    fn test_panel_type_break() {
        let json = r##"{
            "prefix": "BRK",
            "kind": "Break",
            "color": "#CCCCCC",
            "isBreak": true,
            "isCafe": false,
            "isWorkshop": false
        }"##;
        let pt: PanelType = serde_json::from_str(json).unwrap();
        assert!(pt.is_break);
    }

    #[test]
    fn test_uid_from_prefix() {
        assert_eq!(PanelType::uid_from_prefix("GW"), "panel-type-gw");
        assert_eq!(PanelType::uid_from_prefix("ME"), "panel-type-me");
        assert_eq!(PanelType::uid_from_prefix("SPLIT"), "panel-type-split");
    }

    #[test]
    fn test_panel_type_roundtrip() {
        let pt = PanelType {
            uid: Some("panel-type-gp".into()),
            prefix: "GP".into(),
            kind: "Guest Panel".into(),
            color: Some("#E2F9D7".into()),
            is_break: false,
            is_cafe: false,
            is_workshop: false,
            is_hidden: false,
            is_room_hours: false,
            bw_color: None,
            is_implicit: false,
            is_overnight: false,
            is_private: false,
            source: None,
            change_state: ChangeState::Unchanged,
        };
        let json = serde_json::to_string(&pt).unwrap();
        let pt2: PanelType = serde_json::from_str(&json).unwrap();
        assert_eq!(pt, pt2);
    }
}
