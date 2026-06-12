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

use crate::entity::{EntityId, EntityType, EntityUuid, FieldSet};
use crate::field::{CollectedField, FieldDescriptor, NamedField};
use crate::tables::fields;
use crate::tables::fields::code::{CodeHistory, HasCode};
use crate::tables::fields::description::HasDescription;
use crate::tables::fields::name::HasName;
use crate::tables::fields::note::{HasNotes, NoteBag, NoteKind, PublicNote};
use crate::tables::fields::time::HasStartTime;
use crate::tables::panel_like::{EventKind, PanelLike};
use crate::value::ValidationError;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── Type Aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for Timeline entities.
pub type TimelineId = EntityId<TimelineEntityType>;

/// User-facing fields for timeline events. The shared `name` / `description` /
/// `note` are exposed uniformly through [`PanelLike`]. A timeline is a single
/// instant, so it stores `time` directly (an `Option<NaiveDateTime>`) and
/// implements only [`HasStartTime`] — not the duration capability — so the
/// shared `time` field logic applies without synthesising a range.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineCommonData {
    pub name: String,
    pub description: Option<String>,
    pub time: Option<chrono::NaiveDateTime>,
}

// ── TimelineInternalData ───────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
#[derive(Debug, Clone)]
pub struct TimelineInternalData {
    pub id: TimelineId,
    pub data: TimelineCommonData,
    /// Notes keyed by [`NoteKind`]; a timeline supports only
    /// [`NoteKind::Public`]. See [`HasNotes::SUPPORTED_NOTES`].
    pub notes: NoteBag,
    /// Current Uniq ID (e.g. `TL01`) plus the history of previously-held codes;
    /// see [`HasCode`](crate::tables::fields::code::HasCode).
    pub code: CodeHistory,
}

// ── TimelineData ─────────────────────────────────────────────────────────────

/// Export/API view produced by [`TimelineEntityType::export`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineData {
    /// Canonical Uniq ID string (e.g. `"TL01"`), from `code.full_id()`.
    pub code: String,
    /// Previously-held Uniq ID strings (history).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub old_codes: Vec<String>,
    #[serde(flatten)]
    pub data: TimelineCommonData,
    /// Public note ([`NoteKind::Public`]), projected from the note bag.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub note: Option<String>,
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
            old_codes: internal.code.old_codes().to_vec(),
            data: internal.data.clone(),
            note: internal.notes.get_owned(NoteKind::Public),
        }
    }

    fn validate(_internal: &Self::InternalData) -> Vec<ValidationError> {
        Vec::new()
    }
}

// ── PanelLike ───────────────────────────────────────────────────────────────────

impl HasName for TimelineEntityType {
    fn name(d: &Self::InternalData) -> &String {
        &d.data.name
    }
    fn name_mut(d: &mut Self::InternalData) -> &mut String {
        &mut d.data.name
    }
}

impl HasDescription for TimelineEntityType {
    fn description(d: &Self::InternalData) -> &Option<String> {
        &d.data.description
    }
    fn description_mut(d: &mut Self::InternalData) -> &mut Option<String> {
        &mut d.data.description
    }
}

impl HasNotes for TimelineEntityType {
    const SUPPORTED_NOTES: &'static [NoteKind] = &[NoteKind::Public];
    fn notes(d: &Self::InternalData) -> &NoteBag {
        &d.notes
    }
    fn notes_mut(d: &mut Self::InternalData) -> &mut NoteBag {
        &mut d.notes
    }
}

impl PanelLike for TimelineEntityType {
    const KIND: EventKind = EventKind::Timeline;
}

impl HasCode for TimelineEntityType {
    fn code(d: &Self::InternalData) -> &CodeHistory {
        &d.code
    }
    fn code_mut(d: &mut Self::InternalData) -> &mut CodeHistory {
        &mut d.code
    }
}

impl HasStartTime for TimelineEntityType {
    /// A timeline stores its instant directly — no [`TimeRange`] projection.
    ///
    /// [`TimeRange`]: crate::value::time::TimeRange
    fn start_time(d: &Self::InternalData) -> Option<chrono::NaiveDateTime> {
        d.data.time
    }
    fn set_start_time(d: &mut Self::InternalData, start: Option<chrono::NaiveDateTime>) {
        d.data.time = start;
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

// ── Lookup helpers ───────────────────────────────────────────────────────────────

impl TimelineEntityType {
    /// Find all live timeline entries with the given Uniq ID code (case-insensitive).
    ///
    /// Returns all matches; in well-formed data the list has at most one entry,
    /// but duplicate codes are possible in human-authored XLSX files.
    pub fn find_by_code(schedule: &crate::schedule::Schedule, code: &str) -> Vec<TimelineId> {
        let upper = code.to_uppercase();
        schedule
            .iter_entities::<Self>()
            .filter_map(|(id, d)| (d.code.full_id().to_uppercase() == upper).then_some(id))
            .collect()
    }
}

// ── EntityBuildable ─────────────────────────────────────────────────────────────

impl crate::edit::builder::EntityBuildable for TimelineEntityType {
    fn default_data(id: EntityId<Self>) -> Self::InternalData {
        TimelineInternalData {
            id,
            data: TimelineCommonData::default(),
            notes: NoteBag::default(),
            code: CodeHistory::default(),
        }
    }

    fn find_by_natural_key(schedule: &crate::schedule::Schedule, key: &str) -> Vec<EntityId<Self>> {
        Self::find_by_code(schedule, key)
    }
}

// ── Stored field descriptors ──────────────────────────────────────────────────

/// Timeline field descriptors — all defined once in [`crate::tables::fields`]
/// and instantiated here with timeline-specific `order` / `aliases`. `time` maps
/// to the start of the `time_slot` backing field (a timeline carries no
/// duration).
pub static FIELD_CODE: FieldDescriptor<TimelineEntityType> = fields::code::code_field(0);
inventory::submit! { CollectedField(&FIELD_CODE) }

pub static FIELD_NAME: FieldDescriptor<TimelineEntityType> =
    fields::name::name_field(100, &["title", "timeline_name"]);
inventory::submit! { CollectedField(&FIELD_NAME) }

pub static FIELD_DESCRIPTION: FieldDescriptor<TimelineEntityType> =
    fields::description::description_field(200, &["desc"]);
inventory::submit! { CollectedField(&FIELD_DESCRIPTION) }

pub static FIELD_NOTE: FieldDescriptor<TimelineEntityType> =
    fields::note::note_field::<TimelineEntityType, PublicNote>(300);
inventory::submit! { CollectedField(&FIELD_NOTE) }

pub static FIELD_TIME: FieldDescriptor<TimelineEntityType> =
    fields::time::time_field(400, &["start_time", "start"]);
inventory::submit! { CollectedField(&FIELD_TIME) }

/// `old_codes` — history of previously-held Uniq IDs (FEATURE-146).
pub static FIELD_OLD_CODES: FieldDescriptor<TimelineEntityType> = fields::code::old_codes_field(50);
inventory::submit! { CollectedField(&FIELD_OLD_CODES) }

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
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_value;
    use crate::schedule::Schedule;
    use crate::value::uniq_id::PanelUniqId;
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
                time: None,
            },
            notes: {
                let mut notes = NoteBag::default();
                notes.set(NoteKind::Public, Some("Main ballroom".into()));
                notes
            },
            code: CodeHistory::new(PanelUniqId {
                prefix: "TL".into(),
                prefix_num: 1,
                part_num: None,
                session_num: None,
                suffix: None,
            }),
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

        // Timeline has no edges: its panel type is derived from the code prefix.
        let names: Vec<_> = fs.half_edges().map(|he| he.data.name).collect();
        assert!(names.is_empty());
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
            time: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: TimelineCommonData = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_validate_valid_data() {
        // Timelines have no constraints; validation always passes.
        let data = make_test_internal_data();
        let errors = TimelineEntityType::validate(&data);
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
        assert_eq!(exported.note, Some("Main ballroom".into()));
        assert_eq!(exported.data.time, None);
    }
}
