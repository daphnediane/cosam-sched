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
//! via edge-backed computed fields wired through `Schedule::edges_from` /
//! `Schedule::edges_to` using `crate::field::ReadFn::Schedule` / `crate::field::WriteFn::Schedule`.

use crate::converter::{AsBoolean, AsInteger, AsString, AsText, EntityStringResolver};
use crate::entity::{EntityId, EntityType, EntityUuid, FieldSet};
use crate::event_room::{
    EventRoomEntityType, EventRoomId, FIELD_PANELS as EVENT_ROOM_FIELD_PANELS,
};
use crate::field::{FieldDescriptor, ReadFn, VerifyFn, WriteFn};
use crate::field_macros::{define_field, edge_field, stored_field};
use crate::field_node_id::FieldNodeId;
use crate::field_value;
use crate::hotel_room::{HotelRoomEntityType, HotelRoomId};
use crate::panel_type::{PanelTypeEntityType, PanelTypeId};
use crate::panel_uniq_id::PanelUniqId;
use crate::presenter::{PresenterCommonData, PresenterEntityType, PresenterId, FIELD_PANELS};
use crate::time::{parse_datetime, parse_duration, TimeRange};
use crate::value::{
    CrdtFieldType, FieldCardinality, FieldType, FieldTypeItem, FieldValue, FieldValueItem,
    ValidationError,
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    /// Formatted credit strings for display (hidePanelist, altPanelist, group resolution).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub credits: Vec<String>,
    /// Hotel rooms for this panel (traverses event_rooms => hotel room edges).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub hotel_room_ids: Vec<HotelRoomId>,
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
        // Note: credits and hotel_room_ids are computed fields that require
        // Schedule access. They are populated by the Schedule::export_panel method.
        PanelData {
            code: internal.code.full_id(),
            data: internal.data.clone(),
            start_time: internal.time_slot.start_time(),
            end_time: internal.time_slot.end_time(),
            duration: internal.time_slot.duration(),
            presenter_ids: Vec::new(),
            event_room_ids: Vec::new(),
            panel_type_id: None,
            credits: Vec::new(),
            hotel_room_ids: Vec::new(),
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
        type_name: PanelEntityType::TYPE_NAME,
        uuid_namespace: PanelEntityType::uuid_namespace,
        type_id: || std::any::TypeId::of::<PanelInternalData>(),
        read_field_fn: |schedule, uuid, field_name| {
            // SAFETY: uuid came from an existing PanelEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PanelEntityType>::new_unchecked(uuid) };
            PanelEntityType::field_set().read_field_value(field_name, id, schedule)
        },
        write_field_fn: |schedule, uuid, field_name, value| {
            // SAFETY: uuid came from an existing PanelEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PanelEntityType>::new_unchecked(uuid) };
            PanelEntityType::field_set().write_field_value(field_name, id, schedule, value)
        },
        build_fn: |schedule, uuid, fields| {
            crate::builder::build_entity::<PanelEntityType>(
                schedule,
                crate::entity::UuidPreference::Exact(uuid),
                fields
                    .iter()
                    .map(|(n, v)| (crate::field_set::FieldRef::Name(n), v.clone()))
                    .collect(),
            )
            .map(|id| id.entity_uuid())
        },
        snapshot_fn: |schedule, uuid| {
            use crate::field::ReadableField;
            // SAFETY: uuid came from an existing PanelEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PanelEntityType>::new_unchecked(uuid) };
            PanelEntityType::field_set()
                .fields()
                .filter(|d| d.read_fn.is_some() && d.write_fn.is_some())
                .filter_map(|d| {
                    d.read(id, schedule).ok().flatten().map(|v| (d.name, v))
                })
                .collect()
        },
        remove_fn: |schedule, uuid| {
            // SAFETY: uuid came from an existing PanelEntityType entity.
            let id = unsafe { crate::entity::EntityId::<PanelEntityType>::new_unchecked(uuid) };
            schedule.remove_entity::<PanelEntityType>(id);
        },
        rehydrate_fn: |schedule, uuid| {
            crate::crdt::rehydrate_entity::<PanelEntityType>(schedule, uuid)
        },
    }
}

// ── EntityBuildable ─────────────────────────────────────────────────────────────

impl crate::builder::EntityBuildable for PanelEntityType {
    fn default_data(id: EntityId<Self>) -> Self::InternalData {
        PanelInternalData {
            id,
            data: PanelCommonData::default(),
            code: PanelUniqId::default(),
            time_slot: TimeRange::default(),
        }
    }
}

// ── EntityStringResolver implementation ─────────────────────────────────────────

impl EntityStringResolver for PanelEntityType {
    fn entity_to_string(schedule: &crate::schedule::Schedule, id: EntityId<Self>) -> String {
        schedule
            .get_internal(id)
            .map(|data| format!("{}: {}", data.code.full_id(), data.data.name))
            .unwrap_or_else(|| id.to_string())
    }
}

// ── Stored field descriptors ──────────────────────────────────────────────────

define_field!(
    /// Panel `code` (Uniq ID) — stored as the parsed [`PanelUniqId`] on
    /// [`PanelInternalData`], exposed to the field system as a string.
    ///
    /// Hand-written because the storage type is not a plain `String`.
    /// Note: changing a panel's code prefix may reassign it to a different
    /// `PanelType`; the write path parses and mutates only — callers that change
    /// the prefix should also update the `panel_type` edge accordingly.
    pub static FIELD_CODE: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "code",
        display: "Uniq ID",
        description: "Panel Uniq ID (e.g. \"GP032\"), parsed from the Schedule sheet.",
        aliases: &["uid", "uniq_id", "id"],
        required: true,
        crdt_type: CrdtFieldType::Scalar,
        field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
        example: "GP032",
        order: 0,
        read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
            Some(field_value!(d.code.full_id()))
        })),
        write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
            let s = v.into_string()?;
            // Callers that change the prefix should update the panel_type edge.
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
        verify_fn: None,
    }
);

// @todo: Name can be empty, should be optional
stored_field!(FIELD_NAME, PanelEntityType, name, required, as: AsString,
    name: "name", display: "Name",
    desc: "Panel name / title.",
    aliases: &["title", "panel_name"],
    example: "Cosplay Foam Armor 101",
    order: 100);

stored_field!(FIELD_DESCRIPTION, PanelEntityType, description, optional, as: AsText,
    name: "description", display: "Description",
    desc: "Event description shown to attendees.",
    aliases: &["desc"],
    example: "Learn the basics of foam armor construction",
    order: 200);

stored_field!(FIELD_NOTE, PanelEntityType, note, optional, as: AsText,
    name: "note", display: "Note",
    desc: "Extra note displayed verbatim.",
    aliases: &[],
    example: "Bring your own materials",
    order: 300);

stored_field!(FIELD_NOTES_NON_PRINTING, PanelEntityType, notes_non_printing, optional, as: AsText,
    name: "notes_non_printing", display: "Notes (Non Printing)",
    desc: "Internal notes not shown to the public.",
    aliases: &["internal_notes"],
    example: "Internal note for staff",
    order: 400);

stored_field!(FIELD_WORKSHOP_NOTES, PanelEntityType, workshop_notes, optional, as: AsText,
    name: "workshop_notes", display: "Workshop Notes",
    desc: "Notes for workshop staff.",
    aliases: &[],
    example: "Staff notes for workshop",
    order: 500);

stored_field!(FIELD_POWER_NEEDS, PanelEntityType, power_needs, optional, as: AsString,
    name: "power_needs", display: "Power Needs",
    desc: "Power / electrical requirements.",
    aliases: &["power"],
    example: "2 outlets",
    order: 600);

stored_field!(FIELD_SEWING_MACHINES, PanelEntityType, sewing_machines, with_default, as: AsBoolean,
    name: "sewing_machines", display: "Sewing Machines",
    desc: "Whether sewing machines are required.",
    aliases: &["sewing"],
    example: "false",
    order: 700);

stored_field!(FIELD_AV_NOTES, PanelEntityType, av_notes, optional, as: AsText,
    name: "av_notes", display: "AV Notes",
    desc: "Audio/visual setup notes.",
    aliases: &["av"],
    example: "Projector needed",
    order: 800);

stored_field!(FIELD_DIFFICULTY, PanelEntityType, difficulty, optional, as: AsString,
    name: "difficulty", display: "Difficulty",
    desc: "Skill-level indicator (free text).",
    aliases: &[],
    example: "Beginner",
    order: 900);

stored_field!(FIELD_PREREQ, PanelEntityType, prereq, optional, as: AsString,
    name: "prereq", display: "Prerequisites",
    desc: "Comma-separated prerequisite Uniq IDs.",
    aliases: &["prerequisites"],
    example: "GP001",
    order: 1000);

stored_field!(FIELD_COST, PanelEntityType, cost, optional, as: AsString,
    name: "cost", display: "Cost",
    desc: "Raw cost cell value (e.g. \"$35\", \"Free\", \"Kids\").",
    aliases: &[],
    example: "$35",
    order: 1100);

stored_field!(FIELD_IS_FREE, PanelEntityType, is_free, with_default, as: AsBoolean,
    name: "is_free", display: "Is Free",
    desc: "Parsed during import: cost is blank, \"Free\", \"$0\", or \"N/A\".",
    aliases: &["free"],
    example: "false",
    order: 1200);

stored_field!(FIELD_IS_KIDS, PanelEntityType, is_kids, with_default, as: AsBoolean,
    name: "is_kids", display: "Is Kids",
    desc: "Parsed during import: cost indicates kids-only pricing.",
    aliases: &["kids"],
    example: "false",
    order: 1300);

stored_field!(FIELD_IS_FULL, PanelEntityType, is_full, with_default, as: AsBoolean,
    name: "is_full", display: "Full",
    desc: "Event is at capacity.",
    aliases: &["full"],
    example: "false",
    order: 1400);

stored_field!(FIELD_CAPACITY, PanelEntityType, capacity, optional, as: AsInteger,
    name: "capacity", display: "Capacity",
    desc: "Total seats available.",
    aliases: &[],
    example: "50",
    order: 1500);

stored_field!(FIELD_SEATS_SOLD, PanelEntityType, seats_sold, optional, as: AsInteger,
    name: "seats_sold", display: "Seats Sold",
    desc: "Number of seats pre-sold or reserved via ticketing.",
    aliases: &[],
    example: "25",
    order: 1600);

stored_field!(FIELD_PRE_REG_MAX, PanelEntityType, pre_reg_max, optional, as: AsInteger,
    name: "pre_reg_max", display: "Pre-reg Max",
    desc: "Maximum seats available for pre-registration.",
    aliases: &["prereg_max"],
    example: "40",
    order: 1700);

stored_field!(FIELD_TICKET_URL, PanelEntityType, ticket_url, optional, as: AsString,
    name: "ticket_url", display: "Ticket URL",
    desc: "URL for purchasing tickets.",
    aliases: &["ticket_sale"],
    example: "https://example.com/ticket",
    order: 1800);

stored_field!(FIELD_HAVE_TICKET_IMAGE, PanelEntityType, have_ticket_image, with_default, as: AsBoolean,
    name: "have_ticket_image", display: "Have Ticket Image",
    desc: "Whether a ticket / flyer image has been received.",
    aliases: &[],
    example: "false",
    order: 1900);

stored_field!(FIELD_SIMPLETIX_EVENT, PanelEntityType, simpletix_event, optional, as: AsString,
    name: "simpletix_event", display: "SimpleTix Event",
    desc: "Internal admin URL for SimpleTix event configuration.",
    aliases: &["simpletix"],
    example: "https://admin.simpletix.com/event/123",
    order: 2000);

stored_field!(FIELD_SIMPLETIX_LINK, PanelEntityType, simpletix_link, optional, as: AsString,
    name: "simpletix_link", display: "SimpleTix Link",
    desc: "Public-facing direct ticket purchase link.",
    aliases: &[],
    example: "https://simpletix.com/event/123",
    order: 2100);

stored_field!(FIELD_HIDE_PANELIST, PanelEntityType, hide_panelist, with_default, as: AsBoolean,
    name: "hide_panelist", display: "Hide Panelist",
    desc: "Suppress presenter credits for this panel.",
    aliases: &[],
    example: "false",
    order: 2200);

stored_field!(FIELD_ALT_PANELIST, PanelEntityType, alt_panelist, optional, as: AsString,
    name: "alt_panelist", display: "Alt Panelist",
    desc: "Override text for the presenter credits line.",
    aliases: &[],
    example: "Special Guest",
    order: 2300);

// ── Computed time projections ─────────────────────────────────────────────────

define_field!(
    /// Start time — projected from `time_slot`.
    pub static FIELD_START_TIME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "start_time",
        display: "Start Time",
        description: "Panel start time.",
        aliases: &["start"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::DateTime),
        example: "2023-06-25T19:00:00",
        order: 2400,
        read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
            d.time_slot.start_time().map(|dt| field_value!(dt))
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
        verify_fn: Some(VerifyFn::ReRead),
    }
);

define_field!(
    /// End time — projected from `time_slot`.
    pub static FIELD_END_TIME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "end_time",
        display: "End Time",
        description: "Panel end time.",
        aliases: &["end"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::DateTime),
        example: "2023-06-25T20:30:00",
        order: 2500,
        read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
            d.time_slot.end_time().map(|dt| field_value!(dt))
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
        verify_fn: Some(VerifyFn::ReRead),
    }
);

define_field!(
    /// Duration — projected from `time_slot`.
    pub static FIELD_DURATION: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "duration",
        display: "Duration",
        description: "Panel duration.",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Duration),
        example: "90",
        order: 2600,
        read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
            d.time_slot.duration().map(|dur| field_value!(dur))
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
        verify_fn: Some(VerifyFn::ReRead),
    }
);

// ── Edge-backed computed fields ───────────────────────────────────────────────

edge_field!(FIELD_PRESENTERS, PanelEntityType, mode: ro, target: PresenterEntityType, target_field: &crate::presenter::FIELD_PANELS,
    edge: &EDGE_PANEL_PRESENTERS,
    name: "presenters", display: "Presenters",
    desc: "All presenters attached to this panel (credited and uncredited).",
    aliases: &["panelists", "presenter"],
    example: "[]",
    order: 2700);

define_field!(
    /// Credited presenters for this panel.
    ///
    /// **Read:** Returns the `EntityId`s of all presenters whose per-edge
    /// `credited` flag is `true` (the default when no flag has been set).
    ///
    /// **Write:** Sets the exact list of credited presenters.  Any presenter
    /// currently credited but absent from the new list is removed from the
    /// panel entirely; every presenter in the list is added to the panel (if
    /// not already attached) and has their `credited` flag set to `true`.
    /// Presenters in the uncredited partition are not affected.
    pub static FIELD_CREDITED_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "credited_presenters",
        display: "Credited Presenters",
        description: "Presenters credited on this panel. Read returns credited subset; write replaces it.",
        aliases: &["credited_panelists", "credited_presenter"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(
            FieldCardinality::List,
            FieldTypeItem::EntityIdentifier(PresenterEntityType::TYPE_NAME),
        ),
        example: "[presenter_id]",
        order: 2710,
        read_fn: Some(ReadFn::Schedule(
            |sched: &crate::schedule::Schedule, id: PanelId| {
                let node = crate::field_node_id::FieldNodeId::new(id, &FIELD_PRESENTERS);
                let ids: Vec<PresenterId> = sched
                    .connected_entities::<PanelEntityType, PresenterEntityType>(node, &FIELD_PANELS)
                    .into_iter()
                    .filter(|&p| {
                        sched.edge_get_bool::<PanelEntityType, PresenterEntityType>(
                            id, p, "credited",
                        )
                    })
                    .collect();
                Some(crate::schedule::entity_ids_to_field_value(ids))
            },
        )),
        write_fn: Some(WriteFn::Schedule(|sched, panel_id, val| {
            let new_ids =
                crate::schedule::field_value_to_entity_ids::<PresenterEntityType>(val)?;
            // Credited presenters absent from the new list are removed entirely.
            let node = crate::field_node_id::FieldNodeId::new(panel_id, &FIELD_PRESENTERS);
            let current_credited: Vec<PresenterId> = sched
                .connected_entities::<PanelEntityType, PresenterEntityType>(node, &FIELD_PANELS)
                .into_iter()
                .filter(|&p| {
                    sched.edge_get_bool::<PanelEntityType, PresenterEntityType>(
                        panel_id, p, "credited",
                    )
                })
                .collect();
            for p in &current_credited {
                if !new_ids.contains(p) {
                    sched.edge_remove::<PanelEntityType, PresenterEntityType>(
                        FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
                        FieldNodeId::new(*p, &FIELD_PANELS),
                    );
                }
            }
            // Each presenter in the new list → add edge if needed, then mark credited.
            // If a presenter was previously uncredited, they are now credited.
            for p in &new_ids {
                sched.edge_add::<PanelEntityType, PresenterEntityType>(
                    FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
                    FieldNodeId::new(*p, &FIELD_PANELS),
                );
                sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
                    panel_id, *p, "credited", true,
                );
            }
            Ok(())
        })),
        verify_fn: None,
    }
);

define_field!(
    /// Uncredited presenters for this panel.
    ///
    /// **Read:** Returns the `EntityId`s of all presenters whose per-edge
    /// `credited` flag is `false`.
    ///
    /// **Write:** Sets the exact list of uncredited presenters.  Any presenter
    /// currently uncredited but absent from the new list is removed from the
    /// panel entirely; every presenter in the list is added to the panel (if
    /// not already attached) and has their `credited` flag set to `false`.
    /// Presenters in the credited partition are not affected.
    pub static FIELD_UNCREDITED_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "uncredited_presenters",
        display: "Uncredited Presenters",
        description: "Presenters attached but not credited on this panel. Read returns uncredited subset; write replaces it.",
        aliases: &["uncredited_panelists", "uncredited_presenter"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(
            FieldCardinality::List,
            FieldTypeItem::EntityIdentifier(PresenterEntityType::TYPE_NAME),
        ),
        example: "[presenter_id]",
        order: 2720,
        read_fn: Some(ReadFn::Schedule(
            |sched: &crate::schedule::Schedule, id: PanelId| {
                let node = crate::field_node_id::FieldNodeId::new(id, &FIELD_PRESENTERS);
                let ids: Vec<PresenterId> = sched
                    .connected_entities::<PanelEntityType, PresenterEntityType>(node, &FIELD_PANELS)
                    .into_iter()
                    .filter(|&p| {
                        !sched.edge_get_bool::<PanelEntityType, PresenterEntityType>(
                            id, p, "credited",
                        )
                    })
                    .collect();
                Some(crate::schedule::entity_ids_to_field_value(ids))
            },
        )),
        write_fn: Some(WriteFn::Schedule(|sched, panel_id, val| {
            let new_ids =
                crate::schedule::field_value_to_entity_ids::<PresenterEntityType>(val)?;
            // Uncredited presenters absent from the new list are removed entirely.
            let node = crate::field_node_id::FieldNodeId::new(panel_id, &FIELD_PRESENTERS);
            let current_uncredited: Vec<PresenterId> = sched
                .connected_entities::<PanelEntityType, PresenterEntityType>(node, &FIELD_PANELS)
                .into_iter()
                .filter(|&p| {
                    !sched.edge_get_bool::<PanelEntityType, PresenterEntityType>(
                        panel_id, p, "credited",
                    )
                })
                .collect();
            for p in &current_uncredited {
                if !new_ids.contains(p) {
                    sched.edge_remove::<PanelEntityType, PresenterEntityType>(
                        FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
                        FieldNodeId::new(*p, &FIELD_PANELS),
                    );
                }
            }
            // Each presenter in the new list → add edge if needed, then mark uncredited.
            // If a presenter was previously credited, they are now uncredited.
            for p in &new_ids {
                sched.edge_add::<PanelEntityType, PresenterEntityType>(
                    FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
                    FieldNodeId::new(*p, &FIELD_PANELS),
                );
                sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
                    panel_id, *p, "credited", false,
                );
            }
            Ok(())
        })),
        verify_fn: None,
    }
);

define_field!(
    /// Add presenters to this panel and mark them as credited.
    ///
    /// Write-only.  Each presenter in the list is added to the panel (if not
    /// already attached) and has their `credited` flag set to `true`.
    pub static FIELD_ADD_CREDITED_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "add_credited_presenters",
        display: "Add Credited Presenters",
        description: "Add presenters to this panel and mark them as credited.",
        aliases: &["add_credited_presenter"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(
            FieldCardinality::List,
            FieldTypeItem::EntityIdentifier(PresenterEntityType::TYPE_NAME),
        ),
        example: "[presenter_id]",
        order: 2730,
        read_fn: None,
        write_fn: Some(WriteFn::Schedule(|sched, panel_id, val| {
            let ids =
                crate::schedule::field_value_to_entity_ids::<PresenterEntityType>(val)?;
            for p in ids {
                sched.edge_add::<PanelEntityType, PresenterEntityType>(
                    FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
                    FieldNodeId::new(p, &FIELD_PANELS),
                );
                sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
                    panel_id, p, "credited", true,
                );
            }
            Ok(())
        })),
        verify_fn: None,
    }
);

define_field!(
    /// Add presenters to this panel and mark them as uncredited.
    ///
    /// Write-only.  Each presenter in the list is added to the panel (if not
    /// already attached) and has their `credited` flag set to `false`.
    pub static FIELD_ADD_UNCREDITED_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "add_uncredited_presenters",
        display: "Add Uncredited Presenters",
        description: "Add presenters to this panel and mark them as uncredited.",
        aliases: &["add_uncredited_presenter"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(
            FieldCardinality::List,
            FieldTypeItem::EntityIdentifier(PresenterEntityType::TYPE_NAME),
        ),
        example: "[presenter_id]",
        order: 2740,
        read_fn: None,
        write_fn: Some(WriteFn::Schedule(|sched, panel_id, val| {
            let ids =
                crate::schedule::field_value_to_entity_ids::<PresenterEntityType>(val)?;
            for p in ids {
                sched.edge_add::<PanelEntityType, PresenterEntityType>(
                    FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
                    FieldNodeId::new(p, &FIELD_PANELS),
                );
                sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
                    panel_id, p, "credited", false,
                );
            }
            Ok(())
        })),
        verify_fn: None,
    }
);

edge_field!(FIELD_REMOVE_PRESENTERS, PanelEntityType, mode: remove, target: PresenterEntityType, target_field: &crate::presenter::FIELD_PANELS,
    name: "remove_presenters", display: "Remove Presenters",
    desc: "Remove presenters from this panel.",
    aliases: &["remove_presenter"],
    example: "[presenter_id]",
    order: 2900);

define_field!(
    /// Inclusive presenters for a panel.
    ///
    /// For each direct presenter `P` of this panel, the inclusive set contains:
    /// - `P` itself
    /// - All transitive groups of `P` (following forward homogeneous edges upward:
    ///   `P → Group → ParentGroup → …`)
    /// - All transitive members of `P` (following reverse homogeneous edges downward:
    ///   `P ← Member ← SubMember ← …`)
    ///
    /// Crucially, the expansion does **not** cross boundaries: groups of members
    /// and members of groups are not included. For example, if a panel lists
    /// Team A, the result includes Team A, its parent groups (Division C, Corp D)
    /// and its members (Alice, Bob) — but not Team B (a sibling in Division C),
    /// and not Club E (a group of Alice's that has nothing to do with Team A).
    pub static FIELD_INCLUSIVE_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "inclusive_presenters",
        display: "Inclusive Presenters",
        description: "Direct presenters + their transitive groups + their transitive members.",
        aliases: &["inclusive_presenter"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::List, FieldTypeItem::EntityIdentifier(
            PresenterEntityType::TYPE_NAME,
        )),
        example: "[]",
        order: 3000,
        read_fn: Some(ReadFn::Schedule(|sched, panel_id| {
            use std::collections::HashSet;
            let node = crate::field_node_id::FieldNodeId::new(panel_id, &FIELD_PRESENTERS);
            let direct = sched.connected_entities::<PanelEntityType, PresenterEntityType>(node, &FIELD_PANELS);
            let mut result: HashSet<PresenterId> = HashSet::new();
            for p in direct {
                result.insert(p);
                // Inclusive groups of p: all groups reachable going up (forward homogeneous edges)
                for g in sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(
                    crate::field_node_id::FieldNodeId::new(p, &crate::presenter::FIELD_MEMBERS),
                    &crate::presenter::FIELD_GROUPS,
                ) {
                    result.insert(g);
                }
                // Inclusive members of p: all members reachable going down (reverse homogeneous edges)
                for m in sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(
                    crate::field_node_id::FieldNodeId::new(p, &crate::presenter::FIELD_GROUPS),
                    &crate::presenter::FIELD_MEMBERS,
                ) {
                    result.insert(m);
                }
            }
            let ids: Vec<PresenterId> = result.into_iter().collect();
            Some(crate::schedule::entity_ids_to_field_value(ids))
        })),
        write_fn: None,
        verify_fn: None,
    }
);

edge_field!(FIELD_EVENT_ROOMS, PanelEntityType, mode: rw, target: EventRoomEntityType, target_field: &crate::event_room::FIELD_PANELS,
    edge: &EDGE_PANEL_EVENT_ROOMS,
    name: "event_rooms", display: "Event Rooms",
    desc: "Rooms where this panel takes place.",
    aliases: &["rooms", "room", "event_room"],
    example: "[]",
    order: 3100);

edge_field!(FIELD_ADD_ROOMS, PanelEntityType, mode: add, target: EventRoomEntityType, target_field: EVENT_ROOM_FIELD_PANELS,
    name: "add_rooms", display: "Add Rooms",
    desc: "Append event rooms to this panel.",
    aliases: &["add_room"],
    example: "[room_id]",
    order: 3200);

edge_field!(FIELD_REMOVE_ROOMS, PanelEntityType, mode: remove, target: EventRoomEntityType, target_field: &crate::event_room::FIELD_PANELS,
    name: "remove_rooms", display: "Remove Rooms",
    desc: "Remove event rooms from this panel.",
    aliases: &["remove_room"],
    example: "[room_id]",
    order: 3300);

edge_field!(FIELD_PANEL_TYPE, PanelEntityType, mode: one, target: PanelTypeEntityType, target_field: &crate::panel_type::FIELD_PANELS,
    edge: &EDGE_PANEL_PANEL_TYPE,
    name: "panel_type", display: "Panel Type",
    desc: "Panel type / kind.",
    aliases: &["kind", "type"],
    example: "{}",
    order: 3400);

// ── Read-only computed fields ─────────────────────────────────────────────────────

define_field!(
    /// Hotel rooms for this panel (traverses event_rooms => hotel room edges).
    static FIELD_HOTEL_ROOMS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "hotel_rooms",
        display: "Hotel Rooms",
        description: "Hotel rooms where this panel takes place (traverses event rooms).",
        aliases: &["hotel_room"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(
            FieldCardinality::List,
            FieldTypeItem::EntityIdentifier(HotelRoomEntityType::TYPE_NAME),
        ),
        example: "[]",
        order: 3500,
        read_fn: Some(ReadFn::Schedule(
            |sched: &crate::schedule::Schedule, id: PanelId| {
                let node = crate::field_node_id::FieldNodeId::new(id, &FIELD_EVENT_ROOMS);
                let event_room_ids = sched.connected_entities::<PanelEntityType, EventRoomEntityType>(node, &EVENT_ROOM_FIELD_PANELS);
                let mut hotel_room_ids: HashSet<HotelRoomId> = HashSet::new();
                for event_room_id in event_room_ids {
                    let node = crate::field_node_id::FieldNodeId::new(event_room_id, &crate::event_room::FIELD_HOTEL_ROOMS);
                    let rooms = sched.connected_entities::<EventRoomEntityType, HotelRoomEntityType>(node, &crate::hotel_room::FIELD_EVENT_ROOMS);
                    hotel_room_ids.extend(rooms);
                }
                let hotel_room_ids: Vec<HotelRoomId> = hotel_room_ids.into_iter().collect();
                Some(crate::schedule::entity_ids_to_field_value(hotel_room_ids))
            },
        )),
        write_fn: None,
        verify_fn: None,
    }
);

// ── Credits computation ───────────────────────────────────────────────────────

/// Compute the formatted presenter credit strings for `panel_id`.
///
/// Applies `hide_panelist` / `alt_panelist` overrides, then filters to
/// credited presenters (per the per-edge `credited` bool), then formats each
/// credit entry accounting for groups, `always_shown_in_group`, and
/// `always_grouped` members.
///
/// The presenter lookup is built from **all** presenters in the schedule so
/// that group entities that are not themselves panel edges can still be
/// resolved for name formatting.
pub(crate) fn compute_credits(sched: &crate::schedule::Schedule, panel_id: PanelId) -> Vec<String> {
    let panel_internal = match sched.get_internal::<PanelEntityType>(panel_id) {
        Some(p) => p,
        None => return Vec::new(),
    };
    if panel_internal.data.hide_panelist {
        return Vec::new();
    }
    if let Some(ref alt) = panel_internal.data.alt_panelist {
        return vec![alt.clone()];
    }

    let node = crate::field_node_id::FieldNodeId::new(panel_id, &FIELD_PRESENTERS);
    let all_presenter_ids =
        sched.connected_entities::<PanelEntityType, PresenterEntityType>(node, &FIELD_PANELS);
    if all_presenter_ids.is_empty() {
        return Vec::new();
    }
    let credited_ids: Vec<PresenterId> = all_presenter_ids
        .into_iter()
        .filter(|&p| {
            sched.edge_get_bool::<PanelEntityType, PresenterEntityType>(panel_id, p, "credited")
        })
        .collect();
    if credited_ids.is_empty() {
        return Vec::new();
    }

    // Build a schedule-wide lookup so group entities not directly on this
    // panel (e.g. referenced only via always_grouped membership) can be found.
    let presenter_lookup: HashMap<PresenterId, &PresenterCommonData> = sched
        .iter_entities::<PresenterEntityType>()
        .map(|(id, internal)| (id, &internal.data))
        .collect();

    let mut credits: Vec<String> = Vec::new();
    let mut used_as_member: HashSet<PresenterId> = HashSet::new();
    let mut used_groups: HashSet<PresenterId> = HashSet::new();

    // First pass: handle explicit groups and always_grouped members.
    for &presenter_id in &credited_ids {
        if used_as_member.contains(&presenter_id) {
            continue;
        }
        let Some(&presenter_data) = presenter_lookup.get(&presenter_id) else {
            continue;
        };
        if presenter_data.is_explicit_group {
            let node = crate::field_node_id::FieldNodeId::new(
                presenter_id,
                &crate::presenter::FIELD_MEMBERS,
            );
            let member_ids = sched.connected_entities::<PresenterEntityType, PresenterEntityType>(
                node,
                &crate::presenter::FIELD_GROUPS,
            );
            let all_members: HashSet<PresenterId> = member_ids.iter().cloned().collect();
            let credited_members: Vec<PresenterId> = all_members
                .iter()
                .filter(|m| credited_ids.contains(m))
                .cloned()
                .collect();

            if used_groups.contains(&presenter_id) {
                continue;
            }
            if presenter_data.always_shown_in_group {
                // Partial attendance → "Member of Group" / "Group (M1, M2)"; full → group name.
                if credited_members.len() < all_members.len() {
                    match credited_members.len() {
                        0 => credits.push(presenter_data.name.clone()),
                        1 => {
                            if let Some(m) = presenter_lookup.get(&credited_members[0]) {
                                credits.push(format!("{} of {}", m.name, presenter_data.name));
                            }
                        }
                        _ => {
                            let names: Vec<String> = credited_members
                                .iter()
                                .filter_map(|mid| presenter_lookup.get(mid).map(|d| d.name.clone()))
                                .collect();
                            credits.push(format!("{} ({})", presenter_data.name, names.join(", ")));
                        }
                    }
                    for m in &credited_members {
                        used_as_member.insert(*m);
                    }
                } else {
                    credits.push(presenter_data.name.clone());
                    for m in all_members {
                        used_as_member.insert(m);
                    }
                }
            } else {
                // Regular group: show name if all members present, else individuals.
                let show_as_group = member_ids.iter().all(|m| credited_ids.contains(m));
                if show_as_group {
                    credits.push(presenter_data.name.clone());
                    for m in member_ids {
                        used_as_member.insert(m);
                    }
                } else {
                    for m in member_ids {
                        if credited_ids.contains(&m) && !used_as_member.contains(&m) {
                            if let Some(md) = presenter_lookup.get(&m) {
                                credits.push(md.name.clone());
                                used_as_member.insert(m);
                            }
                        }
                    }
                }
            }
            used_groups.insert(presenter_id);
        } else if presenter_data.always_grouped {
            // This member always appears under their group's name.
            let node = crate::field_node_id::FieldNodeId::new(
                presenter_id,
                &crate::presenter::FIELD_MEMBERS,
            );
            let group_ids = sched.connected_entities::<PresenterEntityType, PresenterEntityType>(
                node,
                &crate::presenter::FIELD_GROUPS,
            );
            for group_id in group_ids {
                let Some(&group_data) = presenter_lookup.get(&group_id) else {
                    continue;
                };
                let node = crate::field_node_id::FieldNodeId::new(
                    group_id,
                    &crate::presenter::FIELD_MEMBERS,
                );
                let group_member_ids = sched
                    .connected_entities::<PresenterEntityType, PresenterEntityType>(
                        node,
                        &crate::presenter::FIELD_GROUPS,
                    );
                let show_as_group = group_data.always_shown_in_group
                    || group_member_ids.iter().all(|m| credited_ids.contains(m));

                if used_groups.contains(&group_id) || !show_as_group {
                    continue;
                }
                if group_data.always_shown_in_group {
                    let credited_members: Vec<PresenterId> = group_member_ids
                        .iter()
                        .filter(|m| credited_ids.contains(m))
                        .cloned()
                        .collect();
                    if credited_members.len() < group_member_ids.len() {
                        for m in &credited_members {
                            if let Some(md) = presenter_lookup.get(m) {
                                credits.push(format!("{} of {}", md.name, group_data.name));
                                used_as_member.insert(*m);
                            }
                        }
                    } else {
                        credits.push(group_data.name.clone());
                        for m in group_member_ids {
                            used_as_member.insert(m);
                        }
                    }
                } else {
                    credits.push(group_data.name.clone());
                    for m in group_member_ids {
                        used_as_member.insert(m);
                    }
                }
                used_groups.insert(group_id);
            }
        }
    }

    // Second pass: remaining individuals not consumed by a group.
    for &presenter_id in &credited_ids {
        if used_as_member.contains(&presenter_id) {
            continue;
        }
        if let Some(&pd) = presenter_lookup.get(&presenter_id) {
            if !pd.is_explicit_group && !pd.always_grouped {
                credits.push(pd.name.clone());
            }
        }
    }

    credits
}

define_field!(
    /// Formatted credit strings for display (hidePanelist, altPanelist, group resolution).
    static FIELD_CREDITS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "credits",
        display: "Credits",
        description: "Formatted presenter credit strings for display, accounting for hidePanelist, altPanelist, group resolution, always_shown, and always_grouped flags.",
        aliases: &["credit"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(
            FieldCardinality::List,
            FieldTypeItem::String,
        ),
        example: "[\"John Doe\", \"Group Name (Alice, Bob)\"]",
        order: 3600,
        read_fn: Some(ReadFn::Schedule(
            |sched: &crate::schedule::Schedule, id: PanelId| {
                let strings = compute_credits(sched, id);
                Some(FieldValue::List(
                    strings.into_iter().map(FieldValueItem::String).collect(),
                ))
            },
        )),
        write_fn: None,
        verify_fn: None,
    }
);

// ── FieldSet ──────────────────────────────────────────────────────────────────

static PANEL_FIELD_SET: LazyLock<FieldSet<PanelEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ... (rest of the code remains the same)
// ── Builder ───────────────────────────────────────────────────────────────────

crate::field_macros::define_entity_builder! {
    /// Typed builder for [`PanelEntityType`] entities.
    PanelBuilder for PanelEntityType {
        /// Set the Uniq ID code (e.g. `"GP032"`).  Required.  The write path
        /// parses the string; the `panel_type` edge is *not* updated
        /// automatically — callers changing the prefix should set it too.
        with_code                => FIELD_CODE,
        /// Set the panel name / title.  Required.
        with_name                => FIELD_NAME,
        /// Set the attendee-facing description.
        with_description         => FIELD_DESCRIPTION,
        /// Set the verbatim note displayed to attendees.
        with_note                => FIELD_NOTE,
        /// Set the internal (non-printing) note for staff.
        with_notes_non_printing  => FIELD_NOTES_NON_PRINTING,
        /// Set the workshop-staff notes.
        with_workshop_notes      => FIELD_WORKSHOP_NOTES,
        /// Set the free-text power / electrical requirements.
        with_power_needs         => FIELD_POWER_NEEDS,
        /// Mark whether sewing machines are required.
        with_sewing_machines     => FIELD_SEWING_MACHINES,
        /// Set the A/V setup notes.
        with_av_notes            => FIELD_AV_NOTES,
        /// Set the free-text skill-level indicator.
        with_difficulty          => FIELD_DIFFICULTY,
        /// Set the comma-separated prerequisite Uniq IDs.
        with_prereq              => FIELD_PREREQ,
        /// Set the raw cost cell value (e.g. `"$35"`, `"Free"`, `"Kids"`).
        with_cost                => FIELD_COST,
        /// Mark the panel as free (parsed from `cost` during import).
        with_is_free             => FIELD_IS_FREE,
        /// Mark the panel as kids-only (parsed from `cost` during import).
        with_is_kids             => FIELD_IS_KIDS,
        /// Mark the panel as at capacity.
        with_is_full             => FIELD_IS_FULL,
        /// Set the total seat capacity.
        with_capacity            => FIELD_CAPACITY,
        /// Set the number of seats already sold / reserved.
        with_seats_sold          => FIELD_SEATS_SOLD,
        /// Set the maximum pre-registration seat count.
        with_pre_reg_max         => FIELD_PRE_REG_MAX,
        /// Set the public ticket-purchase URL.
        with_ticket_url          => FIELD_TICKET_URL,
        /// Mark whether a ticket / flyer image has been received.
        with_have_ticket_image   => FIELD_HAVE_TICKET_IMAGE,
        /// Set the internal SimpleTix admin URL.
        with_simpletix_event     => FIELD_SIMPLETIX_EVENT,
        /// Set the public-facing SimpleTix purchase link.
        with_simpletix_link      => FIELD_SIMPLETIX_LINK,
        /// Suppress presenter credits for this panel.
        with_hide_panelist       => FIELD_HIDE_PANELIST,
        /// Override text for the presenter credits line.
        with_alt_panelist        => FIELD_ALT_PANELIST,
        /// Set the start time (projected onto `time_slot`).
        with_start_time          => FIELD_START_TIME,
        /// Set the end time (projected onto `time_slot`).
        with_end_time            => FIELD_END_TIME,
        /// Set the duration (projected onto `time_slot`).
        with_duration            => FIELD_DURATION,
        /// Replace the set of presenters credited for this panel.
        with_presenters          => FIELD_PRESENTERS,
        /// Replace the set of event rooms where this panel takes place.
        with_event_rooms         => FIELD_EVENT_ROOMS,
        /// Set the panel-type / kind edge.
        with_panel_type          => FIELD_PANEL_TYPE,
    }
}

// ── Edge descriptors ──────────────────────────────────────────────────────────

// ── Edge descriptors ──────────────────────────────────────────────────────────

/// Panel ↔ Presenter relationship.  Panel is the canonical CRDT owner.
///
/// The `credited` per-edge field is removed in FEATURE-065 when the single
/// `presenters` list splits into `credited_presenters` / `uncredited_presenters`.
pub(crate) static EDGE_PANEL_PRESENTERS: crate::edge_descriptor::EdgeDescriptor =
    crate::edge_descriptor::EdgeDescriptor {
        name: "panel_presenters",
        owner_field: &FIELD_PRESENTERS,
        target_field: &crate::presenter::FIELD_PANELS,
        fields: &[crate::edge_descriptor::EdgeFieldSpec {
            name: "credited",
            default: crate::edge_descriptor::EdgeFieldDefault::Boolean(true),
        }],
    };
inventory::submit! { crate::edge_descriptor::CollectedEdge(&EDGE_PANEL_PRESENTERS) }

/// Panel ↔ EventRoom relationship.  Panel is the canonical CRDT owner.
pub(crate) static EDGE_PANEL_EVENT_ROOMS: crate::edge_descriptor::EdgeDescriptor =
    crate::edge_descriptor::EdgeDescriptor {
        name: "panel_event_rooms",
        owner_field: &FIELD_EVENT_ROOMS,
        target_field: &crate::event_room::FIELD_PANELS,
        fields: &[],
    };
inventory::submit! { crate::edge_descriptor::CollectedEdge(&EDGE_PANEL_EVENT_ROOMS) }

/// Panel → PanelType relationship.  Panel is the canonical CRDT owner.
pub(crate) static EDGE_PANEL_PANEL_TYPE: crate::edge_descriptor::EdgeDescriptor =
    crate::edge_descriptor::EdgeDescriptor {
        name: "panel_panel_type",
        owner_field: &FIELD_PANEL_TYPE,
        target_field: &crate::panel_type::FIELD_PANELS,
        fields: &[],
    };
inventory::submit! { crate::edge_descriptor::CollectedEdge(&EDGE_PANEL_PANEL_TYPE) }

// ── EntityMatcher ─────────────────────────────────────────────────────────────

impl crate::lookup::EntityScannable for PanelEntityType {}

impl crate::lookup::EntityMatcher for PanelEntityType {
    fn match_entity(query: &str, data: &PanelInternalData) -> Option<crate::lookup::MatchPriority> {
        use crate::lookup::string_match_priority;
        // Match on code (full_id e.g. "GP001P2", base_id e.g. "GP001") and name.
        [
            string_match_priority(query, &data.code.full_id()),
            string_match_priority(query, &data.code.base_id()),
            string_match_priority(query, &data.data.name),
        ]
        .into_iter()
        .flatten()
        .max()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;
    use crate::value::FieldError;
    use crate::{field_text, field_value};
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn new_panel_id() -> PanelId {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        unsafe { PanelId::new_unchecked(non_nil_uuid) }
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
        assert_eq!(count, 40);
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
            Some(field_value!("GP001"))
        );
        // `"uid"` alias still resolves.
        assert_eq!(
            fs.read_field_value("uid", id, &s).unwrap(),
            Some(field_value!("GP001"))
        );
        assert_eq!(
            fs.read_field_value("title", id, &s).unwrap(),
            Some(field_value!("Panel Name"))
        );
    }

    #[test]
    fn write_code_parses_uniq_id() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("code", id, &mut s, field_value!("GW007"))
            .unwrap();
        assert_eq!(
            fs.read_field_value("code", id, &s).unwrap(),
            Some(field_value!("GW007"))
        );
    }

    #[test]
    fn write_code_rejects_unparsable_string() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        let r = fs.write_field_value("code", id, &mut s, field_value!(""));
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
        fs.write_field_value("is_free", id, &mut s, field_value!(true))
            .unwrap();
        fs.write_field_value("capacity", id, &mut s, field_value!(99))
            .unwrap();
        assert_eq!(
            fs.read_field_value("is_free", id, &s).unwrap(),
            Some(field_value!(true))
        );
        assert_eq!(
            fs.read_field_value("capacity", id, &s).unwrap(),
            Some(field_value!(99))
        );
    }

    #[test]
    fn write_wrong_variant_is_error() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        let r = fs.write_field_value("code", id, &mut s, field_value!(1));
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
            Some(field_value!(Duration::minutes(60)))
        );
    }

    #[test]
    fn write_duration_updates_time_slot() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("duration", id, &mut s, field_value!(Duration::minutes(90)))
            .unwrap();
        assert_eq!(
            fs.read_field_value("duration", id, &s).unwrap(),
            Some(field_value!(Duration::minutes(90)))
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
            field_value!("2026-06-26T15:00:00"),
        )
        .unwrap();
        let expected = NaiveDate::from_ymd_opt(2026, 6, 26)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap();
        assert_eq!(
            fs.read_field_value("start_time", id, &s).unwrap(),
            Some(field_value!(expected))
        );
    }

    #[test]
    fn write_duration_from_integer_minutes() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("duration", id, &mut s, field_value!(120))
            .unwrap();
        assert_eq!(
            fs.read_field_value("duration", id, &s).unwrap(),
            Some(field_value!(Duration::minutes(120)))
        );
    }

    // ── Edge-backed fields ───────────────────────────────────────────────

    #[test]
    fn read_edge_fields_empty_without_edges() {
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
        assert_eq!(
            fs.read_field_value("inclusive_presenters", id, &s).unwrap(),
            Some(field_value!(empty_list))
        );
    }

    #[test]
    fn write_add_presenters_is_no_error_for_empty_list() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value(
            "add_credited_presenters",
            id,
            &mut s,
            field_value!(empty_list),
        )
        .unwrap();
        fs.write_field_value(
            "add_uncredited_presenters",
            id,
            &mut s,
            field_value!(empty_list),
        )
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

    #[test]
    fn test_entity_to_string_returns_code_name_format() {
        use crate::converter::EntityStringResolver;
        let id = new_panel_id();
        let mut sched = Schedule::default();
        sched.insert(id, sample_internal(id));
        let s = PanelEntityType::entity_to_string(&sched, id);
        assert_eq!(s, "GP001: Panel Name");
    }

    #[test]
    fn test_entity_to_string_fallback_to_uuid() {
        use crate::converter::EntityStringResolver;
        let id = new_panel_id();
        let sched = Schedule::default();
        let s = PanelEntityType::entity_to_string(&sched, id);
        assert_eq!(s, id.to_string());
    }

    // ── credited flag ─────────────────────────────────────────────────────

    fn make_presenter_for_panel(
        sched: &mut Schedule,
        name: &str,
    ) -> crate::entity::EntityId<PresenterEntityType> {
        use crate::entity::UuidPreference;
        let pres_id = crate::entity::EntityId::<PresenterEntityType>::from_preference(
            UuidPreference::GenerateNew,
        );
        let data = crate::presenter::PresenterInternalData {
            id: pres_id,
            data: crate::presenter::PresenterCommonData {
                name: name.into(),
                ..Default::default()
            },
        };
        sched.insert(pres_id, data);
        pres_id
    }

    #[test]
    fn field_credits_excludes_uncredited_presenter() {
        let panel_id = new_panel_id();
        let mut sched = sched_with(panel_id, sample_internal(panel_id));

        let alice = make_presenter_for_panel(&mut sched, "Alice");
        let bob = make_presenter_for_panel(&mut sched, "Bob");

        sched.edge_add::<PanelEntityType, PresenterEntityType>(
            FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
            FieldNodeId::new(alice, &FIELD_PANELS),
        );
        sched.edge_add::<PanelEntityType, PresenterEntityType>(
            FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
            FieldNodeId::new(bob, &FIELD_PANELS),
        );
        // Mark Bob as uncredited.
        sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
            panel_id, bob, "credited", false,
        );

        let fs = PanelEntityType::field_set();
        let credits = fs.read_field_value("credits", panel_id, &sched).unwrap();
        let credits_str = format!("{credits:?}");
        assert!(
            credits_str.contains("Alice"),
            "Alice should appear in credits"
        );
        assert!(
            !credits_str.contains("Bob"),
            "Bob should be excluded from credits"
        );
    }

    #[test]
    fn credited_presenters_read_returns_only_credited() {
        let panel_id = new_panel_id();
        let mut sched = sched_with(panel_id, sample_internal(panel_id));

        let alice = make_presenter_for_panel(&mut sched, "Alice");
        let bob = make_presenter_for_panel(&mut sched, "Bob");

        sched.edge_add::<PanelEntityType, PresenterEntityType>(
            FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
            FieldNodeId::new(alice, &FIELD_PANELS),
        );
        sched.edge_add::<PanelEntityType, PresenterEntityType>(
            FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
            FieldNodeId::new(bob, &FIELD_PANELS),
        );
        sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
            panel_id, bob, "credited", false,
        );

        let fs = PanelEntityType::field_set();
        let val = fs
            .read_field_value("credited_presenters", panel_id, &sched)
            .unwrap();
        let credited = match val.unwrap() {
            crate::value::FieldValue::List(items) => items,
            other => panic!("expected List, got {other:?}"),
        };
        assert_eq!(credited.len(), 1);
        let rid = match &credited[0] {
            crate::value::FieldValueItem::EntityIdentifier(r) => *r,
            other => panic!("expected EntityIdentifier, got {other:?}"),
        };
        assert_eq!(rid.entity_uuid(), alice.entity_uuid());
    }

    #[test]
    fn credited_presenters_write_removes_absent_and_leaves_uncredited_partition() {
        let panel_id = new_panel_id();
        let mut sched = sched_with(panel_id, sample_internal(panel_id));

        let alice = make_presenter_for_panel(&mut sched, "Alice"); // credited
        let bob = make_presenter_for_panel(&mut sched, "Bob"); // credited, will be dropped
        let carol = make_presenter_for_panel(&mut sched, "Carol"); // uncredited — must be untouched

        sched.edge_add::<PanelEntityType, PresenterEntityType>(
            FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
            FieldNodeId::new(alice, &FIELD_PANELS),
        );
        sched.edge_add::<PanelEntityType, PresenterEntityType>(
            FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
            FieldNodeId::new(bob, &FIELD_PANELS),
        );
        sched.edge_add::<PanelEntityType, PresenterEntityType>(
            FieldNodeId::new(panel_id, &FIELD_PRESENTERS),
            FieldNodeId::new(carol, &FIELD_PANELS),
        );
        sched.edge_set_bool::<PanelEntityType, PresenterEntityType>(
            panel_id, carol, "credited", false,
        );

        // Write credited_presenters = [alice] — bob should be removed, carol untouched.
        let fs = PanelEntityType::field_set();
        let new_val =
            crate::value::FieldValue::List(vec![crate::value::FieldValueItem::EntityIdentifier(
                alice.into(),
            )]);
        fs.write_field_value("credited_presenters", panel_id, &mut sched, new_val)
            .unwrap();

        // Bob's edge should be gone.
        let all =
            sched.connected_entities(FieldNodeId::new(panel_id, &FIELD_PRESENTERS), &FIELD_PANELS);
        assert!(!all.contains(&bob), "Bob should have been removed");

        // Alice still present and credited.
        assert!(all.contains(&alice), "Alice should still be present");
        assert!(
            sched
                .edge_get_bool::<PanelEntityType, PresenterEntityType>(panel_id, alice, "credited"),
            "Alice should be credited"
        );

        // Carol still present and uncredited.
        assert!(
            all.contains(&carol),
            "Carol (uncredited) should be untouched"
        );
        assert!(
            !sched
                .edge_get_bool::<PanelEntityType, PresenterEntityType>(panel_id, carol, "credited"),
            "Carol should remain uncredited"
        );
    }
}
