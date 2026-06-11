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

use crate::entity::{EntityId, EntityType, EntityUuid, FieldSet};
use crate::field::{CollectedField, CollectedHalfEdge, FieldDescriptor, NamedField};
use crate::tables::fields;
use crate::tables::fields::description::HasDescription;
use crate::tables::fields::name::HasName;
use crate::tables::fields::duration::HasDuration;
use crate::tables::fields::note::{HasNotes, NoteBag, NoteKind, PublicNote};
use crate::tables::fields::time::HasStartTime;
use crate::tables::panel_like::{EventKind, PanelLike};
use crate::tables::panel_type::{self, PanelTypeEntityType};
use crate::value::time::TimeRange;
use crate::value::uniq_id::PanelUniqId;
use crate::value::{FieldCardinality, FieldType, FieldTypeItem, ValidationError};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── Type Aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for Break entities.
pub type BreakId = EntityId<BreakEntityType>;

/// User-facing fields for break periods. The shared `name` / `description` /
/// `note` are exposed uniformly through [`PanelLike`], but each panel-like
/// entity owns its own common-data struct.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakCommonData {
    pub name: String,
    pub description: Option<String>,
}

// ── BreakInternalData ──────────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
#[derive(Debug, Clone)]
pub struct BreakInternalData {
    pub id: BreakId,
    pub data: BreakCommonData,
    /// Notes keyed by [`NoteKind`]; a break supports only
    /// [`NoteKind::Public`]. See [`HasNotes::SUPPORTED_NOTES`].
    pub notes: NoteBag,
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
    /// Public note ([`NoteKind::Public`]), projected from the note bag.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub note: Option<String>,
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
            note: internal.notes.get_owned(NoteKind::Public),
            start_time: internal.time_slot.start_time(),
            end_time: internal.time_slot.end_time(),
            duration: internal.time_slot.duration(),
        }
    }

    fn validate(internal: &Self::InternalData) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if let Err(msg) = internal.time_slot.validate() {
            errors.push(ValidationError::Constraint {
                field: "time_slot",
                message: msg,
            });
        }
        errors
    }
}

// ── PanelLike ───────────────────────────────────────────────────────────────────

impl HasName for BreakEntityType {
    fn name(d: &Self::InternalData) -> &String {
        &d.data.name
    }
    fn name_mut(d: &mut Self::InternalData) -> &mut String {
        &mut d.data.name
    }
}

impl HasDescription for BreakEntityType {
    fn description(d: &Self::InternalData) -> &Option<String> {
        &d.data.description
    }
    fn description_mut(d: &mut Self::InternalData) -> &mut Option<String> {
        &mut d.data.description
    }
}

impl HasNotes for BreakEntityType {
    const SUPPORTED_NOTES: &'static [NoteKind] = &[NoteKind::Public];
    fn notes(d: &Self::InternalData) -> &NoteBag {
        &d.notes
    }
    fn notes_mut(d: &mut Self::InternalData) -> &mut NoteBag {
        &mut d.notes
    }
}

impl PanelLike for BreakEntityType {
    const KIND: EventKind = EventKind::Break;
    fn code(d: &Self::InternalData) -> &PanelUniqId {
        &d.code
    }
    fn code_mut(d: &mut Self::InternalData) -> &mut PanelUniqId {
        &mut d.code
    }
}

impl HasStartTime for BreakEntityType {
    fn start_time(d: &Self::InternalData) -> Option<chrono::NaiveDateTime> {
        d.time_slot.start_time()
    }
    fn set_start_time(d: &mut Self::InternalData, start: Option<chrono::NaiveDateTime>) {
        match start {
            Some(t) => d.time_slot.add_start_time(t),
            None => d.time_slot.remove_start_time(),
        }
    }
}

impl HasDuration for BreakEntityType {
    fn time_range(d: &Self::InternalData) -> TimeRange {
        d.time_slot.clone()
    }
    fn set_time_range(d: &mut Self::InternalData, time_range: TimeRange) {
        d.time_slot = time_range;
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
            notes: NoteBag::default(),
            code: PanelUniqId::default(),
            time_slot: TimeRange::default(),
        }
    }

    fn find_by_natural_key(schedule: &crate::schedule::Schedule, key: &str) -> Vec<EntityId<Self>> {
        Self::find_by_code(schedule, key)
    }
}

// ── Stored field descriptors ──────────────────────────────────────────────────

/// Break field descriptors — defined once in [`crate::tables::fields`] and
/// instantiated here with break-specific `order` / `aliases`. See those modules
/// for the read/write logic shared across all panel-like entities.
pub static FIELD_CODE: FieldDescriptor<BreakEntityType> = fields::code::code_field(0);
inventory::submit! { CollectedField(&FIELD_CODE) }

pub static FIELD_NAME: FieldDescriptor<BreakEntityType> =
    fields::name::name_field(100, &["title", "break_name"]);
inventory::submit! { CollectedField(&FIELD_NAME) }

pub static FIELD_DESCRIPTION: FieldDescriptor<BreakEntityType> =
    fields::description::description_field(200, &["desc"]);
inventory::submit! { CollectedField(&FIELD_DESCRIPTION) }

pub static FIELD_NOTE: FieldDescriptor<BreakEntityType> =
    fields::note::note_field::<BreakEntityType, PublicNote>(300);
inventory::submit! { CollectedField(&FIELD_NOTE) }

pub static FIELD_START_TIME: FieldDescriptor<BreakEntityType> = fields::time::start_time_field(400);
inventory::submit! { CollectedField(&FIELD_START_TIME) }

pub static FIELD_END_TIME: FieldDescriptor<BreakEntityType> =
    fields::duration::end_time_field(500);
inventory::submit! { CollectedField(&FIELD_END_TIME) }

pub static FIELD_DURATION: FieldDescriptor<BreakEntityType> =
    fields::duration::duration_field(600);
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
    use crate::value::time::parse_datetime;
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
            },
            notes: {
                let mut notes = NoteBag::default();
                notes.set(NoteKind::Public, Some("Vendor hall stays open".into()));
                notes
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
