/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel entity — one event on the schedule.
//!
//! Three structs define the Panel entity:
//!
//! - [`PanelCommonData`] — serializable, user-facing fields from the Schedule sheet.
//! - [`PanelInternalData`] — `EntityType::InternalData`; the field system operates on this.
//! - [`PanelData`] — export/API view produced by [`PanelEntityType::export`].
//!
//! Time information is held in a [`TimeRange`] backing field (`time_slot`)
//! and exposed through computed `start_time`, `end_time`, and `duration`
//! fields. Relationship data (presenters, event rooms, panel type) is modeled
//! via edge-backed computed fields; the fields are declared here as stubs and
//! fully wired up in FEATURE-018.

use crate::entity::{EntityId, EntityType, FieldSet};
use crate::event_room::EventRoomId;
use crate::field::{FieldDescriptor, ReadFn, VerifyFn, WriteFn};
use crate::field_macros::{
    bool_field, edge_list_field, edge_list_field_rw, edge_mutator_field, edge_none_field_rw,
    opt_i64_field, opt_string_field, opt_text_field, req_string_field,
};
use crate::panel_type::PanelTypeId;
use crate::panel_uniq_id::PanelUniqId;
use crate::presenter::PresenterId;
use crate::time::{parse_datetime, parse_duration, TimeRange};
use crate::value::{CrdtFieldType, ValidationError};
use crate::{field_datetime, field_duration, field_string};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── Type aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for Panel entities.
pub type PanelId = EntityId<PanelEntityType>;

// ── PanelCommonData ───────────────────────────────────────────────────────────

/// User-facing fields from the Schedule sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelCommonData {
    pub name: String,
    pub description: Option<String>,
    pub note: Option<String>,
    pub notes_non_printing: Option<String>,
    pub workshop_notes: Option<String>,
    pub power_needs: Option<String>,
    pub sewing_machines: bool,
    pub av_notes: Option<String>,
    pub difficulty: Option<String>,
    pub prereq: Option<String>,
    pub cost: Option<String>,
    pub is_free: bool,
    pub is_kids: bool,
    pub is_full: bool,
    pub capacity: Option<i64>,
    pub seats_sold: Option<i64>,
    pub pre_reg_max: Option<i64>,
    pub ticket_url: Option<String>,
    pub have_ticket_image: bool,
    pub simpletix_event: Option<String>,
    pub simpletix_link: Option<String>,
    pub hide_panelist: bool,
    pub alt_panelist: Option<String>,
}

impl PanelCommonData {
    fn validate(&self) -> Vec<ValidationError> {
        Vec::new()
    }
}

// ── PanelInternalData ─────────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
#[derive(Debug, Clone)]
pub struct PanelInternalData {
    pub id: PanelId,
    pub data: PanelCommonData,
    /// Parsed Uniq ID (e.g. `GP032`). Structurally valid by construction;
    /// callers parse via [`PanelUniqId::parse`] before building this struct.
    pub code: PanelUniqId,
    pub time_slot: TimeRange,
}

// ── PanelData ─────────────────────────────────────────────────────────────────

/// Export/API view produced by [`PanelEntityType::export`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelData {
    /// Canonical Uniq ID string (e.g. `"GP032"`), from `code.full_id()`.
    pub code: String,
    #[serde(flatten)]
    pub data: PanelCommonData,
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
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub presenter_ids: Vec<PresenterId>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub event_room_ids: Vec<EventRoomId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub panel_type_id: Option<PanelTypeId>,
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

// ── PanelEntityType ───────────────────────────────────────────────────────────

/// Singleton type representing the Panel entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelEntityType;

impl EntityType for PanelEntityType {
    type InternalData = PanelInternalData;
    type Data = PanelData;

    const TYPE_NAME: &'static str = "panel";

    fn uuid_namespace() -> &'static uuid::Uuid {
        static NS: LazyLock<uuid::Uuid> =
            LazyLock::new(|| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, b"panel"));
        &NS
    }

    fn field_set() -> &'static FieldSet<Self> {
        &PANEL_FIELD_SET
    }

    fn export(internal: &Self::InternalData) -> Self::Data {
        PanelData {
            code: internal.code.full_id(),
            data: internal.data.clone(),
            start_time: internal.time_slot.start_time(),
            end_time: internal.time_slot.end_time(),
            duration: internal.time_slot.duration(),
            presenter_ids: Vec::new(),
            event_room_ids: Vec::new(),
            panel_type_id: None,
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

// ── Stored field descriptors ──────────────────────────────────────────────────

/// Panel `code` (Uniq ID) — stored as the parsed [`PanelUniqId`] on
/// [`PanelInternalData`], exposed to the field system as a string.
///
/// Hand-written because the storage type is not a plain `String` and because
/// a future edge-recomputation pass will need to react to code changes:
/// panel ↔ panel-type linkage is keyed off the two-letter prefix, and changing
/// a panel's code may reassign it to a different `PanelType`. The write path
/// here performs the parse and mutation only; edge refresh is deferred to
/// FEATURE-018.
static FIELD_CODE: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "code",
    display: "Uniq ID",
    description: "Panel Uniq ID (e.g. \"GP032\"), parsed from the Schedule sheet.",
    aliases: &["uid", "uniq_id", "id"],
    required: true,
    crdt_type: CrdtFieldType::Scalar,
    example: "GP032",
    read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
        Some(field_string!(d.code.full_id()))
    })),
    write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
        let s = v.into_string()?;
        // TODO(FEATURE-018): refresh edges keyed by prefix/code when
        // edge storage lands (panel ↔ panel_type linkage depends on this).
        match PanelUniqId::parse(&s) {
            Some(parsed) => {
                d.code = parsed;
                Ok(())
            }
            None => Err(crate::value::ConversionError::ParseError {
                message: format!("could not parse panel Uniq ID {s:?}"),
            }
            .into()),
        }
    })),
    index_fn: Some(|query, d: &PanelInternalData| {
        let q = query.to_lowercase();
        let full = d.code.full_id().to_lowercase();
        if full == q {
            return Some(crate::field::MatchPriority::Exact);
        }
        let base = d.code.base_id().to_lowercase();
        if base == q {
            return Some(crate::field::MatchPriority::Exact);
        }
        if full.starts_with(&q) || base.starts_with(&q) {
            return Some(crate::field::MatchPriority::Prefix);
        }
        if d.code.prefix.to_lowercase() == q {
            return Some(crate::field::MatchPriority::Prefix);
        }
        if full.contains(&q) {
            return Some(crate::field::MatchPriority::Contains);
        }
        None
    }),
    verify_fn: None,
};

// @todo: Name can be empty, should be optional
req_string_field!(FIELD_NAME, PanelEntityType, PanelInternalData, name,
    name: "name", display: "Name",
    desc: "Panel name / title.",
    aliases: &["title", "panel_name"],
    example: "Cosplay Foam Armor 101");

opt_text_field!(FIELD_DESCRIPTION, PanelEntityType, PanelInternalData, description,
    name: "description", display: "Description",
    desc: "Event description shown to attendees.",
    aliases: &["desc"],
    example: "Learn the basics of foam armor construction");

opt_text_field!(FIELD_NOTE, PanelEntityType, PanelInternalData, note,
    name: "note", display: "Note",
    desc: "Extra note displayed verbatim.",
    aliases: &[],
    example: "Bring your own materials");

opt_text_field!(FIELD_NOTES_NON_PRINTING, PanelEntityType, PanelInternalData, notes_non_printing,
    name: "notes_non_printing", display: "Notes (Non Printing)",
    desc: "Internal notes not shown to the public.",
    aliases: &["internal_notes"],
    example: "Internal note for staff");

opt_text_field!(FIELD_WORKSHOP_NOTES, PanelEntityType, PanelInternalData, workshop_notes,
    name: "workshop_notes", display: "Workshop Notes",
    desc: "Notes for workshop staff.",
    aliases: &[],
    example: "Staff notes for workshop");

opt_string_field!(FIELD_POWER_NEEDS, PanelEntityType, PanelInternalData, power_needs,
    name: "power_needs", display: "Power Needs",
    desc: "Power / electrical requirements.",
    aliases: &["power"],
    example: "2 outlets");

bool_field!(FIELD_SEWING_MACHINES, PanelEntityType, PanelInternalData, sewing_machines,
    name: "sewing_machines", display: "Sewing Machines",
    desc: "Whether sewing machines are required.",
    aliases: &["sewing"],
    example: "false");

opt_text_field!(FIELD_AV_NOTES, PanelEntityType, PanelInternalData, av_notes,
    name: "av_notes", display: "AV Notes",
    desc: "Audio/visual setup notes.",
    aliases: &["av"],
    example: "Projector needed");

opt_string_field!(FIELD_DIFFICULTY, PanelEntityType, PanelInternalData, difficulty,
    name: "difficulty", display: "Difficulty",
    desc: "Skill-level indicator (free text).",
    aliases: &[],
    example: "Beginner");

opt_string_field!(FIELD_PREREQ, PanelEntityType, PanelInternalData, prereq,
    name: "prereq", display: "Prerequisites",
    desc: "Comma-separated prerequisite Uniq IDs.",
    aliases: &["prerequisites"],
    example: "GP001");

opt_string_field!(FIELD_COST, PanelEntityType, PanelInternalData, cost,
    name: "cost", display: "Cost",
    desc: "Raw cost cell value (e.g. \"$35\", \"Free\", \"Kids\").",
    aliases: &[],
    example: "$35");

bool_field!(FIELD_IS_FREE, PanelEntityType, PanelInternalData, is_free,
    name: "is_free", display: "Is Free",
    desc: "Parsed during import: cost is blank, \"Free\", \"$0\", or \"N/A\".",
    aliases: &["free"],
    example: "false");

bool_field!(FIELD_IS_KIDS, PanelEntityType, PanelInternalData, is_kids,
    name: "is_kids", display: "Is Kids",
    desc: "Parsed during import: cost indicates kids-only pricing.",
    aliases: &["kids"],
    example: "false");

bool_field!(FIELD_IS_FULL, PanelEntityType, PanelInternalData, is_full,
    name: "is_full", display: "Full",
    desc: "Event is at capacity.",
    aliases: &["full"],
    example: "false");

opt_i64_field!(FIELD_CAPACITY, PanelEntityType, PanelInternalData, capacity,
    name: "capacity", display: "Capacity",
    desc: "Total seats available.",
    aliases: &[],
    example: "50");

opt_i64_field!(FIELD_SEATS_SOLD, PanelEntityType, PanelInternalData, seats_sold,
    name: "seats_sold", display: "Seats Sold",
    desc: "Number of seats pre-sold or reserved via ticketing.",
    aliases: &[],
    example: "25");

opt_i64_field!(FIELD_PRE_REG_MAX, PanelEntityType, PanelInternalData, pre_reg_max,
    name: "pre_reg_max", display: "Pre-reg Max",
    desc: "Maximum seats available for pre-registration.",
    aliases: &["prereg_max"],
    example: "40");

opt_string_field!(FIELD_TICKET_URL, PanelEntityType, PanelInternalData, ticket_url,
    name: "ticket_url", display: "Ticket URL",
    desc: "URL for purchasing tickets.",
    aliases: &["ticket_sale"],
    example: "https://example.com/ticket");

bool_field!(FIELD_HAVE_TICKET_IMAGE, PanelEntityType, PanelInternalData, have_ticket_image,
    name: "have_ticket_image", display: "Have Ticket Image",
    desc: "Whether a ticket / flyer image has been received.",
    aliases: &[],
    example: "false");

opt_string_field!(FIELD_SIMPLETIX_EVENT, PanelEntityType, PanelInternalData, simpletix_event,
    name: "simpletix_event", display: "SimpleTix Event",
    desc: "Internal admin URL for SimpleTix event configuration.",
    aliases: &["simpletix"],
    example: "https://admin.simpletix.com/event/123");

opt_string_field!(FIELD_SIMPLETIX_LINK, PanelEntityType, PanelInternalData, simpletix_link,
    name: "simpletix_link", display: "SimpleTix Link",
    desc: "Public-facing direct ticket purchase link.",
    aliases: &[],
    example: "https://simpletix.com/event/123");

bool_field!(FIELD_HIDE_PANELIST, PanelEntityType, PanelInternalData, hide_panelist,
    name: "hide_panelist", display: "Hide Panelist",
    desc: "Suppress presenter credits for this panel.",
    aliases: &[],
    example: "false");

opt_string_field!(FIELD_ALT_PANELIST, PanelEntityType, PanelInternalData, alt_panelist,
    name: "alt_panelist", display: "Alt Panelist",
    desc: "Override text for the presenter credits line.",
    aliases: &[],
    example: "Special Guest");

// ── Computed time projections ─────────────────────────────────────────────────

/// Start time — projected from `time_slot`.
static FIELD_START_TIME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "start_time",
    display: "Start Time",
    description: "Panel start time.",
    aliases: &["start"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    example: "2023-06-25T19:00:00",
    read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
        d.time_slot.start_time().map(|dt| field_datetime!(dt))
    })),
    write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
        match v {
            crate::value::FieldValue::List(_)
            | crate::value::FieldValue::Single(crate::value::FieldValueItem::Text(_)) => {
                d.time_slot.remove_start_time()
            }
            crate::value::FieldValue::Single(crate::value::FieldValueItem::DateTime(dt)) => {
                d.time_slot.add_start_time(dt)
            }
            crate::value::FieldValue::Single(crate::value::FieldValueItem::String(s)) => {
                match parse_datetime(&s) {
                    Some(dt) => d.time_slot.add_start_time(dt),
                    None => {
                        return Err(crate::value::ConversionError::ParseError {
                            message: format!("could not parse datetime {s:?}"),
                        }
                        .into())
                    }
                }
            }
            _ => {
                return Err(crate::value::ConversionError::WrongVariant {
                    expected: "DateTime or String",
                    got: "other",
                }
                .into());
            }
        }
        Ok(())
    })),
    index_fn: None,
    verify_fn: Some(VerifyFn::ReRead),
};

/// End time — projected from `time_slot`.
static FIELD_END_TIME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "end_time",
    display: "End Time",
    description: "Panel end time.",
    aliases: &["end"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    example: "2023-06-25T20:30:00",
    read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
        d.time_slot.end_time().map(|dt| field_datetime!(dt))
    })),
    write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
        match v {
            crate::value::FieldValue::List(_)
            | crate::value::FieldValue::Single(crate::value::FieldValueItem::Text(_)) => {
                d.time_slot.remove_end_time()
            }
            crate::value::FieldValue::Single(crate::value::FieldValueItem::DateTime(dt)) => {
                d.time_slot.add_end_time(dt)
            }
            crate::value::FieldValue::Single(crate::value::FieldValueItem::String(s)) => {
                match parse_datetime(&s) {
                    Some(dt) => d.time_slot.add_end_time(dt),
                    None => {
                        return Err(crate::value::ConversionError::ParseError {
                            message: format!("could not parse datetime {s:?}"),
                        }
                        .into())
                    }
                }
            }
            _ => {
                return Err(crate::value::ConversionError::WrongVariant {
                    expected: "DateTime or String",
                    got: "other",
                }
                .into());
            }
        }
        Ok(())
    })),
    index_fn: None,
    verify_fn: Some(VerifyFn::ReRead),
};

/// Duration — projected from `time_slot`.
static FIELD_DURATION: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "duration",
    display: "Duration",
    description: "Panel duration.",
    aliases: &[],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    example: "90",
    read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
        d.time_slot.duration().map(|dur| field_duration!(dur))
    })),
    write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
        match v {
            crate::value::FieldValue::List(_)
            | crate::value::FieldValue::Single(crate::value::FieldValueItem::Text(_)) => {
                d.time_slot.remove_duration()
            }
            crate::value::FieldValue::Single(crate::value::FieldValueItem::Duration(dur)) => {
                d.time_slot.add_duration(dur)
            }
            crate::value::FieldValue::Single(crate::value::FieldValueItem::Integer(m)) => {
                d.time_slot.add_duration(Duration::minutes(m))
            }
            crate::value::FieldValue::Single(crate::value::FieldValueItem::String(s)) => {
                match parse_duration(&s) {
                    Some(dur) => d.time_slot.add_duration(dur),
                    None => {
                        return Err(crate::value::ConversionError::ParseError {
                            message: format!("could not parse duration {s:?}"),
                        }
                        .into())
                    }
                }
            }
            _ => {
                return Err(crate::value::ConversionError::WrongVariant {
                    expected: "Duration, Integer, or String",
                    got: "other",
                }
                .into());
            }
        }
        Ok(())
    })),
    index_fn: None,
    verify_fn: Some(VerifyFn::ReRead),
};

// ── Edge-backed computed field stubs (full wiring in FEATURE-018) ─────────────
//
// Reads return empty lists (or `None` for the singular `panel_type` field)
// and writes are no-ops until edge storage lands. All stubs are declared via
// the shared edge-stub macros in `crate::field_macros`.

edge_list_field_rw!(FIELD_PRESENTERS, PanelEntityType, PanelInternalData,
    name: "presenters", display: "Presenters",
    desc: "All presenters credited for this panel.",
    aliases: &["panelists", "presenter"],
    example: "[]");

edge_mutator_field!(FIELD_ADD_PRESENTERS, PanelEntityType, PanelInternalData,
    name: "add_presenters", display: "Add Presenters",
    desc: "Append presenters to this panel.",
    aliases: &["add_presenter"],
    example: "[presenter_id]");

edge_mutator_field!(FIELD_REMOVE_PRESENTERS, PanelEntityType, PanelInternalData,
    name: "remove_presenters", display: "Remove Presenters",
    desc: "Remove presenters from this panel.",
    aliases: &["remove_presenter"],
    example: "[presenter_id]");

edge_list_field!(FIELD_INCLUSIVE_PRESENTERS, PanelEntityType, PanelInternalData,
    name: "inclusive_presenters", display: "Inclusive Presenters",
    desc: "Transitive closure: direct presenters + their groups + group members.",
    aliases: &["inclusive_presenter"],
    example: "[]");

edge_list_field_rw!(FIELD_EVENT_ROOMS, PanelEntityType, PanelInternalData,
    name: "event_rooms", display: "Event Rooms",
    desc: "Rooms where this panel takes place.",
    aliases: &["rooms", "room", "event_room"],
    example: "[]");

edge_mutator_field!(FIELD_ADD_ROOMS, PanelEntityType, PanelInternalData,
    name: "add_rooms", display: "Add Rooms",
    desc: "Append event rooms to this panel.",
    aliases: &["add_room"],
    example: "[room_id]");

edge_mutator_field!(FIELD_REMOVE_ROOMS, PanelEntityType, PanelInternalData,
    name: "remove_rooms", display: "Remove Rooms",
    desc: "Remove event rooms from this panel.",
    aliases: &["remove_room"],
    example: "[room_id]");

edge_none_field_rw!(FIELD_PANEL_TYPE, PanelEntityType, PanelInternalData,
    name: "panel_type", display: "Panel Type",
    desc: "Panel type / kind.",
    aliases: &["kind", "type"],
    example: "null");

// ── FieldSet ──────────────────────────────────────────────────────────────────

static PANEL_FIELD_SET: LazyLock<FieldSet<PanelEntityType>> = LazyLock::new(|| {
    FieldSet::new(&[
        // stored
        &FIELD_CODE,
        &FIELD_NAME,
        &FIELD_DESCRIPTION,
        &FIELD_NOTE,
        &FIELD_NOTES_NON_PRINTING,
        &FIELD_WORKSHOP_NOTES,
        &FIELD_POWER_NEEDS,
        &FIELD_SEWING_MACHINES,
        &FIELD_AV_NOTES,
        &FIELD_DIFFICULTY,
        &FIELD_PREREQ,
        &FIELD_COST,
        &FIELD_IS_FREE,
        &FIELD_IS_KIDS,
        &FIELD_IS_FULL,
        &FIELD_CAPACITY,
        &FIELD_SEATS_SOLD,
        &FIELD_PRE_REG_MAX,
        &FIELD_TICKET_URL,
        &FIELD_HAVE_TICKET_IMAGE,
        &FIELD_SIMPLETIX_EVENT,
        &FIELD_SIMPLETIX_LINK,
        &FIELD_HIDE_PANELIST,
        &FIELD_ALT_PANELIST,
        // time projections
        &FIELD_START_TIME,
        &FIELD_END_TIME,
        &FIELD_DURATION,
        // edge stubs
        &FIELD_PRESENTERS,
        &FIELD_ADD_PRESENTERS,
        &FIELD_REMOVE_PRESENTERS,
        &FIELD_INCLUSIVE_PRESENTERS,
        &FIELD_EVENT_ROOMS,
        &FIELD_ADD_ROOMS,
        &FIELD_REMOVE_ROOMS,
        &FIELD_PANEL_TYPE,
    ])
});

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;
    use crate::value::FieldError;
    use crate::{
        field_boolean, field_datetime, field_duration, field_integer, field_string, field_text,
        field_value,
    };
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn new_panel_id() -> PanelId {
        PanelId::new(Uuid::new_v4()).expect("v4 is never nil")
    }

    fn sample_common() -> PanelCommonData {
        PanelCommonData {
            name: "Panel Name".into(),
            description: Some("A description".into()),
            note: None,
            notes_non_printing: None,
            workshop_notes: None,
            power_needs: Some("two outlets".into()),
            sewing_machines: false,
            av_notes: Some("mic".into()),
            difficulty: None,
            prereq: None,
            cost: Some("$35".into()),
            is_free: false,
            is_kids: false,
            is_full: false,
            capacity: Some(50),
            seats_sold: Some(12),
            pre_reg_max: Some(40),
            ticket_url: None,
            have_ticket_image: false,
            simpletix_event: None,
            simpletix_link: None,
            hide_panelist: false,
            alt_panelist: None,
        }
    }

    fn sample_internal(id: PanelId) -> PanelInternalData {
        PanelInternalData {
            id,
            data: sample_common(),
            time_slot: TimeRange::ScheduledWithDuration {
                start_time: NaiveDate::from_ymd_opt(2026, 6, 26)
                    .unwrap()
                    .and_hms_opt(14, 0, 0)
                    .unwrap(),
                duration: Duration::minutes(60),
            },
            code: PanelUniqId::parse("GP001").expect("GP001 is a valid Uniq ID"),
        }
    }

    fn sched_with(id: PanelId, data: PanelInternalData) -> Schedule {
        let mut s = Schedule::default();
        s.insert(id, data);
        s
    }

    // ── Field set wiring ─────────────────────────────────────────────────

    #[test]
    fn field_set_contains_all_declared_fields() {
        let fs = PanelEntityType::field_set();
        let count = fs.fields().count();
        assert_eq!(count, 35);
    }

    #[test]
    fn field_set_aliases_resolve() {
        let fs = PanelEntityType::field_set();
        assert!(fs.get_by_name("id").is_some());
        assert!(fs.get_by_name("title").is_some());
        assert!(fs.get_by_name("rooms").is_some());
        assert!(fs.get_by_name("kind").is_some());
    }

    #[test]
    fn required_fields_are_code_and_name() {
        let fs = PanelEntityType::field_set();
        let names: Vec<_> = fs.required_fields().map(|d| d.name).collect();
        assert_eq!(names, vec!["code", "name"]);
    }

    // ── Stored field read/write ──────────────────────────────────────────

    #[test]
    fn read_code_and_name() {
        let id = new_panel_id();
        let s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        assert_eq!(
            fs.read_field_value("code", id, &s).unwrap(),
            Some(field_string!("GP001"))
        );
        // `"uid"` alias still resolves.
        assert_eq!(
            fs.read_field_value("uid", id, &s).unwrap(),
            Some(field_string!("GP001"))
        );
        assert_eq!(
            fs.read_field_value("title", id, &s).unwrap(),
            Some(field_string!("Panel Name"))
        );
    }

    #[test]
    fn write_code_parses_uniq_id() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("code", id, &mut s, field_string!("GW007"))
            .unwrap();
        assert_eq!(
            fs.read_field_value("code", id, &s).unwrap(),
            Some(field_string!("GW007"))
        );
    }

    #[test]
    fn write_code_rejects_unparsable_string() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        let r = fs.write_field_value("code", id, &mut s, field_string!(""));
        assert!(matches!(r, Err(FieldError::Conversion(_))));
    }

    #[test]
    fn write_description_uses_text_variant() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("description", id, &mut s, field_text!("updated bio"))
            .unwrap();
        assert_eq!(
            fs.read_field_value("description", id, &s).unwrap(),
            Some(field_text!("updated bio"))
        );
    }

    #[test]
    fn write_optional_string_to_none_clears() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("cost", id, &mut s, field_value!(empty_list))
            .unwrap();
        assert_eq!(fs.read_field_value("cost", id, &s).unwrap(), None);
    }

    #[test]
    fn write_bool_and_i64() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("is_free", id, &mut s, field_boolean!(true))
            .unwrap();
        fs.write_field_value("capacity", id, &mut s, field_integer!(99))
            .unwrap();
        assert_eq!(
            fs.read_field_value("is_free", id, &s).unwrap(),
            Some(field_boolean!(true))
        );
        assert_eq!(
            fs.read_field_value("capacity", id, &s).unwrap(),
            Some(field_integer!(99))
        );
    }

    #[test]
    fn write_wrong_variant_is_error() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        let r = fs.write_field_value("code", id, &mut s, field_integer!(1));
        assert!(matches!(r, Err(FieldError::Conversion(_))));
    }

    // ── Time projections ─────────────────────────────────────────────────

    #[test]
    fn read_time_projections() {
        let id = new_panel_id();
        let s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        assert!(matches!(
            fs.read_field_value("start_time", id, &s).unwrap(),
            Some(crate::value::FieldValue::Single(
                crate::value::FieldValueItem::DateTime(_)
            ))
        ));
        assert!(matches!(
            fs.read_field_value("end_time", id, &s).unwrap(),
            Some(crate::value::FieldValue::Single(
                crate::value::FieldValueItem::DateTime(_)
            ))
        ));
        assert_eq!(
            fs.read_field_value("duration", id, &s).unwrap(),
            Some(field_duration!(Duration::minutes(60)))
        );
    }

    #[test]
    fn write_duration_updates_time_slot() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value(
            "duration",
            id,
            &mut s,
            field_duration!(Duration::minutes(90)),
        )
        .unwrap();
        assert_eq!(
            fs.read_field_value("duration", id, &s).unwrap(),
            Some(field_duration!(Duration::minutes(90)))
        );
    }

    #[test]
    fn write_start_time_via_string() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value(
            "start_time",
            id,
            &mut s,
            field_string!("2026-06-26T15:00:00"),
        )
        .unwrap();
        let expected = NaiveDate::from_ymd_opt(2026, 6, 26)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap();
        assert_eq!(
            fs.read_field_value("start_time", id, &s).unwrap(),
            Some(field_datetime!(expected))
        );
    }

    #[test]
    fn write_duration_from_integer_minutes() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("duration", id, &mut s, field_integer!(120))
            .unwrap();
        assert_eq!(
            fs.read_field_value("duration", id, &s).unwrap(),
            Some(field_duration!(Duration::minutes(120)))
        );
    }

    // ── Edge-backed stubs ────────────────────────────────────────────────

    #[test]
    fn read_edge_stubs_are_empty() {
        let id = new_panel_id();
        let s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        assert_eq!(
            fs.read_field_value("presenters", id, &s).unwrap(),
            Some(field_value!(empty_list))
        );
        assert_eq!(
            fs.read_field_value("rooms", id, &s).unwrap(),
            Some(field_value!(empty_list))
        );
        assert_eq!(
            fs.read_field_value("panel_type", id, &s).unwrap(),
            Some(field_value!(empty_list))
        );
    }

    #[test]
    fn write_edge_stub_is_noop() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        // Should not error even though backing storage is not wired up.
        fs.write_field_value("add_presenters", id, &mut s, field_value!(empty_list))
            .unwrap();
    }

    #[test]
    fn write_inclusive_presenters_is_read_only() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        let r = fs.write_field_value("inclusive_presenters", id, &mut s, field_value!(empty_list));
        assert!(matches!(r, Err(FieldError::ReadOnly { .. })));
    }

    // ── Serialization ────────────────────────────────────────────────────

    #[test]
    fn common_data_serde_round_trip() {
        let c = sample_common();
        let json = serde_json::to_string(&c).unwrap();
        let back: PanelCommonData = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn panel_data_serde_round_trip() {
        let id = new_panel_id();
        let internal = sample_internal(id);
        let data = PanelEntityType::export(&internal);
        let json = serde_json::to_string(&data).unwrap();
        let back: PanelData = serde_json::from_str(&json).unwrap();
        assert_eq!(data, back);
    }

    #[test]
    fn export_projects_time_slot() {
        let id = new_panel_id();
        let internal = sample_internal(id);
        let data = PanelEntityType::export(&internal);
        assert!(data.start_time.is_some());
        assert!(data.end_time.is_some());
        assert_eq!(data.duration, Some(Duration::minutes(60)));
    }

    // ── Validation ───────────────────────────────────────────────────────

    #[test]
    fn validate_passes_with_empty_name() {
        let id = new_panel_id();
        let internal = PanelInternalData {
            id,
            data: PanelCommonData::default(),
            code: PanelUniqId::parse("GP001").expect("valid"),
            time_slot: TimeRange::Unspecified,
        };
        let errors = PanelEntityType::validate(&internal);
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_detects_non_positive_duration() {
        // `ScheduledWithEnd` is guaranteed positive by construction, so the
        // reachable failure case for time_slot validation is a zero or
        // negative `ScheduledWithDuration`.
        let id = new_panel_id();
        let mut internal = sample_internal(id);
        internal.time_slot = TimeRange::ScheduledWithDuration {
            start_time: NaiveDate::from_ymd_opt(2026, 6, 26)
                .unwrap()
                .and_hms_opt(14, 0, 0)
                .unwrap(),
            duration: Duration::zero(),
        };
        let errors = PanelEntityType::validate(&internal);
        assert!(errors.iter().any(
            |e| matches!(e, ValidationError::Constraint { field, .. } if *field == "time_slot")
        ));
    }
}
