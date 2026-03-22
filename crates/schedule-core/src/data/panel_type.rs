/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::panel::ExtraFields;
use super::source_info::{ChangeState, SourceInfo};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelType {
    #[serde(default, skip)]
    pub prefix: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub colors: IndexMap<String, String>,
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_timeline: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_private: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExtraFields>,
    #[serde(default, skip_serializing)]
    pub source: Option<SourceInfo>,
    #[serde(default, skip_serializing)]
    pub change_state: ChangeState,
}

impl PanelType {
    pub fn color(&self) -> Option<&str> {
        self.colors.get("color").map(|s| s.as_str())
    }

    pub fn bw_color(&self) -> Option<&str> {
        self.colors.get("bw").map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_type_deserialize() {
        let json = r##"{
            "kind": "Guest Panel",
            "colors": { "color": "#E2F9D7", "bw": "#CCCCCC" },
            "isBreak": false,
            "isCafe": false,
            "isWorkshop": false
        }"##;
        let pt: PanelType = serde_json::from_str(json).unwrap();
        assert_eq!(pt.kind, "Guest Panel");
        assert_eq!(pt.color(), Some("#E2F9D7"));
        assert_eq!(pt.bw_color(), Some("#CCCCCC"));
        assert!(!pt.is_break);
    }

    #[test]
    fn test_panel_type_break() {
        let json = r##"{
            "kind": "Break",
            "colors": { "color": "#CCCCCC" },
            "isBreak": true,
            "isCafe": false,
            "isWorkshop": false
        }"##;
        let pt: PanelType = serde_json::from_str(json).unwrap();
        assert!(pt.is_break);
    }

    #[test]
    fn test_panel_type_timeline() {
        let json = r##"{
            "kind": "Page split",
            "isTimeline": true
        }"##;
        let pt: PanelType = serde_json::from_str(json).unwrap();
        assert!(pt.is_timeline);
        assert!(!pt.is_break);
    }

    #[test]
    fn test_panel_type_roundtrip() {
        let mut colors = IndexMap::new();
        colors.insert("color".into(), "#FDEEB5".into());
        colors.insert("bw".into(), "#DDDDDD".into());
        let pt = PanelType {
            prefix: String::new(),
            kind: "Guest Workshop".into(),
            colors,
            is_break: false,
            is_cafe: false,
            is_workshop: true,
            is_hidden: false,
            is_room_hours: false,
            is_timeline: false,
            is_private: false,
            metadata: None,
            source: None,
            change_state: ChangeState::Unchanged,
        };
        let json = serde_json::to_string(&pt).unwrap();
        let pt2: PanelType = serde_json::from_str(&json).unwrap();
        assert_eq!(pt, pt2);
    }

    #[test]
    fn test_color_accessors() {
        let mut colors = IndexMap::new();
        colors.insert("color".into(), "#E2F9D7".into());
        let pt = PanelType {
            prefix: String::new(),
            kind: "Test".into(),
            colors,
            is_break: false,
            is_cafe: false,
            is_workshop: false,
            is_hidden: false,
            is_room_hours: false,
            is_timeline: false,
            is_private: false,
            metadata: None,
            source: None,
            change_state: ChangeState::Unchanged,
        };
        assert_eq!(pt.color(), Some("#E2F9D7"));
        assert_eq!(pt.bw_color(), None);
    }
}
