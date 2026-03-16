use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelType {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_type_deserialize() {
        let json = r##"{
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
        assert!(!pt.is_break);
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
    fn test_panel_type_roundtrip() {
        let pt = PanelType {
            prefix: "GW".into(),
            kind: "Guest Workshop".into(),
            color: Some("#FDEEB5".into()),
            is_break: false,
            is_cafe: false,
            is_workshop: true,
        };
        let json = serde_json::to_string(&pt).unwrap();
        let pt2: PanelType = serde_json::from_str(&json).unwrap();
        assert_eq!(pt, pt2);
    }
}
