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

use crate::accessor_field_properties;
use crate::define_field;
use crate::edge::EdgeKind;
use crate::entity::{EntityId, EntityType, EntityUuid, FieldSet};
use crate::field::{CollectedNamedField, FieldDescriptor, NamedField};
use crate::field_value;
use crate::query::converter::EntityStringResolver;
use crate::schedule::Schedule;
use crate::tables::event_room::{EventRoomEntityType, EventRoomId};
use crate::tables::hotel_room::{HotelRoomEntityType, HotelRoomId};
use crate::tables::panel_type::{PanelTypeEntityType, PanelTypeId};
use crate::tables::presenter::{
    PresenterCommonData, PresenterEntityType, PresenterId, EDGE_GROUPS, EDGE_MEMBERS,
};
use crate::value::time::{parse_datetime, parse_duration, TimeRange};
use crate::value::uniq_id::PanelUniqId;
use crate::value::{FieldTypeItem, FieldValue, FieldValueItem, ValidationError};
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
            crate::edit::builder::build_entity::<PanelEntityType>(
                schedule,
                crate::entity::UuidPreference::Exact(uuid),
                fields
                    .iter()
                    .map(|(n, v)| (crate::field::set::FieldRef::Name(n), v.clone()))
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
                .filter(|d| d.cb.read_fn.is_some() && d.cb.write_fn.is_some())
                .filter_map(|d| {
                    d.read(id, schedule).ok().flatten().map(|v| (d.name(), v))
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

impl crate::edit::builder::EntityBuildable for PanelEntityType {
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

define_field! {
    /// Panel `code` (Uniq ID) — stored as the parsed [`PanelUniqId`] on
    /// [`PanelInternalData`], exposed to the field system as a string.
    ///
    /// Hand-written because the storage type is not a plain `String`.
    /// Note: changing a panel's code prefix may reassign it to a different
    /// `PanelType`; the write path parses and mutates only — callers that change
    /// the prefix should also update the `panel_type` edge accordingly.
    static FIELD_CODE: FieldDescriptor<PanelEntityType>,
    name: "code", display: "Uniq ID",
    desc: "Panel Uniq ID (e.g. \"GP032\"), parsed from the Schedule sheet.",
    aliases: &["uid", "uniq_id", "id"],
    required,
    example: "GP032",
    order: 0,
    crdt: Scalar, cardinality: single, item: FieldTypeItem::String,
    read: |d: &PanelInternalData| {
        Some(field_value!(d.code.full_id()))
    },
    write: |d: &mut PanelInternalData, v: FieldValue| {
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
    }
}

// @todo: Name can be empty, should be optional
pub static FIELD_NAME: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        name,
        name: "name",
        display: "Name",
        description: "Panel name / title.",
        aliases: &["title", "panel_name"],
        cardinality: Single,
        item: String,
        example: "Cosplay Foam Armor 101",
        order: 100,
    };
    FieldDescriptor {
        data,
        required: true,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_NAME) }

pub static FIELD_DESCRIPTION: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        description,
        name: "description",
        display: "Description",
        description: "Event description shown to attendees.",
        aliases: &["desc"],
        cardinality: Optional,
        item: Text,
        example: "Learn the basics of foam armor construction",
        order: 200,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_DESCRIPTION) }

pub static FIELD_NOTE: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        note,
        name: "note",
        display: "Note",
        description: "Extra note displayed verbatim.",
        aliases: &[],
        cardinality: Optional,
        item: Text,
        example: "Bring your own materials",
        order: 300,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_NOTE) }

pub static FIELD_NOTES_NON_PRINTING: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        notes_non_printing,
        name: "notes_non_printing",
        display: "Notes (Non Printing)",
        description: "Internal notes not shown to the public.",
        aliases: &["internal_notes"],
        cardinality: Optional,
        item: Text,
        example: "Internal note for staff",
        order: 400,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_NOTES_NON_PRINTING) }

pub static FIELD_WORKSHOP_NOTES: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        workshop_notes,
        name: "workshop_notes",
        display: "Workshop Notes",
        description: "Notes for workshop staff.",
        aliases: &[],
        cardinality: Optional,
        item: Text,
        example: "Staff notes for workshop",
        order: 500,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_WORKSHOP_NOTES) }

pub static FIELD_POWER_NEEDS: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        power_needs,
        name: "power_needs",
        display: "Power Needs",
        description: "Power / electrical requirements.",
        aliases: &["power"],
        cardinality: Optional,
        item: String,
        example: "2 outlets",
        order: 600,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_POWER_NEEDS) }

pub static FIELD_SEWING_MACHINES: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        sewing_machines,
        name: "sewing_machines",
        display: "Sewing Machines",
        description: "Whether sewing machines are required.",
        aliases: &["sewing"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 700,
        required: false,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_SEWING_MACHINES) }

pub static FIELD_AV_NOTES: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        av_notes,
        name: "av_notes",
        display: "AV Notes",
        description: "Audio/visual setup notes.",
        aliases: &["av"],
        cardinality: Optional,
        item: Text,
        example: "Projector needed",
        order: 800,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_AV_NOTES) }

pub static FIELD_DIFFICULTY: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        difficulty,
        name: "difficulty",
        display: "Difficulty",
        description: "Skill-level indicator (free text).",
        aliases: &[],
        cardinality: Optional,
        item: String,
        example: "Beginner",
        order: 900,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_DIFFICULTY) }

pub static FIELD_PREREQ: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        prereq,
        name: "prereq",
        display: "Prerequisites",
        description: "Comma-separated prerequisite Uniq IDs.",
        aliases: &["prerequisites"],
        cardinality: Optional,
        item: String,
        example: "GP001",
        order: 1000,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_PREREQ) }

pub static FIELD_COST: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        cost,
        name: "cost",
        display: "Cost",
        description: "Raw cost cell value (e.g. \"$35\", \"Free\", \"Kids\").",
        aliases: &[],
        cardinality: Optional,
        item: String,
        example: "$35",
        order: 1100,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_COST) }

pub static FIELD_IS_FREE: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        is_free,
        name: "is_free",
        display: "Is Free",
        description: "Parsed during import: cost is blank, \"Free\", \"$0\", or \"N/A\".",
        aliases: &["free"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 1200,
        required: false,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_IS_FREE) }

pub static FIELD_IS_KIDS: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        is_kids,
        name: "is_kids",
        display: "Is Kids",
        description: "Parsed during import: cost indicates kids-only pricing.",
        aliases: &["kids"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 1300,
        required: false,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_IS_KIDS) }

pub static FIELD_IS_FULL: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        is_full,
        name: "is_full",
        display: "Full",
        description: "Event is at capacity.",
        aliases: &["full"],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 1400,
        required: false,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_IS_FULL) }

pub static FIELD_CAPACITY: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        capacity,
        name: "capacity",
        display: "Capacity",
        description: "Total seats available.",
        aliases: &[],
        cardinality: Optional,
        item: Integer,
        example: "50",
        order: 1500,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_CAPACITY) }

pub static FIELD_SEATS_SOLD: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        seats_sold,
        name: "seats_sold",
        display: "Seats Sold",
        description: "Number of seats pre-sold or reserved via ticketing.",
        aliases: &[],
        cardinality: Optional,
        item: Integer,
        example: "25",
        order: 1600,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_SEATS_SOLD) }

pub static FIELD_PRE_REG_MAX: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        pre_reg_max,
        name: "pre_reg_max",
        display: "Pre-reg Max",
        description: "Maximum seats available for pre-registration.",
        aliases: &["prereg_max"],
        cardinality: Optional,
        item: Integer,
        example: "40",
        order: 1700,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_PRE_REG_MAX) }

pub static FIELD_TICKET_URL: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        ticket_url,
        name: "ticket_url",
        display: "Ticket URL",
        description: "URL for purchasing tickets.",
        aliases: &["ticket_sale"],
        cardinality: Optional,
        item: String,
        example: "https://example.com/ticket",
        order: 1800,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_TICKET_URL) }

pub static FIELD_HAVE_TICKET_IMAGE: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        have_ticket_image,
        name: "have_ticket_image",
        display: "Have Ticket Image",
        description: "Whether a ticket / flyer image has been received.",
        aliases: &[],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 1900,
        required: false,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_HAVE_TICKET_IMAGE) }

pub static FIELD_SIMPLETIX_EVENT: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        simpletix_event,
        name: "simpletix_event",
        display: "SimpleTix Event",
        description: "Internal admin URL for SimpleTix event configuration.",
        aliases: &["simpletix"],
        cardinality: Optional,
        item: String,
        example: "https://admin.simpletix.com/event/123",
        order: 2000,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_SIMPLETIX_EVENT) }

pub static FIELD_SIMPLETIX_LINK: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        simpletix_link,
        name: "simpletix_link",
        display: "SimpleTix Link",
        description: "Public-facing direct ticket purchase link.",
        aliases: &[],
        cardinality: Optional,
        item: String,
        example: "https://simpletix.com/event/123",
        order: 2100,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_SIMPLETIX_LINK) }

pub static FIELD_HIDE_PANELIST: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        hide_panelist,
        name: "hide_panelist",
        display: "Hide Panelist",
        description: "Suppress presenter credits for this panel.",
        aliases: &[],
        cardinality: Single,
        item: Boolean,
        example: "false",
        order: 2200,
        required: false,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_HIDE_PANELIST) }

pub static FIELD_ALT_PANELIST: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = accessor_field_properties! {
        PanelEntityType,
        alt_panelist,
        name: "alt_panelist",
        display: "Alt Panelist",
        description: "Override text for the presenter credits line.",
        aliases: &[],
        cardinality: Optional,
        item: String,
        example: "Special Guest",
        order: 2300,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&FIELD_ALT_PANELIST) }

// ── Computed time projections ─────────────────────────────────────────────────

define_field! {
    /// Start time — projected from `time_slot`.
    static FIELD_START_TIME: FieldDescriptor<PanelEntityType>,
    name: "start_time", display: "Start Time",
    desc: "Panel start time.",
    aliases: &["start"],
    example: "2023-06-25T19:00:00",
    order: 2400,
    crdt: Derived, cardinality: optional, item: FieldTypeItem::DateTime,
    read: |d: &PanelInternalData| {
        d.time_slot.start_time().map(|dt| field_value!(dt))
    },
    write: |d: &mut PanelInternalData, v: FieldValue| {
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
    verify: ReRead
}

define_field! {
    /// End time — projected from `time_slot`.
    static FIELD_END_TIME: FieldDescriptor<PanelEntityType>,
    name: "end_time", display: "End Time",
    desc: "Panel end time.",
    aliases: &["end"],
    example: "2023-06-25T20:30:00",
    order: 2500,
    crdt: Derived, cardinality: optional, item: FieldTypeItem::DateTime,
    read: |d: &PanelInternalData| {
        d.time_slot.end_time().map(|dt| field_value!(dt))
    },
    write: |d: &mut PanelInternalData, v: FieldValue| {
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
    verify: ReRead
}

define_field! {
    /// Duration — projected from `time_slot`.
    static FIELD_DURATION: FieldDescriptor<PanelEntityType>,
    name: "duration", display: "Duration",
    desc: "Panel duration.",
    aliases: &[],
    example: "90",
    order: 2600,
    crdt: Derived, cardinality: optional, item: FieldTypeItem::Duration,
    read: |d: &PanelInternalData| {
        d.time_slot.duration().map(|dur| field_value!(dur))
    },
    write: |d: &mut PanelInternalData, v: FieldValue| {
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
    verify: ReRead
}

// ── Edge-backed computed fields ───────────────────────────────────────────────

// Note: presenter::FIELD_PANELS is a computed field, not an edge field, so it keeps the FIELD_ name.
// TODO: When we migrate computed fields to use HALF_EDGE naming, update this reference.
pub static HALF_EDGE_CREDITED_PRESENTERS: crate::field::FieldDescriptor<PanelEntityType> = {
    let (data, cb, edge_kind) = crate::edge_field_properties! {
        PanelEntityType,
        target: PresenterEntityType,
        target_field: &crate::tables::presenter::FIELD_PANELS,
        exclusive_with: &HALF_EDGE_UNCREDITED_PRESENTERS,
        name: "credited_presenters",
        display: "Credited Presenters",
        description: "Presenters credited on this panel.",
        aliases: &["credited_panelists", "credited_presenter"],
        example: "[presenter_id]",
        order: 2710,
    };
    crate::field::FieldDescriptor {
        data,
        required: false,
        edge_kind,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&HALF_EDGE_CREDITED_PRESENTERS) }

// Temporary alias for migration - remove when edit_integration.rs is updated
#[allow(deprecated)]
pub use HALF_EDGE_CREDITED_PRESENTERS as FIELD_CREDITED_PRESENTERS;

pub static HALF_EDGE_UNCREDITED_PRESENTERS: crate::field::FieldDescriptor<PanelEntityType> = {
    let (data, cb, edge_kind) = crate::edge_field_properties! {
        PanelEntityType,
        target: PresenterEntityType,
        target_field: &crate::tables::presenter::FIELD_PANELS,
        exclusive_with: &HALF_EDGE_CREDITED_PRESENTERS,
        name: "uncredited_presenters",
        display: "Uncredited Presenters",
        description: "Presenters attached but not credited on this panel.",
        aliases: &["uncredited_panelists", "uncredited_presenter"],
        example: "[presenter_id]",
        order: 2720,
    };
    crate::field::FieldDescriptor {
        data,
        required: false,
        edge_kind,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&HALF_EDGE_UNCREDITED_PRESENTERS) }

// Temporary alias for migration - remove when edit_integration.rs is updated
#[allow(deprecated)]
pub use HALF_EDGE_UNCREDITED_PRESENTERS as FIELD_UNCREDITED_PRESENTERS;

define_field! {
    /// All presenters attached to this panel (credited and uncredited).
    ///
    /// Read-only union of credited and uncredited presenter lists.
    static FIELD_PRESENTERS: FieldDescriptor<PanelEntityType>,
    name: "presenters", display: "Presenters",
    desc: "All presenters attached to this panel (credited and uncredited).",
    aliases: &["panelists", "presenter"],
    example: "[]",
    order: 2700,
    crdt: Derived, cardinality: list,
    item: FieldTypeItem::EntityIdentifier(PresenterEntityType::TYPE_NAME),
    read: |sched: &Schedule, id: PanelId| {
        let credited_edge = HALF_EDGE_CREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        let uncredited_edge = HALF_EDGE_UNCREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        crate::schedule::combine_full_edges(sched, id, &[&credited_edge, &uncredited_edge]).ok().flatten()
    }
}

// Note: FIELD_ADD_CREDITED_PRESENTERS and FIELD_ADD_UNCREDITED_PRESENTERS use 'add' edge mode
// which is not yet supported by edge_field_properties!. Leave as define_field! for now.
// TODO: Migrate when edge_field_properties! supports add/remove modes.

define_field! {
    /// Add presenters to this panel and mark them as credited.
    ///
    /// Write-only.  Each presenter in the list is added to the credited list
    /// and removed from the uncredited list (if present).
    static FIELD_ADD_CREDITED_PRESENTERS: FieldDescriptor<PanelEntityType>,
    edge: add, target: PresenterEntityType, target_field: &crate::tables::presenter::FIELD_PANELS,
    exclusive_with: &HALF_EDGE_UNCREDITED_PRESENTERS,
    name: "add_credited_presenters", display: "Add Credited Presenters",
    desc: "Add presenters to this panel and mark them as credited.",
    aliases: &["add_credited_presenter"],
    example: "[presenter_id]",
    order: 2730
}

define_field! {
    /// Add presenters to this panel and mark them as uncredited.
    ///
    /// Write-only.  Each presenter in the list is added to the uncredited list
    /// and removed from the credited list (if present).
    static FIELD_ADD_UNCREDITED_PRESENTERS: FieldDescriptor<PanelEntityType>,
    edge: add, target: PresenterEntityType, target_field: &crate::tables::presenter::FIELD_PANELS,
    exclusive_with: &HALF_EDGE_CREDITED_PRESENTERS,
    name: "add_uncredited_presenters", display: "Add Uncredited Presenters",
    desc: "Add presenters to this panel and mark them as uncredited.",
    aliases: &["add_uncredited_presenter"],
    example: "[presenter_id]",
    order: 2740
}

define_field! {
    /// Remove presenters from this panel.
    ///
    /// Removes each presenter from both credited and uncredited lists.
    static FIELD_REMOVE_PRESENTERS: FieldDescriptor<PanelEntityType>,
    name: "remove_presenters", display: "Remove Presenters",
    desc: "Remove presenters from this panel (both credited and uncredited).",
    aliases: &["remove_presenter"],
    example: "[presenter_id]",
    order: 2900,
    crdt: Derived, cardinality: list,
    item: FieldTypeItem::EntityIdentifier(PresenterEntityType::TYPE_NAME),
    write: |sched: &mut Schedule, panel_id: PanelId, val: FieldValue| {
        let ids = crate::schedule::field_value_to_entity_ids::<PresenterEntityType>(val)?;
        let credited_edge = HALF_EDGE_CREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        let uncredited_edge = HALF_EDGE_UNCREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        for p in ids {
            sched.edge_remove(panel_id, credited_edge, std::iter::once(p));
            sched.edge_remove(panel_id, uncredited_edge, std::iter::once(p));
        }
        Ok(())
    }
}

define_field! {
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
    static FIELD_INCLUSIVE_PRESENTERS: FieldDescriptor<PanelEntityType>,
    name: "inclusive_presenters", display: "Inclusive Presenters",
    desc: "Direct presenters + their transitive groups + their transitive members.",
    aliases: &["inclusive_presenter"],
    example: "[]",
    order: 3000,
    crdt: Derived, cardinality: list,
    item: FieldTypeItem::EntityIdentifier(PresenterEntityType::TYPE_NAME),
    read: |sched: &Schedule, panel_id: PanelId| {
        let edge = FIELD_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        let direct = sched
            .connected_field_nodes(panel_id, edge)
            .into_iter()
            .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
            .collect::<Vec<PresenterId>>();
        let mut result: HashSet<PresenterId> = HashSet::new();
        for p in direct {
            result.insert(p);
            // Inclusive members of p: all members of p (following EDGE_MEMBERS from p)
            for m in sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(
                p,
                EDGE_MEMBERS,
            ) {
                result.insert(m);
            }
            // Inclusive groups of p: all groups p belongs to (following EDGE_GROUPS from p)
            for g in sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(
                p,
                EDGE_GROUPS,
            ) {
                result.insert(g);
            }
        }
        let ids: Vec<PresenterId> = result.into_iter().collect();
        Some(crate::schedule::entity_ids_to_field_value(ids))
    }
}

pub static HALF_EDGE_EVENT_ROOMS: crate::field::FieldDescriptor<PanelEntityType> = {
    let (data, cb, edge_kind) = crate::edge_field_properties! {
        PanelEntityType,
        target: EventRoomEntityType,
        target_field: &crate::tables::event_room::HALF_EDGE_PANELS,
        name: "event_rooms",
        display: "Event Rooms",
        description: "Rooms where this panel takes place.",
        aliases: &["rooms", "room", "event_room"],
        example: "[]",
        order: 3100,
    };
    crate::field::FieldDescriptor {
        data,
        required: false,
        edge_kind,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&HALF_EDGE_EVENT_ROOMS) }

// Temporary alias for migration
#[allow(deprecated)]
pub use HALF_EDGE_EVENT_ROOMS as FIELD_EVENT_ROOMS;

// Note: FIELD_ADD_ROOMS and FIELD_REMOVE_ROOMS use 'add'/'remove' edge modes
// which are not yet supported by edge_field_properties!. Leave as define_field! for now.
// TODO: Migrate when edge_field_properties! supports add/remove modes.

define_field! {
    static FIELD_ADD_ROOMS: FieldDescriptor<PanelEntityType>,
    edge: add, target: EventRoomEntityType, target_field: &crate::tables::event_room::HALF_EDGE_PANELS,
    name: "add_rooms", display: "Add Rooms",
    desc: "Append event rooms to this panel.",
    aliases: &["add_room"],
    example: "[room_id]",
    order: 3200
}

define_field! {
    static FIELD_REMOVE_ROOMS: FieldDescriptor<PanelEntityType>,
    edge: remove, target: EventRoomEntityType, target_field: &crate::tables::event_room::HALF_EDGE_PANELS,
    name: "remove_rooms", display: "Remove Rooms",
    desc: "Remove event rooms from this panel.",
    aliases: &["remove_room"],
    example: "[room_id]",
    order: 3300
}

pub static HALF_EDGE_PANEL_TYPE: crate::field::FieldDescriptor<PanelEntityType> = {
    let (data, cb, edge_kind) = crate::edge_field_properties! {
        PanelEntityType,
        target: PanelTypeEntityType,
        target_field: &crate::tables::panel_type::HALF_EDGE_PANELS,
        name: "panel_type",
        display: "Panel Type",
        description: "Panel type / kind.",
        aliases: &["kind", "type"],
        example: "{}",
        order: 3400,
    };
    crate::field::FieldDescriptor {
        data,
        required: false,
        edge_kind,
        cb,
    }
};
inventory::submit! { CollectedNamedField(&HALF_EDGE_PANEL_TYPE) }

// Temporary alias for migration - remove when edit_integration.rs is updated
#[allow(deprecated)]
pub use HALF_EDGE_PANEL_TYPE as FIELD_PANEL_TYPE;

/// Full edge from panel credited presenters to presenter panels
pub const EDGE_CREDITED_PRESENTERS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_CREDITED_PRESENTERS,
    far: &crate::tables::presenter::FIELD_PANELS,
};

/// Full edge from panel uncredited presenters to presenter panels
pub const EDGE_UNCREDITED_PRESENTERS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_UNCREDITED_PRESENTERS,
    far: &crate::tables::presenter::FIELD_PANELS,
};

/// Full edge from panel event rooms to event room panels
pub const EDGE_EVENT_ROOMS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_EVENT_ROOMS,
    far: &crate::tables::event_room::HALF_EDGE_PANELS,
};

/// Full edge from panel panel type to panel type panels
pub const EDGE_PANEL_TYPE: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_PANEL_TYPE,
    far: &crate::tables::panel_type::HALF_EDGE_PANELS,
};

// ── Read-only computed fields ─────────────────────────────────────────────────────

define_field! {
    /// Hotel rooms for this panel (traverses event_rooms => hotel room edges).
    static FIELD_HOTEL_ROOMS: FieldDescriptor<PanelEntityType>,
    name: "hotel_rooms", display: "Hotel Rooms",
    desc: "Hotel rooms where this panel takes place (traverses event rooms).",
    aliases: &["hotel_room"],
    example: "[]",
    order: 3500,
    crdt: Derived, cardinality: list,
    item: FieldTypeItem::EntityIdentifier(HotelRoomEntityType::TYPE_NAME),
    read: |sched: &Schedule, id: PanelId| {
        let event_edge = HALF_EDGE_EVENT_ROOMS.edge_to(&crate::tables::event_room::HALF_EDGE_PANELS);
        let event_room_ids = sched
            .connected_field_nodes(id, event_edge)
            .into_iter()
            .map(|e| unsafe { EventRoomId::new_unchecked(e.entity_uuid()) })
            .collect::<Vec<EventRoomId>>();
        let mut hotel_room_ids: HashSet<HotelRoomId> = HashSet::new();
        for event_room_id in event_room_ids {
            let hotel_edge = crate::tables::event_room::HALF_EDGE_HOTEL_ROOMS.edge_to(&crate::tables::hotel_room::HALF_EDGE_EVENT_ROOMS);
            let rooms = sched
                .connected_field_nodes(event_room_id, hotel_edge)
                .into_iter()
                .map(|e| unsafe { HotelRoomId::new_unchecked(e.entity_uuid()) })
                .collect::<Vec<HotelRoomId>>();
            hotel_room_ids.extend(rooms);
        }
        let hotel_room_ids: Vec<HotelRoomId> = hotel_room_ids.into_iter().collect();
        Some(crate::schedule::entity_ids_to_field_value(hotel_room_ids))
    }
}

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

    // Get credited presenters directly from the credited edge list
    let credited_edge =
        HALF_EDGE_CREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
    let credited_ids = sched
        .connected_field_nodes(panel_id, credited_edge)
        .into_iter()
        .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
        .collect::<Vec<PresenterId>>();
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
            let member_ids = sched
                .connected_field_nodes(presenter_id, EDGE_MEMBERS)
                .into_iter()
                .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
                .collect::<Vec<PresenterId>>();
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
            // First: find which groups this presenter belongs to
            let group_ids = sched
                .connected_field_nodes(presenter_id, EDGE_GROUPS)
                .into_iter()
                .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
                .collect::<Vec<PresenterId>>();
            for group_id in group_ids {
                let Some(&group_data) = presenter_lookup.get(&group_id) else {
                    continue;
                };
                // Then: find all members of that group
                let group_member_ids = sched
                    .connected_field_nodes(group_id, EDGE_MEMBERS)
                    .into_iter()
                    .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
                    .collect::<Vec<PresenterId>>();
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

define_field! {
    /// Formatted credit strings for display (hidePanelist, altPanelist, group resolution).
    static FIELD_CREDITS: FieldDescriptor<PanelEntityType>,
    name: "credits", display: "Credits",
    desc: "Formatted presenter credit strings for display, accounting for hidePanelist, altPanelist, group resolution, always_shown, and always_grouped flags.",
    aliases: &["credit"],
    example: "[\"John Doe\", \"Group Name (Alice, Bob)\"]",
    order: 3600,
    crdt: Derived, cardinality: list, item: FieldTypeItem::String,
    read: |sched: &Schedule, id: PanelId| {
        let strings = compute_credits(sched, id);
        Some(FieldValue::List(
            strings.into_iter().map(FieldValueItem::String).collect(),
        ))
    }
}

// ── FieldSet ──────────────────────────────────────────────────────────────────

static PANEL_FIELD_SET: LazyLock<FieldSet<PanelEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ... (rest of the code remains the same)
// ── Builder ───────────────────────────────────────────────────────────────────

crate::field::macros::define_entity_builder! {
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
        with_panel_type          => HALF_EDGE_PANEL_TYPE,
    }
}

// ── EntityMatcher ─────────────────────────────────────────────────────────────

impl crate::query::lookup::EntityScannable for PanelEntityType {}

impl crate::query::lookup::EntityMatcher for PanelEntityType {
    fn match_entity(
        query: &str,
        data: &PanelInternalData,
    ) -> Option<crate::query::lookup::MatchPriority> {
        use crate::query::lookup::string_match_priority;
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
        let names: Vec<_> = fs.required_fields().map(|d| d.name()).collect();
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
        fs.write_field_value("description", id, &mut s, crate::field_text!("updated bio"))
            .unwrap();
        assert_eq!(
            fs.read_field_value("description", id, &s).unwrap(),
            Some(crate::field_text!("updated bio"))
        );
    }

    #[test]
    fn write_optional_string_to_none_clears() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("cost", id, &mut s, crate::field_empty_list!())
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
            Some(crate::field_empty_list!())
        );
        assert_eq!(
            fs.read_field_value("rooms", id, &s).unwrap(),
            Some(crate::field_empty_list!())
        );
        assert_eq!(
            fs.read_field_value("panel_type", id, &s).unwrap(),
            Some(crate::field_empty_list!())
        );
        assert_eq!(
            fs.read_field_value("inclusive_presenters", id, &s).unwrap(),
            Some(crate::field_empty_list!())
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
            crate::field_empty_list!(),
        )
        .unwrap();
        fs.write_field_value(
            "add_uncredited_presenters",
            id,
            &mut s,
            crate::field_empty_list!(),
        )
        .unwrap();
    }

    #[test]
    fn write_inclusive_presenters_is_read_only() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        let r = fs.write_field_value(
            "inclusive_presenters",
            id,
            &mut s,
            crate::field_empty_list!(),
        );
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
        use crate::query::converter::EntityStringResolver;
        let id = new_panel_id();
        let mut sched = Schedule::default();
        sched.insert(id, sample_internal(id));
        let s = PanelEntityType::entity_to_string(&sched, id);
        assert_eq!(s, "GP001: Panel Name");
    }

    #[test]
    fn test_entity_to_string_fallback_to_uuid() {
        use crate::query::converter::EntityStringResolver;
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
        let data = crate::tables::presenter::PresenterInternalData {
            id: pres_id,
            data: crate::tables::presenter::PresenterCommonData {
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

        let credited_edge =
            HALF_EDGE_CREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        sched
            .edge_add(panel_id, credited_edge, std::iter::once(alice))
            .expect("edge type validation failed");
        sched
            .edge_add(panel_id, credited_edge, std::iter::once(bob))
            .expect("edge type validation failed");
        PanelEntityType::field_set()
            .write_field_value(
                "add_uncredited_presenters",
                panel_id,
                &mut sched,
                crate::schedule::entity_ids_to_field_value(vec![bob]),
            )
            .expect("write should succeed");

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

        let credited_edge =
            HALF_EDGE_CREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        sched
            .edge_add(panel_id, credited_edge, std::iter::once(alice))
            .expect("edge type validation failed: credited presenter");
        sched
            .edge_add(panel_id, credited_edge, std::iter::once(bob))
            .expect("edge type validation failed: credited presenter");
        PanelEntityType::field_set()
            .write_field_value(
                "add_uncredited_presenters",
                panel_id,
                &mut sched,
                crate::schedule::entity_ids_to_field_value(vec![bob]),
            )
            .expect("write should succeed");

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
    fn credited_presenters_write_removes_absent() {
        let panel_id = new_panel_id();
        let mut sched = sched_with(panel_id, sample_internal(panel_id));

        let alice = make_presenter_for_panel(&mut sched, "Alice"); // credited
        let bob = make_presenter_for_panel(&mut sched, "Bob"); // credited, will be dropped

        let credited_edge =
            HALF_EDGE_CREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        sched
            .edge_add(panel_id, credited_edge, std::iter::once(alice))
            .expect("edge type validation failed");
        sched
            .edge_add(panel_id, credited_edge, std::iter::once(bob))
            .expect("edge type validation failed");

        // Write credited_presenters = [alice] — bob should be removed.
        sched
            .edge_set(panel_id, credited_edge, vec![alice])
            .expect("edge type validation failed");

        // Bob's edge should be gone.
        let all = sched
            .connected_field_nodes(panel_id, credited_edge)
            .into_iter()
            .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
            .collect::<Vec<PresenterId>>();
        assert!(!all.contains(&bob), "Bob should have been removed");

        // Alice still present and credited.
        assert!(all.contains(&alice), "Alice should still be present");
        let credited_val = PanelEntityType::field_set()
            .read_field_value("credited_presenters", panel_id, &sched)
            .unwrap()
            .unwrap();
        let credited_ids = match credited_val {
            crate::value::FieldValue::List(items) => items,
            _ => panic!("expected List"),
        };
        assert!(
            credited_ids.iter().any(|item| {
                matches!(item, crate::value::FieldValueItem::EntityIdentifier(id) if id.entity_uuid() == alice.entity_uuid())
            }),
            "Alice should be credited"
        );
    }

    /// Writing `credited_presenters` must only touch the credited edge list —
    /// presenters in the uncredited partition for the same panel must be
    /// preserved.
    #[test]
    fn credited_presenters_write_leaves_uncredited_partition() {
        let panel_id = new_panel_id();
        let mut sched = sched_with(panel_id, sample_internal(panel_id));

        let alice = make_presenter_for_panel(&mut sched, "Alice"); // credited
        let bob = make_presenter_for_panel(&mut sched, "Bob"); // credited, will be dropped
        let carol = make_presenter_for_panel(&mut sched, "Carol"); // uncredited — must be untouched

        let credited_edge =
            HALF_EDGE_CREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        let uncredited_edge =
            HALF_EDGE_UNCREDITED_PRESENTERS.edge_to(&crate::tables::presenter::FIELD_PANELS);
        sched
            .edge_add(panel_id, credited_edge, std::iter::once(alice))
            .expect("edge type validation failed");
        sched
            .edge_add(panel_id, credited_edge, std::iter::once(bob))
            .expect("edge type validation failed");
        sched
            .edge_add(panel_id, uncredited_edge, std::iter::once(carol))
            .expect("edge type validation failed");

        // Replace credited_presenters with [alice] — bob removed, carol untouched.
        sched
            .edge_set(panel_id, credited_edge, vec![alice])
            .expect("edge type validation failed");

        // Carol should still be in the uncredited list.
        let uncredited = sched
            .connected_field_nodes(panel_id, uncredited_edge)
            .into_iter()
            .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
            .collect::<Vec<PresenterId>>();
        assert!(
            uncredited.contains(&carol),
            "Carol (uncredited) should be untouched"
        );

        // Carol should not have leaked into credited.
        let credited = sched
            .connected_field_nodes(panel_id, credited_edge)
            .into_iter()
            .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
            .collect::<Vec<PresenterId>>();
        assert!(
            !credited.contains(&carol),
            "Carol should not be in credited list"
        );
    }
}
