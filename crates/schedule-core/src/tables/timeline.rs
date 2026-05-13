/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Timeline entity — timeline event type.
//!
//! Three structs define the Timeline entity:
//!
//! - [`TimelineCommonData`] — user-facing fields
//! - [`TimelineInternalData`] — `EntityType::InternalData`; the field system operates on this
//! - [`TimelineData`] — export/API view produced by [`TimelineEntityType::export`]
//!
//! Timelines are distinct from panels in that they have a specific time point
//! rather than a duration range.

use crate::accessor_field_properties;
use crate::callback_field_properties;
use crate::entity::{EntityId, EntityType, EntityUuid, FieldSet};
use crate::field::{CollectedField, CollectedHalfEdge, FieldDescriptor, NamedField};
use crate::field_value;
use crate::tables::panel_type::{self, PanelTypeEntityType};
use crate::value::uniq_id::PanelUniqId;
use crate::value::{FieldCardinality, FieldType, FieldTypeItem, FieldValue, ValidationError};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── Type Aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for Timeline entities.
pub type TimelineId = EntityId<TimelineEntityType>;

// ── TimelineCommonData ─────────────────────────────────────────────────────────

/// User-facing fields for timeline events.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineCommonData {
    pub name: String,
    pub description: Option<String>,
    pub note: Option<String>,
    pub time: Option<chrono::NaiveDateTime>,
}

impl TimelineCommonData {
    fn validate(&self) -> Vec<ValidationError> {
        Vec::new()
    }
}

// ── TimelineInternalData ───────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
#[derive(Debug, Clone)]
pub struct TimelineInternalData {
    pub id: TimelineId,
    pub data: TimelineCommonData,
    /// Parsed Uniq ID (e.g. `TL01`). Structurally valid by construction;
    /// callers parse via [`PanelUniqId::parse`] before building this struct.
    pub code: PanelUniqId,
}

// ── TimelineData ─────────────────────────────────────────────────────────────

/// Export/API view produced by [`TimelineEntityType::export`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineData {
    /// Canonical Uniq ID string (e.g. `"TL01"`), from `code.full_id()`.
    pub code: String,
    #[serde(flatten)]
    pub data: TimelineCommonData,
}

// ── TimelineEntityType ───────────────────────────────────────────────────────

/// Singleton type representing the Timeline entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimelineEntityType;

impl EntityType for TimelineEntityType {
    type InternalData = TimelineInternalData;
    type Data = TimelineData;

    const TYPE_NAME: &'static str = "timeline";

    fn uuid_namespace() -> &'static uuid::Uuid {
        static NS: LazyLock<uuid::Uuid> =
            LazyLock::new(|| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, b"timeline"));
        &NS
    }

    fn field_set() -> &'static FieldSet<Self> {
        &TIMELINE_FIELD_SET
    }

    fn export(internal: &Self::InternalData) -> Self::Data {
        TimelineData {
            code: internal.code.full_id(),
            data: internal.data.clone(),
        }
    }

    fn validate(internal: &Self::InternalData) -> Vec<ValidationError> {
        internal.data.validate()
    }
}

inventory::submit! {
    crate::entity::RegisteredEntityType {
        type_name: TimelineEntityType::TYPE_NAME,
        uuid_namespace: TimelineEntityType::uuid_namespace,
        type_id: || std::any::TypeId::of::<TimelineInternalData>(),
        read_field_fn: |schedule, uuid, field_name| {
            // SAFETY: uuid came from an existing TimelineEntityType entity.
            let id = unsafe { crate::entity::EntityId::<TimelineEntityType>::new_unchecked(uuid) };
            TimelineEntityType::field_set().read_field_value(field_name, id, schedule)
        },
        write_field_fn: |schedule, uuid, field_name, value| {
            // SAFETY: uuid came from an existing TimelineEntityType entity.
            let id = unsafe { crate::entity::EntityId::<TimelineEntityType>::new_unchecked(uuid) };
            TimelineEntityType::field_set().write_field_value(field_name, id, schedule, value)
        },
        build_fn: |schedule, uuid, fields| {
            crate::edit::builder::build_entity::<TimelineEntityType>(
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
            // SAFETY: uuid came from an existing TimelineEntityType entity.
            let id = unsafe { crate::entity::EntityId::<TimelineEntityType>::new_unchecked(uuid) };
            TimelineEntityType::field_set()
                .fields()
                .filter(|d| d.cb.read_fn.is_some() && d.cb.write_fn.is_some())
                .filter_map(|d| {
                    d.read(id, schedule).ok().flatten().map(|v| (d.name(), v))
                })
                .collect()
        },
        remove_fn: |schedule, uuid| {
            // SAFETY: uuid came from an existing TimelineEntityType entity.
            let id = unsafe { crate::entity::EntityId::<TimelineEntityType>::new_unchecked(uuid) };
            schedule.remove_entity::<TimelineEntityType>(id);
        },
        rehydrate_fn: |schedule, uuid| {
            crate::crdt::rehydrate_entity::<TimelineEntityType>(schedule, uuid)
        },
    }
}

// ── EntityBuildable ─────────────────────────────────────────────────────────────

impl crate::edit::builder::EntityBuildable for TimelineEntityType {
    fn default_data(id: EntityId<Self>) -> Self::InternalData {
        TimelineInternalData {
            id,
            data: TimelineCommonData::default(),
            code: PanelUniqId::default(),
        }
    }
}

// ── Stored field descriptors ──────────────────────────────────────────────────

/// Timeline `code` (Uniq ID) — stored as the parsed [`PanelUniqId`] on
/// [`TimelineInternalData`], exposed to the field system as a string.
pub static FIELD_CODE: FieldDescriptor<TimelineEntityType> = {
    let (data, crdt_type, cb) = callback_field_properties! {
        TimelineEntityType,
        name: "code",
        display: "Code",
        description: "Timeline code (e.g. \"TL01\"), parsed from the Schedule sheet.",
        aliases: &["uid", "uniq_id", "id"],
        cardinality: Single,
        item: String,
        example: "TL01",
        order: 0,
        read: |d: &TimelineInternalData| {
            Some(field_value!(d.code.full_id()))
        },
        write: |d: &mut TimelineInternalData, v: FieldValue| {
            let s = v.into_string()?;
            // Callers that change the prefix should update the panel_type edge.
            match PanelUniqId::parse(&s) {
                Some(parsed) => {
                    d.code = parsed;
                    Ok(())
                }
                None => Err(crate::value::ConversionError::ParseError {
                    message: format!("could not parse timeline code {s:?}"),
                }
                .into()),
            }
        }
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: true,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_CODE) }

// @todo: Name can be empty, should be optional
pub static FIELD_NAME: FieldDescriptor<TimelineEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        TimelineEntityType,
        name,
        name: "name",
        display: "Name",
        description: "Timeline name / title.",
        aliases: &["title", "timeline_name"],
        cardinality: Single,
        item: String,
        example: "Thursday Morning",
        order: 100,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: true,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_NAME) }

pub static FIELD_DESCRIPTION: FieldDescriptor<TimelineEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        TimelineEntityType,
        description,
        name: "description",
        display: "Description",
        description: "Timeline description.",
        aliases: &["desc"],
        cardinality: Optional,
        item: String,
        example: "Mark the start of stuff on Thursday",
        order: 200,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_DESCRIPTION) }

pub static FIELD_NOTE: FieldDescriptor<TimelineEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        TimelineEntityType,
        note,
        name: "note",
        display: "Note",
        description: "Extra note displayed verbatim.",
        aliases: &[],
        cardinality: Optional,
        item: String,
        example: "Used for generating schedule cards for guests",
        order: 300,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_NOTE) }

pub static FIELD_TIME: FieldDescriptor<TimelineEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        TimelineEntityType,
        time,
        name: "time",
        display: "Time",
        description: "Timeline time point.",
        aliases: &["start_time", "start"],
        cardinality: Optional,
        item: DateTime,
        example: "2026-01-01T09:00:00",
        order: 400,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_TIME) }

// Panel types associated with this timeline.
pub static HALF_EDGE_PANEL_TYPES: crate::edge::HalfEdgeDescriptor = {
    crate::edge::HalfEdgeDescriptor {
        data: crate::field::CommonFieldData {
            name: "panel_types",
            display: "Panel Types",
            description: "Panel types associated with this timeline.",
            aliases: &[],
            field_type: FieldType(
                FieldCardinality::List,
                FieldTypeItem::EntityIdentifier(PanelTypeEntityType::TYPE_NAME),
            ),
            example: "[]",
            order: 500,
        },
        edge_kind: crate::edge::EdgeKind::Owner {
            target_field: &panel_type::HALF_EDGE_TIMELINES,
            exclusive_with: None,
        },
        entity_name: TimelineEntityType::TYPE_NAME,
    }
};
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_PANEL_TYPES) }

/// Full edge from timeline panel types to panel type timelines
pub const EDGE_PANEL_TYPES: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_PANEL_TYPES,
    far: &panel_type::HALF_EDGE_TIMELINES,
};

// ── FieldSet ───────────────────────────────────────────────────────────────────

static TIMELINE_FIELD_SET: LazyLock<FieldSet<TimelineEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ── Builder ───────────────────────────────────────────────────────────────────

crate::field::macros::define_entity_builder! {
    /// Typed builder for [`TimelineEntityType`] entities.
    TimelineBuilder for TimelineEntityType {
        /// Set the Uniq ID code (e.g. `"TL01"`). Required.
        with_code        => FIELD_CODE,
        /// Set the timeline name. Required.
        with_name        => FIELD_NAME,
        /// Set the timeline description.
        with_description => FIELD_DESCRIPTION,
        /// Set the timeline note.
        with_note        => FIELD_NOTE,
        /// Set the timeline time point.
        with_time        => FIELD_TIME,
        /// Set the panel types associated with this timeline.
        with_panel_types => HALF_EDGE_PANEL_TYPES,
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_value;
    use crate::schedule::Schedule;
    use uuid::Uuid;

    fn make_timeline_id() -> TimelineId {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        unsafe { TimelineId::new_unchecked(non_nil_uuid) }
    }

    fn make_test_internal_data() -> TimelineInternalData {
        TimelineInternalData {
            id: make_timeline_id(),
            data: TimelineCommonData {
                name: "Opening Ceremony".into(),
                description: Some("Opening ceremony for the convention".into()),
                note: Some("Main ballroom".into()),
                time: None,
            },
            code: PanelUniqId {
                prefix: "TL".into(),
                prefix_num: 1,
                part_num: None,
                session_num: None,
                suffix: None,
            },
        }
    }

    fn make_schedule_with_timeline(id: TimelineId, data: TimelineInternalData) -> Schedule {
        let mut sched = Schedule::default();
        sched.insert(id, data);
        sched
    }

    // ── FieldSet Lookup ───────────────────────────────────────────────────────

    #[test]
    fn test_field_set_half_edges() {
        let fs = TimelineEntityType::field_set();

        // Test that half-edges are included
        let half_edges: Vec<_> = fs.half_edges().collect();
        assert_eq!(half_edges.len(), 1);
        assert_eq!(half_edges[0].data.name, "panel_types");
    }

    // ── Field Read/Write ─────────────────────────────────────────────────────

    #[test]
    fn test_read_field_code() {
        let id = make_timeline_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_timeline(id, data);

        let fs = TimelineEntityType::field_set();
        let value = fs.read_field_value("code", id, &sched).unwrap();

        assert_eq!(value, Some(field_value!("TL001")));
    }

    #[test]
    fn test_read_field_name() {
        let id = make_timeline_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_timeline(id, data);

        let fs = TimelineEntityType::field_set();
        let value = fs.read_field_value("name", id, &sched).unwrap();

        assert_eq!(value, Some(field_value!("Opening Ceremony")));
    }

    #[test]
    fn test_write_field_code() {
        let id = make_timeline_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_timeline(id, data);

        let fs = TimelineEntityType::field_set();
        fs.write_field_value("code", id, &mut sched, field_value!("TL002"))
            .unwrap();

        let value = fs.read_field_value("code", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!("TL002")));
    }

    #[test]
    fn test_common_data_serde_roundtrip() {
        let original = TimelineCommonData {
            name: "Opening Ceremony".into(),
            description: Some("Opening ceremony for the convention".into()),
            note: Some("Main ballroom".into()),
            time: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: TimelineCommonData = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_validate_missing_name() {
        let data = TimelineCommonData::default();
        let errors = data.validate();
        // Timelines do not require names
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_valid_data() {
        let data = TimelineCommonData {
            name: "Test Timeline".into(),
            ..Default::default()
        };
        let errors = data.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_export() {
        let id = make_timeline_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_timeline(id, data);

        let internal = sched.get_internal::<TimelineEntityType>(id).unwrap();
        let exported = TimelineEntityType::export(internal);
        assert_eq!(exported.code, "TL001");
        assert_eq!(exported.data.name, "Opening Ceremony");
        assert_eq!(
            exported.data.description,
            Some("Opening ceremony for the convention".into())
        );
        assert_eq!(exported.data.note, Some("Main ballroom".into()));
        assert_eq!(exported.data.time, None);
    }
}
