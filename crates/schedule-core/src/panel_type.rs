/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelType entity — the simplest entity type, proof of concept for the field system.
//!
//! Three structs define the PanelType entity:
//!
//! - [`PanelTypeCommonData`] — user-facing fields from the PanelTypes sheet
//! - [`PanelTypeInternalData`] — `EntityType::InternalData`; the field system operates on this
//! - [`PanelTypeData`] — export/API view produced by [`PanelTypeEntityType::export`]
//!
//! Field descriptors are static values assembled into a [`FieldSet`] inside a [`LazyLock`].

use crate::entity::{EntityId, EntityType, FieldSet};
use crate::field::{FieldDescriptor, MatchPriority, ReadFn};
use crate::field_macros::{bool_field, edge_list_field, opt_string_field, req_string_field};
use crate::panel::PanelId;
use crate::value::{CrdtFieldType, FieldValue, ValidationError};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── Type Aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for PanelType entities.
pub type PanelTypeId = EntityId<PanelTypeEntityType>;

// ── PanelTypeCommonData ───────────────────────────────────────────────────────

/// User-facing fields from the PanelTypes sheet.
///
/// This struct is serializable and represents the data as stored/imported.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelTypeCommonData {
    /// Two-letter Uniq ID prefix (required, indexed).
    pub prefix: String,

    /// Human-readable kind name (required, indexed).
    pub panel_kind: String,

    /// Hidden flag — not shown in UI.
    pub hidden: bool,

    /// Is a workshop panel.
    pub is_workshop: bool,

    /// Is a break period.
    pub is_break: bool,

    /// Is a cafe event.
    pub is_cafe: bool,

    /// Is room hours scheduling.
    pub is_room_hours: bool,

    /// Is timeline event.
    pub is_timeline: bool,

    /// Is private event.
    pub is_private: bool,

    /// CSS color (e.g. `"#db2777"`).
    pub color: Option<String>,

    /// Alternate monochrome color.
    pub bw: Option<String>,
}

impl PanelTypeCommonData {
    /// Validate the common data and return any constraint violations.
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        if self.prefix.is_empty() {
            errors.push(ValidationError::Required { field: "prefix" });
        }
        if self.panel_kind.is_empty() {
            errors.push(ValidationError::Required {
                field: "panel_kind",
            });
        }

        errors
    }
}

// ── PanelTypeInternalData ─────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
///
/// This type is public because it appears in the [`EntityType`] trait,
/// but direct field mutation should be done through the field system rather
/// than by modifying this struct directly.
#[derive(Debug, Clone)]
pub struct PanelTypeInternalData {
    pub data: PanelTypeCommonData,
    pub code: PanelTypeId,
}

// ── PanelTypeData ───────────────────────────────────────────────────────────────

/// Export/API view produced by `export(&Schedule)`.
///
/// This is the public face of PanelType data for serialization and API use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelTypeData {
    #[serde(flatten)]
    pub data: PanelTypeCommonData,
    pub code: String,
    /// Panels of this type — assembled from edge maps (deferred to FEATURE-018).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub panels: Vec<PanelId>,
}

// ── PanelTypeEntityType ─────────────────────────────────────────────────────────

/// Singleton type representing the PanelType entity kind.
///
/// Implements [`EntityType`] to provide type-safe identification, field registry,
/// and export functionality for PanelType entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelTypeEntityType;

impl EntityType for PanelTypeEntityType {
    type InternalData = PanelTypeInternalData;
    type Data = PanelTypeData;

    const TYPE_NAME: &'static str = "panel_type";

    fn uuid_namespace() -> &'static uuid::Uuid {
        static NS: LazyLock<uuid::Uuid> =
            LazyLock::new(|| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, b"panel_type"));
        &NS
    }

    fn field_set() -> &'static FieldSet<Self> {
        &PANEL_TYPE_FIELD_SET
    }

    fn export(internal: &Self::InternalData) -> Self::Data {
        PanelTypeData {
            data: internal.data.clone(),
            code: internal.code.to_string(),
            panels: Vec::new(), // Edge-backed; populated in FEATURE-018
        }
    }

    fn validate(internal: &Self::InternalData) -> Vec<ValidationError> {
        internal.data.validate()
    }
}

// ── Field Descriptors ──────────────────────────────────────────────────────────

req_string_field!(FIELD_PREFIX, PanelTypeEntityType, PanelTypeInternalData, prefix,
    name: "prefix", display: "Prefix",
    desc: "Two-letter Uniq ID prefix for panels of this type.",
    aliases: &["uniq_id_prefix"]);

req_string_field!(FIELD_PANEL_KIND, PanelTypeEntityType, PanelTypeInternalData, panel_kind,
    name: "panel_kind", display: "Panel Kind",
    desc: "Human-readable kind name for this panel type.",
    aliases: &["kind", "type_name"]);

bool_field!(FIELD_HIDDEN, PanelTypeEntityType, PanelTypeInternalData, hidden,
    name: "hidden", display: "Hidden",
    desc: "Whether this panel type is hidden from UI.",
    aliases: &[]);

bool_field!(FIELD_IS_WORKSHOP, PanelTypeEntityType, PanelTypeInternalData, is_workshop,
    name: "is_workshop", display: "Is Workshop",
    desc: "Whether panels of this type are workshops.",
    aliases: &["workshop"]);

bool_field!(FIELD_IS_BREAK, PanelTypeEntityType, PanelTypeInternalData, is_break,
    name: "is_break", display: "Is Break",
    desc: "Whether panels of this type are break periods.",
    aliases: &["break"]);

bool_field!(FIELD_IS_CAFE, PanelTypeEntityType, PanelTypeInternalData, is_cafe,
    name: "is_cafe", display: "Is Cafe",
    desc: "Whether panels of this type are cafe events.",
    aliases: &["cafe"]);

bool_field!(FIELD_IS_ROOM_HOURS, PanelTypeEntityType, PanelTypeInternalData, is_room_hours,
    name: "is_room_hours", display: "Is Room Hours",
    desc: "Whether panels of this type are room hours.",
    aliases: &["room_hours"]);

bool_field!(FIELD_IS_TIMELINE, PanelTypeEntityType, PanelTypeInternalData, is_timeline,
    name: "is_timeline", display: "Is Timeline",
    desc: "Whether panels of this type are timeline events.",
    aliases: &["timeline"]);

bool_field!(FIELD_IS_PRIVATE, PanelTypeEntityType, PanelTypeInternalData, is_private,
    name: "is_private", display: "Is Private",
    desc: "Whether panels of this type are private events.",
    aliases: &["private"]);

opt_string_field!(FIELD_COLOR, PanelTypeEntityType, PanelTypeInternalData, color,
    name: "color", display: "Color",
    desc: "CSS color for panels of this type.",
    aliases: &[]);

opt_string_field!(FIELD_BW, PanelTypeEntityType, PanelTypeInternalData, bw,
    name: "bw", display: "BW Color",
    desc: "Alternate monochrome color for panels of this type.",
    aliases: &["bw_color", "monochrome"]);

/// Computed display name — derived from `panel_kind` and `prefix`.
///
/// Read-only computed field that produces a human-readable identifier.
static FIELD_DISPLAY_NAME: FieldDescriptor<PanelTypeEntityType> = FieldDescriptor {
    name: "display_name",
    display: "Display Name",
    description: "Human-readable display name combining kind and prefix.",
    aliases: &["name"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: Some(ReadFn::Bare(|d: &PanelTypeInternalData| {
        let name = if d.data.prefix.is_empty() {
            d.data.panel_kind.clone()
        } else if d.data.panel_kind.is_empty() {
            d.data.prefix.clone()
        } else {
            format!("{} ({})", d.data.panel_kind, d.data.prefix)
        };
        Some(FieldValue::String(name))
    })),
    write_fn: None, // Read-only computed field
    index_fn: Some(|query, d: &PanelTypeInternalData| {
        let q = query.to_lowercase();
        // Can match against prefix or panel_kind
        let p = d.data.prefix.to_lowercase();
        let k = d.data.panel_kind.to_lowercase();

        if p == q || k == q {
            Some(MatchPriority::Exact)
        } else if p.starts_with(&q) || k.starts_with(&q) {
            Some(MatchPriority::Prefix)
        } else if p.contains(&q) || k.contains(&q) {
            Some(MatchPriority::Contains)
        } else {
            None
        }
    }),
    verify_fn: None,
};

// Panels of this type — edge-backed computed field (deferred to FEATURE-018).
// Populated from edge maps when relationship storage is implemented.
edge_list_field!(FIELD_PANELS, PanelTypeEntityType, PanelTypeInternalData,
    name: "panels", display: "Panels",
    desc: "Panels of this type.",
    aliases: &[]);

// ── FieldSet ────────────────────────────────────────────────────────────────────

/// Static field registry for PanelType entities.
///
/// Assembled manually in a `LazyLock` as specified by the work item.
static PANEL_TYPE_FIELD_SET: LazyLock<FieldSet<PanelTypeEntityType>> = LazyLock::new(|| {
    FieldSet::new(&[
        &FIELD_PREFIX,
        &FIELD_PANEL_KIND,
        &FIELD_HIDDEN,
        &FIELD_IS_WORKSHOP,
        &FIELD_IS_BREAK,
        &FIELD_IS_CAFE,
        &FIELD_IS_ROOM_HOURS,
        &FIELD_IS_TIMELINE,
        &FIELD_IS_PRIVATE,
        &FIELD_COLOR,
        &FIELD_BW,
        &FIELD_DISPLAY_NAME,
        &FIELD_PANELS,
    ])
});

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;
    use crate::value::FieldError;
    use uuid::Uuid;

    fn make_panel_type_id() -> PanelTypeId {
        PanelTypeId::new(Uuid::new_v4()).expect("v4 is never nil")
    }

    fn make_test_internal_data() -> PanelTypeInternalData {
        PanelTypeInternalData {
            data: PanelTypeCommonData {
                prefix: "GP".into(),
                panel_kind: "Guest Panel".into(),
                hidden: false,
                is_workshop: false,
                is_break: false,
                is_cafe: false,
                is_room_hours: false,
                is_timeline: false,
                is_private: false,
                color: Some("#db2777".into()),
                bw: Some("#666666".into()),
            },
            code: make_panel_type_id(),
        }
    }

    fn make_schedule_with_panel_type(id: PanelTypeId, data: PanelTypeInternalData) -> Schedule {
        let mut sched = Schedule::default();
        sched.insert(id, data);
        sched
    }

    // ── FieldSet Lookup ───────────────────────────────────────────────────────

    #[test]
    fn test_field_set_lookup_by_canonical_name() {
        let fs = PanelTypeEntityType::field_set();
        assert!(fs.get_by_name("prefix").is_some());
        assert!(fs.get_by_name("panel_kind").is_some());
        assert!(fs.get_by_name("hidden").is_some());
        assert!(fs.get_by_name("display_name").is_some());
    }

    #[test]
    fn test_field_set_lookup_by_alias() {
        let fs = PanelTypeEntityType::field_set();
        // panel_kind aliases
        assert!(fs.get_by_name("kind").is_some());
        assert!(fs.get_by_name("type_name").is_some());
        // prefix alias
        assert!(fs.get_by_name("uniq_id_prefix").is_some());
        // display_name alias
        assert!(fs.get_by_name("name").is_some());
        // bw aliases
        assert!(fs.get_by_name("bw_color").is_some());
        assert!(fs.get_by_name("monochrome").is_some());
    }

    #[test]
    fn test_field_set_unknown_name_returns_none() {
        let fs = PanelTypeEntityType::field_set();
        assert!(fs.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_field_set_fields_count() {
        let fs = PanelTypeEntityType::field_set();
        let fields: Vec<_> = fs.fields().collect();
        assert_eq!(fields.len(), 13);
    }

    #[test]
    fn test_required_fields() {
        let fs = PanelTypeEntityType::field_set();
        let required: Vec<_> = fs.required_fields().map(|d| d.name).collect();
        assert!(required.contains(&"prefix"));
        assert!(required.contains(&"panel_kind"));
        assert_eq!(required.len(), 2);
    }

    // ── Field Read ────────────────────────────────────────────────────────────

    #[test]
    fn test_read_prefix() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("prefix", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::String("GP".into())));
    }

    #[test]
    fn test_read_panel_kind() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("panel_kind", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::String("Guest Panel".into())));
    }

    #[test]
    fn test_read_hidden() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("hidden", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::Boolean(false)));
    }

    #[test]
    fn test_read_color() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("color", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::String("#db2777".into())));
    }

    #[test]
    fn test_read_display_name_computed() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("display_name", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::String("Guest Panel (GP)".into())));
    }

    #[test]
    fn test_read_display_name_from_alias() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("name", id, &sched).unwrap(); // alias
        assert_eq!(value, Some(FieldValue::String("Guest Panel (GP)".into())));
    }

    #[test]
    fn test_read_panels_edge_field() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("panels", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::List(Vec::new())));
    }

    // ── Field Write ────────────────────────────────────────────────────────────

    #[test]
    fn test_write_prefix() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value("prefix", id, &mut sched, FieldValue::String("SP".into()))
            .unwrap();

        let value = fs.read_field_value("prefix", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::String("SP".into())));
    }

    #[test]
    fn test_write_panel_kind() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value(
            "panel_kind",
            id,
            &mut sched,
            FieldValue::String("Special Panel".into()),
        )
        .unwrap();

        let value = fs.read_field_value("panel_kind", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::String("Special Panel".into())));
    }

    #[test]
    fn test_write_hidden() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value("hidden", id, &mut sched, FieldValue::Boolean(true))
            .unwrap();

        let value = fs.read_field_value("hidden", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::Boolean(true)));
    }

    #[test]
    fn test_write_is_workshop() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value("is_workshop", id, &mut sched, FieldValue::Boolean(true))
            .unwrap();

        let value = fs.read_field_value("is_workshop", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::Boolean(true)));
    }

    #[test]
    fn test_write_color_to_none() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value("color", id, &mut sched, FieldValue::None)
            .unwrap();

        let value = fs.read_field_value("color", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::None));
    }

    #[test]
    fn test_write_color_to_value() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value(
            "color",
            id,
            &mut sched,
            FieldValue::String("#ff0000".into()),
        )
        .unwrap();

        let value = fs.read_field_value("color", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::String("#ff0000".into())));
    }

    #[test]
    fn test_write_readonly_display_name_fails() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let result = fs.write_field_value(
            "display_name",
            id,
            &mut sched,
            FieldValue::String("X".into()),
        );
        assert!(matches!(result, Err(FieldError::ReadOnly { .. })));
    }

    #[test]
    fn test_write_wrong_type_returns_error() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let result = fs.write_field_value("prefix", id, &mut sched, FieldValue::Integer(42));
        assert!(matches!(result, Err(FieldError::Conversion(_))));
    }

    // ── Serialization ───────────────────────────────────────────────────────────

    #[test]
    fn test_common_data_serde_roundtrip() {
        let original = PanelTypeCommonData {
            prefix: "GP".into(),
            panel_kind: "Guest Panel".into(),
            hidden: false,
            is_workshop: true,
            is_break: false,
            is_cafe: false,
            is_room_hours: false,
            is_timeline: false,
            is_private: false,
            color: Some("#db2777".into()),
            bw: Some("#666666".into()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let back: PanelTypeCommonData = serde_json::from_str(&json).unwrap();

        assert_eq!(original, back);
    }

    #[test]
    fn test_data_serde_roundtrip() {
        let id = make_panel_type_id();
        let internal = PanelTypeInternalData {
            data: PanelTypeCommonData {
                prefix: "GP".into(),
                panel_kind: "Guest Panel".into(),
                hidden: false,
                is_workshop: true,
                is_break: false,
                is_cafe: false,
                is_room_hours: false,
                is_timeline: false,
                is_private: false,
                color: Some("#db2777".into()),
                bw: Some("#666666".into()),
            },
            code: id,
        };

        let data = PanelTypeEntityType::export(&internal);
        let json = serde_json::to_string(&data).unwrap();
        let back: PanelTypeData = serde_json::from_str(&json).unwrap();

        assert_eq!(data, back);
        assert_eq!(back.code, id.to_string());
    }

    #[test]
    fn test_common_data_default() {
        let default = PanelTypeCommonData::default();
        assert!(default.prefix.is_empty());
        assert!(default.panel_kind.is_empty());
        assert!(!default.hidden);
        assert!(!default.is_workshop);
        assert!(default.color.is_none());
    }

    #[test]
    fn test_common_data_validate_empty() {
        let data = PanelTypeCommonData::default();
        let errors = data.validate();
        assert_eq!(errors.len(), 2);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::Required { field } if *field == "prefix")));
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::Required { field } if *field == "panel_kind")));
    }

    #[test]
    fn test_common_data_validate_valid() {
        let data = PanelTypeCommonData {
            prefix: "GP".into(),
            panel_kind: "Guest Panel".into(),
            ..Default::default()
        };
        let errors = data.validate();
        assert!(errors.is_empty());
    }

    // ── Index/Match ─────────────────────────────────────────────────────────────

    #[test]
    fn test_match_prefix_exact() {
        let data = PanelTypeInternalData {
            data: PanelTypeCommonData {
                prefix: "GP".into(),
                panel_kind: "Guest Panel".into(),
                ..Default::default()
            },
            code: make_panel_type_id(),
        };

        let fs = PanelTypeEntityType::field_set();
        let priority = fs.match_index("gp", &data);
        assert_eq!(priority, Some(MatchPriority::Exact));
    }

    #[test]
    fn test_match_panel_kind_prefix() {
        let data = PanelTypeInternalData {
            data: PanelTypeCommonData {
                prefix: "GP".into(),
                panel_kind: "Guest Panel".into(),
                ..Default::default()
            },
            code: make_panel_type_id(),
        };

        let fs = PanelTypeEntityType::field_set();
        let priority = fs.match_index("guest", &data);
        assert_eq!(priority, Some(MatchPriority::Prefix));
    }

    #[test]
    fn test_match_display_name_no_match() {
        let data = PanelTypeInternalData {
            data: PanelTypeCommonData {
                prefix: "GP".into(),
                panel_kind: "Guest Panel".into(),
                ..Default::default()
            },
            code: make_panel_type_id(),
        };

        let fs = PanelTypeEntityType::field_set();
        let priority = fs.match_index("xyz", &data);
        assert_eq!(priority, None);
    }
}
