/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel type entity implementation

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::{NonNilUuid, Uuid};

/// Panel type ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PanelTypeId(NonNilUuid);

impl fmt::Display for PanelTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panel-type-{}", self.0)
    }
}

impl From<NonNilUuid> for PanelTypeId {
    fn from(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl From<PanelTypeId> for NonNilUuid {
    fn from(id: PanelTypeId) -> NonNilUuid {
        id.0
    }
}

impl From<PanelTypeId> for Uuid {
    fn from(id: PanelTypeId) -> Uuid {
        id.0.into()
    }
}

impl crate::entity::TypedId for PanelTypeId {
    type EntityType = PanelTypeEntityType;
    fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }
    fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl PanelTypeId {
    /// Get the NonNilUuid from this ID
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }

    /// Get the raw UUID from this ID
    pub fn uuid(&self) -> Uuid {
        self.0.into()
    }

    /// Create a PanelTypeId from a NonNilUuid (infallible)
    pub fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }

    /// Try to create a PanelTypeId from a raw UUID (boundary use only)
    pub fn try_from_raw_uuid(uuid: Uuid) -> Option<Self> {
        NonNilUuid::new(uuid).map(Self)
    }
}

/// Panel type entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PanelType)]
pub struct PanelType {
    #[field(display = "Prefix", description = "Prefix for the panel type")]
    #[alias("prefix", "Prefix")]
    #[indexable(priority = 200)]
    #[required]
    pub prefix: String,

    #[field(display = "Kind", description = "Type/kind of panel")]
    #[alias("kind", "Panel_Kind", "type", "category")]
    #[indexable(priority = 100)]
    #[required]
    pub kind: String,

    #[field(display = "Color", description = "Display color for the panel type")]
    #[alias("color", "Color", "display_color")]
    pub color: Option<String>,

    #[field(
        display = "Is Break",
        description = "Whether this is a break time slot"
    )]
    #[alias("break", "Is_Break", "is_break", "breakTime")]
    pub is_break: bool,

    #[field(
        display = "Is Cafe",
        description = "Whether this is a cafe/social event"
    )]
    #[alias("cafe", "Is_Cafe", "Is_Café", "is_cafe", "social")]
    pub is_cafe: bool,

    #[field(
        display = "Is Workshop",
        description = "Whether this is a workshop event"
    )]
    #[alias("workshop", "Is_Workshop", "is_workshop", "hands_on")]
    pub is_workshop: bool,

    #[field(
        display = "Is Hidden",
        description = "Whether this panel type should be hidden"
    )]
    #[alias("hidden", "Hidden", "is_hidden", "invisible")]
    pub is_hidden: bool,

    #[field(
        display = "Is Room Hours",
        description = "Whether this represents room hours"
    )]
    #[alias("room_hours", "Is_Room_Hours", "is_room_hours", "facility")]
    pub is_room_hours: bool,

    #[field(
        display = "Is Timeline",
        description = "Whether this appears on timeline"
    )]
    #[alias("timeline", "Is_Timeline", "Is_Time_Line", "is_timeline", "scheduled")]
    pub is_timeline: bool,

    #[field(
        display = "Is Private",
        description = "Whether this is a private event"
    )]
    #[alias("private", "Is_Private", "is_private", "restricted")]
    pub is_private: bool,

    #[field(display = "B&W Color", description = "Black and white display color")]
    #[alias("bw", "BW", "bw_color", "monochrome_color")]
    pub bw_color: Option<String>,

    #[computed_field(display = "Panels", description = "All panels of this type")]
    #[alias("panels_of_type", "panel_list", "typed_panels")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PanelTypeData| {
        let panel_ids = schedule.find_related::<crate::entity::PanelEntityType>(
            entity.entity_uuid,
            crate::edge::EdgeType::PanelToPanelType,
            crate::schedule::RelationshipDirection::Incoming
        );
        Some(crate::field::FieldValue::List(
            schedule.get_entity_names::<crate::entity::PanelEntityType>(&panel_ids)
                .into_iter()
                .map(crate::field::FieldValue::String)
                .collect()
        ))
    })]
    pub panels: Vec<crate::entity::PanelId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
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
    fn panel_type_id_try_from_nil_uuid_returns_none() {
        assert!(PanelTypeId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn panel_type_id_display() {
        let id = PanelTypeId::from(test_nn());
        assert_eq!(
            id.to_string(),
            "panel-type-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn panel_type_id_serde_round_trip() {
        let id = PanelTypeId::from(test_nn());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000001\"");
        let back: PanelTypeId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
