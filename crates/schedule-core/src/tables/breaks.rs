/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Break entity — convention-wide break period.
//!
//! Three structs define the Break entity:
//!
//! - [`BreakCommonData`] — user-facing fields
//! - [`BreakInternalData`] — `EntityType::InternalData`; the field system operates on this
//! - [`BreakData`] — export/API view produced by [`BreakEntityType::export`]
//!
//! Breaks parallel [`TimelineEntityType`](crate::tables::timeline) but, unlike a
//! timeline (a single time point), a break carries a duration — its timing is
//! held in a [`TimeRange`] backing field (`time_slot`) exposed through computed
//! `start_time`, `end_time`, and `duration` fields, mirroring
//! [`PanelEntityType`](crate::tables::panel). Breaks are not panels: they are
//! not assigned to rooms and are excluded from per-room rendering.

use crate::accessor_field_properties;
use crate::callback_field_properties;
use crate::entity::{EntityId, EntityType, EntityUuid, FieldSet};
use crate::field::{CollectedField, CollectedHalfEdge, FieldDescriptor, NamedField};
use crate::field_value;
use crate::tables::panel_type::{self, PanelTypeEntityType};
use crate::value::time::{parse_datetime, parse_duration, TimeRange};
use crate::value::uniq_id::PanelUniqId;
use crate::value::{
    FieldCardinality, FieldType, FieldTypeItem, FieldValue, FieldValueItem, ValidationError,
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── Type Aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for Break entities.
pub type BreakId = EntityId<BreakEntityType>;

// ── BreakCommonData ────────────────────────────────────────────────────────────

/// User-facing fields for break periods.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakCommonData {
    pub name: String,
    pub description: Option<String>,
    pub note: Option<String>,
}

impl BreakCommonData {
    fn validate(&self) -> Vec<ValidationError> {
        Vec::new()
    }
}

// ── BreakInternalData ──────────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
#[derive(Debug, Clone)]
pub struct BreakInternalData {
    pub id: BreakId,
    pub data: BreakCommonData,
    /// Parsed Uniq ID (e.g. `BREAK001`). Structurally valid by construction;
    /// callers parse via [`PanelUniqId::parse`] before building this struct.
    pub code: PanelUniqId,
    /// Break timing — start plus duration or end (a [`TimeRange`]).
    pub time_slot: TimeRange,
}

// ── BreakData ──────────────────────────────────────────────────────────────────

/// Export/API view produced by [`BreakEntityType::export`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakData {
    /// Canonical Uniq ID string (e.g. `"BREAK001"`), from `code.full_id()`.
    pub code: String,
    #[serde(flatten)]
    pub data: BreakCommonData,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start_time: Option<chrono::NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub end_time: Option<chrono::NaiveDateTime>,
    /// Duration in whole minutes (serialized); converted to/from [`Duration`].
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        serialize_with = "serialize_opt_duration_minutes",
        deserialize_with = "deserialize_opt_duration_minutes"
    )]
    pub duration: Option<Duration>,
}

fn serialize_opt_duration_minutes<S: serde::Serializer>(
    value: &Option<Duration>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(d) => s.serialize_some(&d.num_minutes()),
        None => s.serialize_none(),
    }
}

fn deserialize_opt_duration_minutes<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Duration>, D::Error> {
    let opt: Option<i64> = Option::deserialize(d)?;
    Ok(opt.map(Duration::minutes))
}

// ── BreakEntityType ────────────────────────────────────────────────────────────

/// Singleton type representing the Break entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BreakEntityType;

impl EntityType for BreakEntityType {
    type InternalData = BreakInternalData;
    type Data = BreakData;

    const TYPE_NAME: &'static str = "break";

    fn uuid_namespace() -> &'static uuid::Uuid {
        static NS: LazyLock<uuid::Uuid> =
            LazyLock::new(|| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, b"break"));
        &NS
    }

    fn field_set() -> &'static FieldSet<Self> {
        &BREAK_FIELD_SET
    }

    fn export(internal: &Self::InternalData) -> Self::Data {
        BreakData {
            code: internal.code.full_id(),
            data: internal.data.clone(),
            start_time: internal.time_slot.start_time(),
            end_time: internal.time_slot.end_time(),
            duration: internal.time_slot.duration(),
        }
    }

    fn validate(internal: &Self::InternalData) -> Vec<ValidationError> {
        let mut errors = internal.data.validate();
        if let Err(msg) = internal.time_slot.validate() {
            errors.push(ValidationError::Constraint {
                field: "time_slot",
                message: msg,
            });
        }
        errors
    }
}

inventory::submit! {
    crate::entity::RegisteredEntityType {
        type_name: BreakEntityType::TYPE_NAME,
        uuid_namespace: BreakEntityType::uuid_namespace,
        type_id: || std::any::TypeId::of::<BreakInternalData>(),
        read_field_fn: |schedule, uuid, field_name| {
            // SAFETY: uuid came from an existing BreakEntityType entity.
            let id = unsafe { crate::entity::EntityId::<BreakEntityType>::new_unchecked(uuid) };
            BreakEntityType::field_set().read_field_value(field_name, id, schedule)
        },
        write_field_fn: |schedule, uuid, field_name, value| {
            // SAFETY: uuid came from an existing BreakEntityType entity.
            let id = unsafe { crate::entity::EntityId::<BreakEntityType>::new_unchecked(uuid) };
            BreakEntityType::field_set().write_field_value(field_name, id, schedule, value)
        },
        build_fn: |schedule, uuid, fields| {
            crate::edit::builder::build_entity::<BreakEntityType>(
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
            // SAFETY: uuid came from an existing BreakEntityType entity.
            let id = unsafe { crate::entity::EntityId::<BreakEntityType>::new_unchecked(uuid) };
            BreakEntityType::field_set()
                .fields()
                .filter(|d| d.cb.read_fn.is_some() && d.cb.write_fn.is_some())
                .filter_map(|d| {
                    d.read(id, schedule).ok().flatten().map(|v| (d.name(), v))
                })
                .collect()
        },
        remove_fn: |schedule, uuid| {
            // SAFETY: uuid came from an existing BreakEntityType entity.
            let id = unsafe { crate::entity::EntityId::<BreakEntityType>::new_unchecked(uuid) };
            schedule.remove_entity::<BreakEntityType>(id);
        },
        rehydrate_fn: |schedule, uuid| {
            crate::crdt::rehydrate_entity::<BreakEntityType>(schedule, uuid)
        },
    }
}

// ── Lookup helpers ───────────────────────────────────────────────────────────────

impl BreakEntityType {
    /// Find all live breaks with the given Uniq ID code (case-insensitive).
    ///
    /// Returns all matches; in well-formed data the list has at most one entry,
    /// but duplicate codes are possible in human-authored XLSX files.
    pub fn find_by_code(schedule: &crate::schedule::Schedule, code: &str) -> Vec<BreakId> {
        let upper = code.to_uppercase();
        schedule
            .iter_entities::<Self>()
            .filter_map(|(id, d)| (d.code.full_id().to_uppercase() == upper).then_some(id))
            .collect()
    }
}

// ── EntityBuildable ─────────────────────────────────────────────────────────────

impl crate::edit::builder::EntityBuildable for BreakEntityType {
    fn default_data(id: EntityId<Self>) -> Self::InternalData {
        BreakInternalData {
            id,
            data: BreakCommonData::default(),
            code: PanelUniqId::default(),
            time_slot: TimeRange::default(),
        }
    }

    fn find_by_natural_key(schedule: &crate::schedule::Schedule, key: &str) -> Vec<EntityId<Self>> {
        Self::find_by_code(schedule, key)
    }
}

// ── Stored field descriptors ──────────────────────────────────────────────────

/// Break `code` (Uniq ID) — stored as the parsed [`PanelUniqId`] on
/// [`BreakInternalData`], exposed to the field system as a string.
pub static FIELD_CODE: FieldDescriptor<BreakEntityType> = {
    let (data, crdt_type, cb) = callback_field_properties! {
        BreakEntityType,
        name: "code",
        display: "Code",
        description: "Break code (e.g. \"BREAK001\"), parsed from the Schedule sheet.",
        aliases: &["uid", "uniq_id", "id"],
        cardinality: Single,
        item: String,
        example: "BREAK001",
        order: 0,
        read: |d: &BreakInternalData| {
            Some(field_value!(d.code.full_id()))
        },
        write: |d: &mut BreakInternalData, v: FieldValue| {
            let s = v.into_string()?;
            // Callers that change the prefix should update the panel_type edge.
            match PanelUniqId::parse(&s) {
                Some(parsed) => {
                    d.code = parsed;
                    Ok(())
                }
                None => Err(crate::value::ConversionError::ParseError {
                    message: format!("could not parse break code {s:?}"),
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

pub static FIELD_NAME: FieldDescriptor<BreakEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        BreakEntityType,
        name,
        name: "name",
        display: "Name",
        description: "Break name / title.",
        aliases: &["title", "break_name"],
        cardinality: Single,
        item: String,
        example: "Lunch Break",
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

pub static FIELD_DESCRIPTION: FieldDescriptor<BreakEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        BreakEntityType,
        description,
        name: "description",
        display: "Description",
        description: "Break description.",
        aliases: &["desc"],
        cardinality: Optional,
        item: String,
        example: "Lunch on your own",
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

pub static FIELD_NOTE: FieldDescriptor<BreakEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        BreakEntityType,
        note,
        name: "note",
        display: "Note",
        description: "Extra note displayed verbatim.",
        aliases: &[],
        cardinality: Optional,
        item: String,
        example: "Vendor hall stays open",
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

/// Start time — projected from `time_slot`.
pub static FIELD_START_TIME: FieldDescriptor<BreakEntityType> = {
    let (data, crdt_type, cb) = callback_field_properties! {
        BreakEntityType,
        name: "start_time",
        display: "Start Time",
        description: "Break start time.",
        aliases: &["start", "time"],
        cardinality: Optional,
        item: DateTime,
        example: "2026-06-26T12:00:00",
        order: 400,
        read: |d: &BreakInternalData| {
            d.time_slot.start_time().map(|dt| field_value!(dt))
        },
        write: |d: &mut BreakInternalData, v: FieldValue| {
            match v {
                FieldValue::List(_) | FieldValue::Single(FieldValueItem::Text(_)) => {
                    d.time_slot.remove_start_time()
                }
                FieldValue::Single(FieldValueItem::DateTime(dt)) => {
                    d.time_slot.add_start_time(dt)
                }
                FieldValue::Single(FieldValueItem::String(s)) => match parse_datetime(&s) {
                    Some(dt) => d.time_slot.add_start_time(dt),
                    None => {
                        return Err(crate::value::ConversionError::ParseError {
                            message: format!("could not parse datetime {s:?}"),
                        }
                        .into())
                    }
                },
                _ => {
                    return Err(crate::value::ConversionError::WrongVariant {
                        expected: "DateTime or String",
                        got: "other",
                    }
                    .into());
                }
            }
            Ok(())
        },
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_START_TIME) }

/// End time — projected from `time_slot`.
pub static FIELD_END_TIME: FieldDescriptor<BreakEntityType> = {
    let (data, crdt_type, cb) = callback_field_properties! {
        BreakEntityType,
        name: "end_time",
        display: "End Time",
        description: "Break end time.",
        aliases: &["end"],
        cardinality: Optional,
        item: DateTime,
        example: "2026-06-26T13:00:00",
        order: 500,
        read: |d: &BreakInternalData| {
            d.time_slot.end_time().map(|dt| field_value!(dt))
        },
        write: |d: &mut BreakInternalData, v: FieldValue| {
            match v {
                FieldValue::List(_) | FieldValue::Single(FieldValueItem::Text(_)) => {
                    d.time_slot.remove_end_time()
                }
                FieldValue::Single(FieldValueItem::DateTime(dt)) => {
                    d.time_slot.add_end_time(dt)
                }
                FieldValue::Single(FieldValueItem::String(s)) => match parse_datetime(&s) {
                    Some(dt) => d.time_slot.add_end_time(dt),
                    None => {
                        return Err(crate::value::ConversionError::ParseError {
                            message: format!("could not parse datetime {s:?}"),
                        }
                        .into())
                    }
                },
                _ => {
                    return Err(crate::value::ConversionError::WrongVariant {
                        expected: "DateTime or String",
                        got: "other",
                    }
                    .into());
                }
            }
            Ok(())
        },
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_END_TIME) }

/// Duration — projected from `time_slot`.
pub static FIELD_DURATION: FieldDescriptor<BreakEntityType> = {
    let (data, crdt_type, cb) = callback_field_properties! {
        BreakEntityType,
        name: "duration",
        display: "Duration",
        description: "Break duration.",
        aliases: &[],
        cardinality: Optional,
        item: Duration,
        example: "60",
        order: 600,
        read: |d: &BreakInternalData| {
            d.time_slot.duration().map(|dur| field_value!(dur))
        },
        write: |d: &mut BreakInternalData, v: FieldValue| {
            match v {
                FieldValue::List(_) | FieldValue::Single(FieldValueItem::Text(_)) => {
                    d.time_slot.remove_duration()
                }
                FieldValue::Single(FieldValueItem::Duration(dur)) => {
                    d.time_slot.add_duration(dur)
                }
                FieldValue::Single(FieldValueItem::Integer(m)) => {
                    d.time_slot.add_duration(Duration::minutes(m))
                }
                FieldValue::Single(FieldValueItem::String(s)) => match parse_duration(&s) {
                    Some(dur) => d.time_slot.add_duration(dur),
                    None => {
                        return Err(crate::value::ConversionError::ParseError {
                            message: format!("could not parse duration {s:?}"),
                        }
                        .into())
                    }
                },
                _ => {
                    return Err(crate::value::ConversionError::WrongVariant {
                        expected: "Duration, Integer, or String",
                        got: "other",
                    }
                    .into());
                }
            }
            Ok(())
        },
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_DURATION) }

// Panel types associated with this break.
pub static HALF_EDGE_PANEL_TYPES: crate::edge::HalfEdgeDescriptor = {
    crate::edge::HalfEdgeDescriptor {
        data: crate::field::CommonFieldData {
            name: "panel_types",
            display: "Panel Types",
            description: "Panel types associated with this break.",
            aliases: &[],
            field_type: FieldType(
                FieldCardinality::List,
                FieldTypeItem::EntityIdentifier(PanelTypeEntityType::TYPE_NAME),
            ),
            example: "[]",
            order: 700,
        },
        edge_kind: crate::edge::EdgeKind::Owner {
            target_field: &panel_type::HALF_EDGE_BREAKS,
            exclusive_with: None,
        },
        entity_name: BreakEntityType::TYPE_NAME,
    }
};
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_PANEL_TYPES) }

/// Full edge from break panel types to panel type breaks
pub const EDGE_PANEL_TYPES: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_PANEL_TYPES,
    far: &panel_type::HALF_EDGE_BREAKS,
};

// ── FieldSet ───────────────────────────────────────────────────────────────────

static BREAK_FIELD_SET: LazyLock<FieldSet<BreakEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ── Builder ───────────────────────────────────────────────────────────────────

crate::field::macros::define_entity_builder! {
    /// Typed builder for [`BreakEntityType`] entities.
    BreakBuilder for BreakEntityType {
        /// Set the Uniq ID code (e.g. `"BREAK001"`). Required.
        with_code        => FIELD_CODE,
        /// Set the break name. Required.
        with_name        => FIELD_NAME,
        /// Set the break description.
        with_description => FIELD_DESCRIPTION,
        /// Set the break note.
        with_note        => FIELD_NOTE,
        /// Set the break start time.
        with_start_time  => FIELD_START_TIME,
        /// Set the break end time.
        with_end_time    => FIELD_END_TIME,
        /// Set the break duration.
        with_duration    => FIELD_DURATION,
        /// Set the panel types associated with this break.
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

    fn make_break_id() -> BreakId {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        unsafe { BreakId::new_unchecked(non_nil_uuid) }
    }

    fn make_test_internal_data() -> BreakInternalData {
        BreakInternalData {
            id: make_break_id(),
            data: BreakCommonData {
                name: "Lunch Break".into(),
                description: Some("Lunch on your own".into()),
                note: Some("Vendor hall stays open".into()),
            },
            code: PanelUniqId {
                prefix: "BREAK".into(),
                prefix_num: 1,
                part_num: None,
                session_num: None,
                suffix: None,
            },
            time_slot: TimeRange::ScheduledWithDuration {
                start_time: parse_datetime("2026-06-26T12:00:00").unwrap(),
                duration: Duration::minutes(60),
            },
        }
    }

    fn make_schedule_with_break(id: BreakId, data: BreakInternalData) -> Schedule {
        let mut sched = Schedule::default();
        sched.insert(id, data);
        sched
    }

    #[test]
    fn test_field_set_half_edges() {
        let fs = BreakEntityType::field_set();
        let half_edges: Vec<_> = fs.half_edges().collect();
        assert_eq!(half_edges.len(), 1);
        assert_eq!(half_edges[0].data.name, "panel_types");
    }

    #[test]
    fn test_read_field_code() {
        let id = make_break_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_break(id, data);

        let fs = BreakEntityType::field_set();
        let value = fs.read_field_value("code", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!("BREAK001")));
    }

    #[test]
    fn test_read_field_duration() {
        let id = make_break_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_break(id, data);

        let fs = BreakEntityType::field_set();
        let value = fs.read_field_value("duration", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!(Duration::minutes(60))));
    }

    #[test]
    fn test_write_field_code() {
        let id = make_break_id();
        let data = make_test_internal_data();
        let mut sched = make_schedule_with_break(id, data);

        let fs = BreakEntityType::field_set();
        fs.write_field_value("code", id, &mut sched, field_value!("BREAK002"))
            .unwrap();

        let value = fs.read_field_value("code", id, &sched).unwrap();
        assert_eq!(value, Some(field_value!("BREAK002")));
    }

    #[test]
    fn test_common_data_serde_roundtrip() {
        let original = BreakCommonData {
            name: "Lunch Break".into(),
            description: Some("Lunch on your own".into()),
            note: Some("Vendor hall stays open".into()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: BreakCommonData = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_export_preserves_duration() {
        let id = make_break_id();
        let data = make_test_internal_data();
        let sched = make_schedule_with_break(id, data);

        let internal = sched.get_internal::<BreakEntityType>(id).unwrap();
        let exported = BreakEntityType::export(internal);
        assert_eq!(exported.code, "BREAK001");
        assert_eq!(exported.data.name, "Lunch Break");
        assert_eq!(exported.duration, Some(Duration::minutes(60)));
        assert_eq!(
            exported.start_time,
            Some(parse_datetime("2026-06-26T12:00:00").unwrap())
        );
        assert_eq!(
            exported.end_time,
            Some(parse_datetime("2026-06-26T13:00:00").unwrap())
        );
    }

    #[test]
    fn test_builder_round_trip_with_duration() {
        let mut sched = Schedule::default();
        let id = BreakBuilder::new()
            .with_code("BREAK001")
            .with_name("Lunch Break")
            .with_start_time(Some(parse_datetime("2026-06-26T12:00:00").unwrap()))
            .with_duration(Some(Duration::minutes(60)))
            .build(&mut sched)
            .unwrap();

        let internal = sched.get_internal::<BreakEntityType>(id).unwrap();
        assert_eq!(internal.code.full_id(), "BREAK001");
        assert_eq!(internal.time_slot.duration(), Some(Duration::minutes(60)));
    }
}
