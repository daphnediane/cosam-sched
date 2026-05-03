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

use crate::accessor_field_properties;
use crate::callback_field_properties;
use crate::edge::EdgeKind;
use crate::entity::{EntityId, EntityType, EntityUuid, UuidPreference};
use crate::field::set::FieldSet;
use crate::field::{CollectedNamedField, FieldDescriptor, NamedField};
use crate::field_value;
use crate::query::converter::EntityStringResolver;
use crate::tables::panel::{PanelEntityType, PanelId};
use crate::value::ValidationError;
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
    pub id: PanelTypeId,
    pub data: PanelTypeCommonData,
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
    /// Panels of this type — assembled from edge maps.
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
            panels: Vec::new(), // Edge-backed; read via field system
        }
    }

    fn validate(internal: &Self::InternalData) -> Vec<ValidationError> {
        internal.data.validate()
    }
}

inventory::submit! {
    crate::entity::RegisteredEntityType {
        type_name: PanelTypeEntityType::TYPE_NAME,
        uuid_namespace: PanelTypeEntityType::uuid_namespace,
        type_id: || std::any::TypeId::of::<PanelTypeInternalData>(),
        read_field_fn: |schedule, uuid, field_name| {
            // SAFETY: uuid came from an existing PanelTypeEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PanelTypeEntityType>::new_unchecked(uuid) };
            PanelTypeEntityType::field_set().read_field_value(field_name, id, schedule)
        },
        write_field_fn: |schedule, uuid, field_name, value| {
            // SAFETY: uuid came from an existing PanelTypeEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PanelTypeEntityType>::new_unchecked(uuid) };
            PanelTypeEntityType::field_set().write_field_value(field_name, id, schedule, value)
        },
        build_fn: |schedule, uuid, fields| {
            crate::edit::builder::build_entity::<PanelTypeEntityType>(
                schedule,
                crate::entity::UuidPreference::Exact(uuid),
                fields
                    .iter()
                    .map(|(n, v)| crate::field::set::FieldUpdate {
                        op: crate::field::set::FieldOp::Set,
                        field: crate::field::set::FieldRef::Name(n),
                        value: v.clone(),
                    })
                    .collect(),
            )
            .map(|id| id.entity_uuid())
        },
        snapshot_fn: |schedule, uuid| {
            use crate::field::ReadableField;
            // SAFETY: uuid came from an existing PanelTypeEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PanelTypeEntityType>::new_unchecked(uuid) };
            PanelTypeEntityType::field_set()
                .fields()
                .filter(|d| d.cb.read_fn.is_some() && d.cb.write_fn.is_some())
                .filter_map(|d| {
                    d.read(id, schedule).ok().flatten().map(|v| (d.name(), v))
                })
                .collect()
        },
        remove_fn: |schedule, uuid| {
            // SAFETY: uuid came from an existing PanelTypeEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PanelTypeEntityType>::new_unchecked(uuid) };
            schedule.remove_entity::<PanelTypeEntityType>(id);
        },
        rehydrate_fn: |schedule, uuid| {
            crate::crdt::rehydrate_entity::<PanelTypeEntityType>(schedule, uuid)
        },
    }
}

// ── EntityBuildable ─────────────────────────────────────────────────────────────

impl crate::edit::builder::EntityBuildable for PanelTypeEntityType {
    fn default_data(id: EntityId<Self>) -> Self::InternalData {
        PanelTypeInternalData {
            id,
            data: PanelTypeCommonData::default(),
        }
    }
}

// ── EntityStringResolver implementation ─────────────────────────────────────────

impl EntityStringResolver for PanelTypeEntityType {
    fn entity_to_string(schedule: &crate::schedule::Schedule, id: EntityId<Self>) -> String {
        schedule
            .get_internal(id)
            .map(|data| data.data.panel_kind.clone())
            .unwrap_or_else(|| id.to_string())
    }
}

// ── Field Descriptors ──────────────────────────────────────────────────────────

pub static FIELD_PREFIX: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        prefix,
        name: "prefix",
        display: "Prefix",
        description: "Two-letter Uniq ID prefix for panels of this type.",
        aliases: &["uniq_id_prefix"],
        cardinality: Single,
        item: String,
        example: "GP",
        order: 0,
    };
    FieldDescriptor {
        data,
        required: true,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_PREFIX) }

pub static FIELD_PANEL_KIND: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        panel_kind,
        name: "panel_kind",
        display: "Panel Kind",
        description: "Human-readable kind name for this panel type.",
        aliases: &["kind", "type_name"],
        cardinality: Single,
        item: String,
        example: "Guest Panel",
        order: 100,
    };
    FieldDescriptor {
        data,
        required: true,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_PANEL_KIND) }

pub static FIELD_HIDDEN: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        hidden,
        name: "hidden",
        display: "Hidden",
        description: "Whether this panel type is hidden from UI.",
        aliases: &[],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 200,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_HIDDEN) }

pub static FIELD_IS_WORKSHOP: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        is_workshop,
        name: "is_workshop",
        display: "Is Workshop",
        description: "Whether panels of this type are workshops.",
        aliases: &["workshop"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 300,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_IS_WORKSHOP) }

pub static FIELD_IS_BREAK: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        is_break,
        name: "is_break",
        display: "Is Break",
        description: "Whether panels of this type are break periods.",
        aliases: &["break"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 400,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_IS_BREAK) }

pub static FIELD_IS_CAFE: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        is_cafe,
        name: "is_cafe",
        display: "Is Cafe",
        description: "Whether panels of this type are cafe events.",
        aliases: &["cafe"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 500,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_IS_CAFE) }

pub static FIELD_IS_ROOM_HOURS: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        is_room_hours,
        name: "is_room_hours",
        display: "Is Room Hours",
        description: "Whether panels of this type are room hours.",
        aliases: &["room_hours"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 600,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_IS_ROOM_HOURS) }

pub static FIELD_IS_TIMELINE: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        is_timeline,
        name: "is_timeline",
        display: "Is Timeline",
        description: "Whether panels of this type are timeline events.",
        aliases: &["timeline"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 700,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_IS_TIMELINE) }

pub static FIELD_IS_PRIVATE: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        is_private,
        name: "is_private",
        display: "Is Private",
        description: "Whether panels of this type are private events.",
        aliases: &["private"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 800,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_IS_PRIVATE) }

pub static FIELD_COLOR: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        color,
        name: "color",
        display: "Color",
        description: "CSS color for panels of this type.",
        aliases: &[],
        cardinality: Optional,
        item: String,
        example: "#db2777",
        order: 900,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_COLOR) }

pub static FIELD_BW: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelTypeEntityType,
        bw,
        name: "bw",
        display: "BW Color",
        description: "Alternate monochrome color for panels of this type.",
        aliases: &["bw_color", "monochrome"],
        cardinality: Optional,
        item: String,
        example: "#666666",
        order: 1000,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_BW) }

/// Computed display name — derived from `panel_kind` and `prefix`.
///
/// Read-only computed field that produces a human-readable identifier.
pub static FIELD_DISPLAY_NAME: FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb) = callback_field_properties! {
        PanelTypeEntityType,
        name: "display_name",
        display: "Display Name",
        description: "Human-readable display name combining kind and prefix.",
        aliases: &["name"],
        cardinality: Single,
        item: String,
        example: "Guest Panel (GP)",
        order: 1100,
        read: |d: &PanelTypeInternalData| {
            let name = if d.data.prefix.is_empty() {
                d.data.panel_kind.clone()
            } else if d.data.panel_kind.is_empty() {
                d.data.prefix.clone()
            } else {
                format!("{} ({})", d.data.panel_kind, d.data.prefix)
            };
            Some(field_value!(name))
        }
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_DISPLAY_NAME) }

// Panels of this type — reverse heterogeneous edge from Panel → PanelType.
pub static HALF_EDGE_PANELS: crate::field::FieldDescriptor<PanelTypeEntityType> = {
    let (data, cb, edge_kind) = crate::edge_field_properties! {
        PanelTypeEntityType,
        target: PanelEntityType,
        source_fields: &[&crate::tables::panel::HALF_EDGE_PANEL_TYPE],
        name: "panels",
        display: "Panels",
        description: "Panels of this type.",
        aliases: &[],
        example: "[]",
        order: 1200,
    };
    crate::field::FieldDescriptor {
        data,
        required: false,
        edge_kind,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&HALF_EDGE_PANELS) }

// Temporary alias for migration - remove when edit_integration.rs is updated
#[allow(deprecated)]
pub use HALF_EDGE_PANELS as FIELD_PANELS;

/// Full edge from panel type panels to panel panel type
pub const EDGE_PANELS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_PANELS,
    far: &crate::tables::panel::HALF_EDGE_PANEL_TYPE,
};

// ── FieldSet ────────────────────────────────────────────────────────────────────

static PANEL_TYPE_FIELD_SET: LazyLock<FieldSet<PanelTypeEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ── Builder ─────────────────────────────────────────────────────────────────────

crate::field::macros::define_entity_builder! {
    /// Typed builder for [`PanelTypeEntityType`] entities, generated by the
    /// `define_entity_builder!` macro.
    PanelTypeBuilder for PanelTypeEntityType {
        /// Set the two-letter Uniq ID prefix (e.g. `"GP"`, `"SP"`).  Required.
        with_prefix        => FIELD_PREFIX,
        /// Set the human-readable kind name for this panel type
        /// (e.g. `"Guest Panel"`).  Required.
        with_panel_kind    => FIELD_PANEL_KIND,
        /// Hide panels of this type from UI listings.
        with_hidden        => FIELD_HIDDEN,
        /// Mark panels of this type as workshops.
        with_is_workshop   => FIELD_IS_WORKSHOP,
        /// Mark panels of this type as break periods.
        with_is_break      => FIELD_IS_BREAK,
        /// Mark panels of this type as cafe events.
        with_is_cafe       => FIELD_IS_CAFE,
        /// Mark panels of this type as room-hours rows.
        with_is_room_hours => FIELD_IS_ROOM_HOURS,
        /// Mark panels of this type as timeline events.
        with_is_timeline   => FIELD_IS_TIMELINE,
        /// Mark panels of this type as private events.
        with_is_private    => FIELD_IS_PRIVATE,
        /// Set the CSS color for color-mode rendering.
        with_color         => FIELD_COLOR,
        /// Set the alternate monochrome color for black-and-white rendering.
        with_bw            => FIELD_BW,
    }
}

// ── EntityMatcher ────────────────────────────────────────────────────────────────

impl crate::query::lookup::EntityScannable for PanelTypeEntityType {}

impl crate::query::lookup::EntityMatcher for PanelTypeEntityType {
    fn can_create(full: &str, partial: &str) -> crate::query::lookup::CanCreate {
        if partial.is_empty() {
            crate::query::lookup::CanCreate::No
        } else if full == partial {
            crate::query::lookup::CanCreate::Yes(crate::query::lookup::MatchConsumed::Full)
        } else {
            crate::query::lookup::CanCreate::Yes(crate::query::lookup::MatchConsumed::Partial)
        }
    }

    fn match_entity(
        query: &str,
        data: &PanelTypeInternalData,
    ) -> Option<crate::query::lookup::MatchPriority> {
        use crate::query::lookup::string_match_priority;
        // Match on prefix (e.g. "GP"), kind (e.g. "General Programming"),
        // and the combined display form (e.g. "GP General Programming").
        let display = format!("{} {}", data.data.prefix, data.data.panel_kind);
        [
            string_match_priority(query, &data.data.prefix),
            string_match_priority(query, &data.data.panel_kind),
            string_match_priority(query, &display),
        ]
        .into_iter()
        .flatten()
        .max()
    }
}

// ── EntityCreatable ───────────────────────────────────────────────────────────

impl crate::query::lookup::EntityCreatable for PanelTypeEntityType {
    fn create_from_string(
        schedule: &mut crate::schedule::Schedule,
        s: &str,
    ) -> Result<EntityId<Self>, crate::query::lookup::LookupError> {
        let prefix: String = s.chars().take(2).collect();
        let id = EntityId::from_preference(UuidPreference::FromV5 {
            name: s.to_string(),
        });
        schedule.insert(
            id,
            PanelTypeInternalData {
                id,
                data: PanelTypeCommonData {
                    prefix,
                    panel_kind: s.to_string(),
                    ..Default::default()
                },
            },
        );
        Ok(id)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::lookup::{match_priority, EntityMatcher};
    use crate::schedule::Schedule;
    use crate::value::FieldError;
    use uuid::Uuid;

    fn make_panel_type_id() -> PanelTypeId {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        unsafe { PanelTypeId::new_unchecked(non_nil_uuid) }
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
            id: make_panel_type_id(),
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
        let required: Vec<_> = fs.required_fields().map(|d| d.name()).collect();
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
        assert_eq!(value, Some(field_value!("GP")));
    }

    #[test]
    fn test_read_panel_kind() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("panel_kind", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!("Guest Panel")));
    }

    #[test]
    fn test_read_hidden() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("hidden", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!(false)));
    }

    #[test]
    fn test_read_color() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("color", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!("#db2777")));
    }

    #[test]
    fn test_read_display_name_computed() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("display_name", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!("Guest Panel (GP)")));
    }

    #[test]
    fn test_read_display_name_from_alias() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("name", id, &sched).unwrap(); // alias
        assert_eq!(value, Some(field_value!("Guest Panel (GP)")));
    }

    #[test]
    fn test_read_panels_edge_field() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let value = fs.read_field_value("panels", id, &sched).unwrap();
        assert_eq!(value, Some(crate::field_empty_list!()));
    }

    // ── Field Write ────────────────────────────────────────────────────────────

    #[test]
    fn test_write_prefix() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value("prefix", id, &mut sched, field_value!("SP"))
            .unwrap();

        let value = fs.read_field_value("prefix", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!("SP")));
    }

    #[test]
    fn test_write_panel_kind() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value("panel_kind", id, &mut sched, field_value!("Special Panel"))
            .unwrap();

        let value = fs.read_field_value("panel_kind", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!("Special Panel")));
    }

    #[test]
    fn test_write_hidden() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value("hidden", id, &mut sched, field_value!(true))
            .unwrap();

        let value = fs.read_field_value("hidden", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!(true)));
    }

    #[test]
    fn test_write_is_workshop() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value("is_workshop", id, &mut sched, field_value!(true))
            .unwrap();

        let value = fs.read_field_value("is_workshop", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!(true)));
    }

    #[test]
    fn test_write_color_to_none() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        fs.write_field_value("color", id, &mut sched, crate::field_empty_list!())
            .unwrap();

        let value = fs.read_field_value("color", id, &sched).unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_write_color_to_value() {}

    #[test]
    fn test_write_readonly_display_name_fails() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        let result = fs.write_field_value("display_name", id, &mut sched, field_value!("X"));
        assert!(matches!(result, Err(FieldError::ReadOnly { .. })));
    }

    #[test]
    fn test_write_wrong_type_converts_with_cross_type_support() {
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);

        let fs = PanelTypeEntityType::field_set();
        // Integer now converts to String via cross-type conversion
        let result = fs.write_field_value("prefix", id, &mut sched, field_value!(42));
        assert!(result.is_ok());
        assert_eq!(
            sched
                .get_internal::<PanelTypeEntityType>(id)
                .unwrap()
                .data
                .prefix,
            "42"
        );
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
            id,
        };

        let data = PanelTypeEntityType::export(&internal);
        let json = serde_json::to_string(&data).unwrap();
        let back: PanelTypeData = serde_json::from_str(&json).unwrap();

        assert_eq!(data, back);
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
        let data = PanelTypeCommonData {
            prefix: String::new(),
            panel_kind: String::new(),
            ..Default::default()
        };
        let errors = data.validate();
        assert_eq!(errors.len(), 1);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::Required { field } if *field == "prefix")));
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

    #[test]
    fn test_entity_to_string_returns_panel_kind() {
        use crate::query::converter::EntityStringResolver;
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_panel_type(id, data);
        let s = PanelTypeEntityType::entity_to_string(&sched, id);
        assert_eq!(s, "Guest Panel");
    }

    #[test]
    fn test_entity_to_string_fallback_to_uuid() {
        use crate::query::converter::EntityStringResolver;
        let id = make_panel_type_id();
        let sched = Schedule::default();
        let s = PanelTypeEntityType::entity_to_string(&sched, id);
        assert_eq!(s, id.to_string());
    }

    #[test]
    fn test_lookup_or_create_single_creates_new_entity() {
        use crate::query::lookup::lookup_or_create_single;
        let mut sched = Schedule::default();
        let id =
            lookup_or_create_single::<PanelTypeEntityType>(&mut sched, "New Panel Type").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.panel_kind, "New Panel Type");
    }

    #[test]
    fn test_lookup_or_create_single_returns_existing() {
        use crate::query::lookup::lookup_or_create_single;
        let id = make_panel_type_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_panel_type(id, data);
        let found_id =
            lookup_or_create_single::<PanelTypeEntityType>(&mut sched, "Guest Panel").unwrap();
        assert_eq!(found_id, id);
    }

    // ── Index/Match ─────────────────────────────────────────────────────────────

    #[test]
    fn test_match_prefix_exact() {
        let data = make_test_internal_data();
        let priority = PanelTypeEntityType::match_entity("gp", &data);
        assert_eq!(priority, Some(match_priority::EXACT_MATCH));
    }

    #[test]
    fn test_match_panel_kind_starts_with() {
        let data = make_test_internal_data();
        let priority = PanelTypeEntityType::match_entity("guest", &data);
        assert_eq!(priority, Some(match_priority::STRONG_MATCH));
    }

    #[test]
    fn test_match_no_match() {
        let data = make_test_internal_data();
        let priority = PanelTypeEntityType::match_entity("xyz", &data);
        assert_eq!(priority, None);
    }

    // ── EntityCreatable ──────────────────────────────────────────────────────

    #[test]
    fn test_create_from_string_uses_first_two_chars_as_prefix() {
        use crate::query::lookup::EntityCreatable;
        let mut sched = Schedule::default();
        let id = PanelTypeEntityType::create_from_string(&mut sched, "Guest Panel").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.prefix, "Gu");
        assert_eq!(data.data.panel_kind, "Guest Panel");
    }

    #[test]
    fn test_create_from_string_is_deterministic() {
        use crate::query::lookup::EntityCreatable;
        let mut sched1 = Schedule::default();
        let mut sched2 = Schedule::default();
        let id1 = PanelTypeEntityType::create_from_string(&mut sched1, "Guest Panel").unwrap();
        let id2 = PanelTypeEntityType::create_from_string(&mut sched2, "Guest Panel").unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_can_create_no_separator() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            PanelTypeEntityType::can_create("Guest Panel", "Guest Panel"),
            CanCreate::Yes(crate::query::lookup::MatchConsumed::Full)
        ));
    }

    #[test]
    fn test_can_create_with_separator() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            PanelTypeEntityType::can_create("Guest Panel, Staff", "Guest Panel"),
            CanCreate::Yes(crate::query::lookup::MatchConsumed::Partial)
        ));
    }

    // ── PanelTypeBuilder (generated by define_entity_builder!) ──

    #[test]
    fn panel_type_builder_builds_with_required_fields() {
        let mut sched = Schedule::default();
        let id = PanelTypeBuilder::new()
            .with_prefix("GP")
            .with_panel_kind("Guest Panel")
            .with_color(Some("#db2777".to_owned()))
            .build(&mut sched)
            .unwrap();

        let data = sched.get_internal::<PanelTypeEntityType>(id).unwrap();
        assert_eq!(data.data.prefix, "GP");
        assert_eq!(data.data.panel_kind, "Guest Panel");
        assert_eq!(data.data.color.as_deref(), Some("#db2777"));
    }

    #[test]
    fn panel_type_builder_default_matches_new() {
        let a = PanelTypeBuilder::default()
            .with_prefix("GP")
            .with_panel_kind("Guest Panel");
        let b = PanelTypeBuilder::new()
            .with_prefix("GP")
            .with_panel_kind("Guest Panel");
        let mut sched = Schedule::default();
        assert!(a.build(&mut sched).is_ok());
        assert!(b.build(&mut sched).is_ok());
    }

    #[test]
    fn panel_type_builder_uuid_preference_is_honored() {
        let mut sched1 = Schedule::default();
        let id1 = PanelTypeBuilder::new()
            .with_uuid_preference(UuidPreference::FromV5 { name: "GP".into() })
            .with_prefix("GP")
            .with_panel_kind("Guest Panel")
            .build(&mut sched1)
            .unwrap();

        let mut sched2 = Schedule::default();
        let id2 = PanelTypeBuilder::new()
            .with_uuid_preference(UuidPreference::FromV5 { name: "GP".into() })
            .with_prefix("GP")
            .with_panel_kind("Guest Panel")
            .build(&mut sched2)
            .unwrap();

        assert_eq!(id1.entity_uuid(), id2.entity_uuid());
    }

    #[test]
    fn panel_type_builder_missing_required_rolls_back() {
        use crate::edit::builder::BuildError;
        let mut sched = Schedule::default();
        let err = PanelTypeBuilder::new()
            .with_panel_kind("Guest Panel")
            .build(&mut sched)
            .unwrap_err();
        assert!(matches!(err, BuildError::Validation(_)));
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
    }

    #[test]
    fn panel_type_builder_apply_to_existing_entity() {
        let mut sched = Schedule::default();
        let id = PanelTypeBuilder::new()
            .with_prefix("GP")
            .with_panel_kind("Guest Panel")
            .build(&mut sched)
            .unwrap();

        PanelTypeBuilder::new()
            .with_color(Some("#000000".to_owned()))
            .with_hidden(true)
            .apply_to(id, &mut sched)
            .unwrap();

        let data = sched.get_internal::<PanelTypeEntityType>(id).unwrap();
        assert_eq!(data.data.color.as_deref(), Some("#000000"));
        assert!(data.data.hidden);
        // Unchanged fields retain their original values.
        assert_eq!(data.data.panel_kind, "Guest Panel");
    }
}
