/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelType entity — maps Uniq ID prefixes to panel kinds.

use crate::EntityFields;

/// Panel type entity — maps a two-letter Uniq ID prefix to a kind name and
/// display/scheduling flags.
///
/// Sourced from the **PanelTypes** sheet of the schedule spreadsheet.
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PanelType)]
pub struct PanelType {
    #[field(
        display = "Prefix",
        description = "Two-letter Uniq ID prefix (e.g. \"GP\", \"FW\")"
    )]
    #[alias("prefix")]
    #[required]
    #[indexable(priority = 220)]
    pub prefix: String,

    #[field(
        display = "Panel Kind",
        description = "Human-readable kind name (e.g. \"Guest Panel\")"
    )]
    #[alias("panel_kind", "kind", "name")]
    #[required]
    #[indexable(priority = 210)]
    pub panel_kind: String,

    #[field(
        display = "Hidden",
        description = "Hide this type from the public schedule"
    )]
    #[alias("hidden", "is_hidden")]
    pub hidden: bool,

    #[field(display = "Is Workshop", description = "This type is a paid workshop")]
    #[alias("is_workshop", "workshop")]
    pub is_workshop: bool,

    #[field(
        display = "Is Break",
        description = "This type represents a convention-wide break"
    )]
    #[alias("is_break")]
    pub is_break: bool,

    #[field(display = "Is Café", description = "This type is a café panel")]
    #[alias("is_cafe", "is_café")]
    pub is_cafe: bool,

    #[field(
        display = "Is Room Hours",
        description = "This type represents room operating hours"
    )]
    #[alias("is_room_hours")]
    pub is_room_hours: bool,

    #[field(
        display = "Is TimeLine",
        description = "This type is a timeline / page-split marker"
    )]
    #[alias("is_timeline")]
    pub is_timeline: bool,

    #[field(
        display = "Is Private",
        description = "This type is private / staff-only (not shown publicly)"
    )]
    #[alias("is_private")]
    pub is_private: bool,

    #[field(
        display = "Color",
        description = "CSS color for this panel type (e.g. \"#db2777\")"
    )]
    #[alias("color", "colour")]
    pub color: Option<String>,

    #[field(
        display = "BW",
        description = "Alternate monochrome color for this panel type"
    )]
    #[alias("bw", "mono_color", "monochrome")]
    pub bw: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::{NonNilUuid, Uuid};

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
            ]))
        }
    }

    #[test]
    fn panel_type_id_from_uuid() {
        let nn = test_nn();
        let id = PanelTypeId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn panel_type_id_try_from_nil_returns_none() {
        assert!(PanelTypeId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn panel_type_id_display() {
        let id = PanelTypeId::from(test_nn());
        assert_eq!(
            id.to_string(),
            "panel-type-00000000-0000-0000-0000-000000000003"
        );
    }

    #[test]
    fn panel_type_id_serde_round_trip() {
        let id = PanelTypeId::from(test_nn());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000003\"");
        let back: PanelTypeId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
