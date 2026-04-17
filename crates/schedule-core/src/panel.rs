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
use crate::field::{FieldDescriptor, MatchPriority, ReadFn, WriteFn};
use crate::panel_type::PanelTypeId;
use crate::panel_uniq_id::PanelUniqId;
use crate::time::{parse_datetime, parse_duration, TimeRange};
use crate::value::{CrdtFieldType, FieldValue, ValidationError};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── Type aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for Panel entities.
pub type PanelId = EntityId<PanelEntityType>;

// Placeholder edge target ID types — full implementations land with their
// owning entities in FEATURE-016. Declared here as zero-sized placeholders so
// [`PanelData`] has stable field types for serialization round-trip tests.

/// Placeholder PresenterId until FEATURE-016 introduces the Presenter entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PresenterIdPlaceholder;

/// Placeholder EventRoomId until FEATURE-016 introduces the EventRoom entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct EventRoomIdPlaceholder;

// ── PanelCommonData ───────────────────────────────────────────────────────────

/// User-facing fields from the Schedule sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelCommonData {
    pub uid: String,
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
        let mut errors = Vec::new();
        if self.uid.is_empty() {
            errors.push(ValidationError::Required { field: "uid" });
        }
        if self.name.is_empty() {
            errors.push(ValidationError::Required { field: "name" });
        }
        errors
    }
}

// ── PanelInternalData ─────────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
#[derive(Debug, Clone)]
pub struct PanelInternalData {
    pub data: PanelCommonData,
    pub code: PanelId,
    pub time_slot: TimeRange,
    pub parsed_uid: Option<PanelUniqId>,
}

// ── PanelData ─────────────────────────────────────────────────────────────────

/// Export/API view produced by [`PanelEntityType::export`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelData {
    #[serde(flatten)]
    pub data: PanelCommonData,
    pub code: String,
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
    pub presenter_ids: Vec<PresenterIdPlaceholder>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub event_room_ids: Vec<EventRoomIdPlaceholder>,
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
            data: internal.data.clone(),
            code: internal.code.to_string(),
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

// ── Field-descriptor helpers ──────────────────────────────────────────────────

fn substring_match(query: &str, value: &str) -> Option<MatchPriority> {
    let q = query.to_lowercase();
    let v = value.to_lowercase();
    if v == q {
        Some(MatchPriority::Exact)
    } else if v.starts_with(&q) {
        Some(MatchPriority::Prefix)
    } else if v.contains(&q) {
        Some(MatchPriority::Contains)
    } else {
        None
    }
}

// Required indexed String field.
macro_rules! req_string_field {
    (
        $static_name:ident, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr
    ) => {
        static $static_name: FieldDescriptor<PanelEntityType> = FieldDescriptor {
            name: $name,
            display: $display,
            description: $desc,
            aliases: $aliases,
            required: true,
            crdt_type: CrdtFieldType::Scalar,
            read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
                Some(FieldValue::String(d.data.$field.clone()))
            })),
            write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
                d.data.$field = v.into_string()?;
                Ok(())
            })),
            index_fn: Some(|query, d: &PanelInternalData| substring_match(query, &d.data.$field)),
            verify_fn: None,
        };
    };
}

// Optional short String field (CRDT::Scalar). `FieldValue::String` variant.
macro_rules! opt_string_field {
    (
        $static_name:ident, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr
    ) => {
        static $static_name: FieldDescriptor<PanelEntityType> = FieldDescriptor {
            name: $name,
            display: $display,
            description: $desc,
            aliases: $aliases,
            required: false,
            crdt_type: CrdtFieldType::Scalar,
            read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
                Some(match &d.data.$field {
                    Some(s) => FieldValue::String(s.clone()),
                    None => FieldValue::None,
                })
            })),
            write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
                if v.is_none() {
                    d.data.$field = None;
                } else {
                    d.data.$field = Some(v.into_string()?);
                }
                Ok(())
            })),
            index_fn: None,
            verify_fn: None,
        };
    };
}

// Optional prose field stored as `Option<String>` but CRDT::Text; read/write
// via `FieldValue::Text` variant.
macro_rules! opt_text_field {
    (
        $static_name:ident, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr
    ) => {
        static $static_name: FieldDescriptor<PanelEntityType> = FieldDescriptor {
            name: $name,
            display: $display,
            description: $desc,
            aliases: $aliases,
            required: false,
            crdt_type: CrdtFieldType::Text,
            read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
                Some(match &d.data.$field {
                    Some(s) => FieldValue::Text(s.clone()),
                    None => FieldValue::None,
                })
            })),
            write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
                if v.is_none() {
                    d.data.$field = None;
                } else {
                    d.data.$field = Some(v.into_text()?);
                }
                Ok(())
            })),
            index_fn: None,
            verify_fn: None,
        };
    };
}

// Boolean field.
macro_rules! bool_field {
    (
        $static_name:ident, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr
    ) => {
        static $static_name: FieldDescriptor<PanelEntityType> = FieldDescriptor {
            name: $name,
            display: $display,
            description: $desc,
            aliases: $aliases,
            required: false,
            crdt_type: CrdtFieldType::Scalar,
            read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
                Some(FieldValue::Boolean(d.data.$field))
            })),
            write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
                d.data.$field = v.into_bool()?;
                Ok(())
            })),
            index_fn: None,
            verify_fn: None,
        };
    };
}

// Optional `i64` field.
macro_rules! opt_i64_field {
    (
        $static_name:ident, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr
    ) => {
        static $static_name: FieldDescriptor<PanelEntityType> = FieldDescriptor {
            name: $name,
            display: $display,
            description: $desc,
            aliases: $aliases,
            required: false,
            crdt_type: CrdtFieldType::Scalar,
            read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
                Some(match d.data.$field {
                    Some(n) => FieldValue::Integer(n),
                    None => FieldValue::None,
                })
            })),
            write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
                if v.is_none() {
                    d.data.$field = None;
                } else {
                    d.data.$field = Some(v.into_integer()?);
                }
                Ok(())
            })),
            index_fn: None,
            verify_fn: None,
        };
    };
}

// ── Stored field descriptors ──────────────────────────────────────────────────

req_string_field!(FIELD_UID, uid,
    name: "uid", display: "Uniq ID",
    desc: "Panel identifier from the spreadsheet.",
    aliases: &["id", "uniq_id"]);

req_string_field!(FIELD_NAME, name,
    name: "name", display: "Name",
    desc: "Panel name / title.",
    aliases: &["title", "panel_name"]);

opt_text_field!(FIELD_DESCRIPTION, description,
    name: "description", display: "Description",
    desc: "Event description shown to attendees.",
    aliases: &["desc"]);

opt_text_field!(FIELD_NOTE, note,
    name: "note", display: "Note",
    desc: "Extra note displayed verbatim.",
    aliases: &[]);

opt_text_field!(FIELD_NOTES_NON_PRINTING, notes_non_printing,
    name: "notes_non_printing", display: "Notes (Non Printing)",
    desc: "Internal notes not shown to the public.",
    aliases: &["internal_notes"]);

opt_text_field!(FIELD_WORKSHOP_NOTES, workshop_notes,
    name: "workshop_notes", display: "Workshop Notes",
    desc: "Notes for workshop staff.",
    aliases: &[]);

opt_string_field!(FIELD_POWER_NEEDS, power_needs,
    name: "power_needs", display: "Power Needs",
    desc: "Power / electrical requirements.",
    aliases: &["power"]);

bool_field!(FIELD_SEWING_MACHINES, sewing_machines,
    name: "sewing_machines", display: "Sewing Machines",
    desc: "Whether sewing machines are required.",
    aliases: &["sewing"]);

opt_text_field!(FIELD_AV_NOTES, av_notes,
    name: "av_notes", display: "AV Notes",
    desc: "Audio/visual setup notes.",
    aliases: &["av"]);

opt_string_field!(FIELD_DIFFICULTY, difficulty,
    name: "difficulty", display: "Difficulty",
    desc: "Skill-level indicator (free text).",
    aliases: &[]);

opt_string_field!(FIELD_PREREQ, prereq,
    name: "prereq", display: "Prerequisites",
    desc: "Comma-separated prerequisite Uniq IDs.",
    aliases: &["prerequisites"]);

opt_string_field!(FIELD_COST, cost,
    name: "cost", display: "Cost",
    desc: "Raw cost cell value (e.g. \"$35\", \"Free\", \"Kids\").",
    aliases: &[]);

bool_field!(FIELD_IS_FREE, is_free,
    name: "is_free", display: "Is Free",
    desc: "Parsed during import: cost is blank, \"Free\", \"$0\", or \"N/A\".",
    aliases: &["free"]);

bool_field!(FIELD_IS_KIDS, is_kids,
    name: "is_kids", display: "Is Kids",
    desc: "Parsed during import: cost indicates kids-only pricing.",
    aliases: &["kids"]);

bool_field!(FIELD_IS_FULL, is_full,
    name: "is_full", display: "Full",
    desc: "Event is at capacity.",
    aliases: &["full"]);

opt_i64_field!(FIELD_CAPACITY, capacity,
    name: "capacity", display: "Capacity",
    desc: "Total seats available.",
    aliases: &[]);

opt_i64_field!(FIELD_SEATS_SOLD, seats_sold,
    name: "seats_sold", display: "Seats Sold",
    desc: "Number of seats pre-sold or reserved via ticketing.",
    aliases: &[]);

opt_i64_field!(FIELD_PRE_REG_MAX, pre_reg_max,
    name: "pre_reg_max", display: "Pre-reg Max",
    desc: "Maximum seats available for pre-registration.",
    aliases: &["prereg_max"]);

opt_string_field!(FIELD_TICKET_URL, ticket_url,
    name: "ticket_url", display: "Ticket URL",
    desc: "URL for purchasing tickets.",
    aliases: &["ticket_sale"]);

bool_field!(FIELD_HAVE_TICKET_IMAGE, have_ticket_image,
    name: "have_ticket_image", display: "Have Ticket Image",
    desc: "Whether a ticket / flyer image has been received.",
    aliases: &[]);

opt_string_field!(FIELD_SIMPLETIX_EVENT, simpletix_event,
    name: "simpletix_event", display: "SimpleTix Event",
    desc: "Internal admin URL for SimpleTix event configuration.",
    aliases: &["simpletix"]);

opt_string_field!(FIELD_SIMPLETIX_LINK, simpletix_link,
    name: "simpletix_link", display: "SimpleTix Link",
    desc: "Public-facing direct ticket purchase link.",
    aliases: &[]);

bool_field!(FIELD_HIDE_PANELIST, hide_panelist,
    name: "hide_panelist", display: "Hide Panelist",
    desc: "Suppress presenter credits for this panel.",
    aliases: &[]);

opt_string_field!(FIELD_ALT_PANELIST, alt_panelist,
    name: "alt_panelist", display: "Alt Panelist",
    desc: "Override text for the presenter credits line.",
    aliases: &[]);

// ── Computed time projections ─────────────────────────────────────────────────

/// Start time — projected from `time_slot`.
static FIELD_START_TIME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "start_time",
    display: "Start Time",
    description: "Panel start time.",
    aliases: &["start"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
        Some(match d.time_slot.start_time() {
            Some(dt) => FieldValue::DateTime(dt),
            None => FieldValue::None,
        })
    })),
    write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
        match v {
            FieldValue::None => d.time_slot.remove_start_time(),
            FieldValue::DateTime(dt) => d.time_slot.add_start_time(dt),
            FieldValue::String(s) => match parse_datetime(&s) {
                Some(dt) => d.time_slot.add_start_time(dt),
                None => {
                    return Err(crate::value::ConversionError::ParseError {
                        message: format!("could not parse datetime {s:?}"),
                    }
                    .into())
                }
            },
            other => {
                return Err(crate::value::ConversionError::WrongVariant {
                    expected: "DateTime or String",
                    got: match other {
                        FieldValue::Integer(_) => "Integer",
                        FieldValue::Boolean(_) => "Boolean",
                        FieldValue::Duration(_) => "Duration",
                        _ => "other",
                    },
                }
                .into());
            }
        }
        Ok(())
    })),
    index_fn: None,
    verify_fn: None,
};

/// End time — projected from `time_slot`.
static FIELD_END_TIME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "end_time",
    display: "End Time",
    description: "Panel end time.",
    aliases: &["end"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
        Some(match d.time_slot.end_time() {
            Some(dt) => FieldValue::DateTime(dt),
            None => FieldValue::None,
        })
    })),
    write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
        match v {
            FieldValue::None => d.time_slot.remove_end_time(),
            FieldValue::DateTime(dt) => d.time_slot.add_end_time(dt),
            FieldValue::String(s) => match parse_datetime(&s) {
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
    })),
    index_fn: None,
    verify_fn: None,
};

/// Duration — projected from `time_slot`.
static FIELD_DURATION: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "duration",
    display: "Duration",
    description: "Panel duration.",
    aliases: &[],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: Some(ReadFn::Bare(|d: &PanelInternalData| {
        Some(match d.time_slot.duration() {
            Some(dur) => FieldValue::Duration(dur),
            None => FieldValue::None,
        })
    })),
    write_fn: Some(WriteFn::Bare(|d: &mut PanelInternalData, v| {
        match v {
            FieldValue::None => d.time_slot.remove_duration(),
            FieldValue::Duration(dur) => d.time_slot.add_duration(dur),
            FieldValue::Integer(m) => d.time_slot.add_duration(Duration::minutes(m)),
            FieldValue::String(s) => match parse_duration(&s) {
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
    })),
    index_fn: None,
    verify_fn: None,
};

// ── Edge-backed computed field stubs (full wiring in FEATURE-018) ─────────────
//
// Reads return `FieldValue::List(Vec::new())` (or `FieldValue::None` for the
// singular `panel_type` field) until edge storage lands. Writes accept any
// value and are no-ops so the field-set exposes a consistent read/write API.

fn edge_read_empty_list(_d: &PanelInternalData) -> Option<FieldValue> {
    Some(FieldValue::List(Vec::new()))
}

fn edge_read_none(_d: &PanelInternalData) -> Option<FieldValue> {
    Some(FieldValue::None)
}

fn edge_write_noop(
    _d: &mut PanelInternalData,
    _v: FieldValue,
) -> Result<(), crate::value::FieldError> {
    Ok(())
}

static FIELD_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "presenters",
    display: "Presenters",
    description: "All presenters credited for this panel.",
    aliases: &["panelists", "presenter"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: Some(ReadFn::Bare(edge_read_empty_list)),
    write_fn: Some(WriteFn::Bare(edge_write_noop)),
    index_fn: None,
    verify_fn: None,
};

static FIELD_ADD_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "add_presenters",
    display: "Add Presenters",
    description: "Append presenters to this panel.",
    aliases: &["add_presenter"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: None,
    write_fn: Some(WriteFn::Bare(edge_write_noop)),
    index_fn: None,
    verify_fn: None,
};

static FIELD_REMOVE_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "remove_presenters",
    display: "Remove Presenters",
    description: "Remove presenters from this panel.",
    aliases: &["remove_presenter"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: None,
    write_fn: Some(WriteFn::Bare(edge_write_noop)),
    index_fn: None,
    verify_fn: None,
};

static FIELD_INCLUSIVE_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "inclusive_presenters",
    display: "Inclusive Presenters",
    description: "Transitive closure: direct presenters + their groups + group members.",
    aliases: &["inclusive_presenter"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: Some(ReadFn::Bare(edge_read_empty_list)),
    write_fn: None,
    index_fn: None,
    verify_fn: None,
};

static FIELD_EVENT_ROOMS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "event_rooms",
    display: "Event Rooms",
    description: "Rooms where this panel takes place.",
    aliases: &["rooms", "room", "event_room"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: Some(ReadFn::Bare(edge_read_empty_list)),
    write_fn: Some(WriteFn::Bare(edge_write_noop)),
    index_fn: None,
    verify_fn: None,
};

static FIELD_ADD_ROOMS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "add_rooms",
    display: "Add Rooms",
    description: "Append event rooms to this panel.",
    aliases: &["add_room"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: None,
    write_fn: Some(WriteFn::Bare(edge_write_noop)),
    index_fn: None,
    verify_fn: None,
};

static FIELD_REMOVE_ROOMS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "remove_rooms",
    display: "Remove Rooms",
    description: "Remove event rooms from this panel.",
    aliases: &["remove_room"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: None,
    write_fn: Some(WriteFn::Bare(edge_write_noop)),
    index_fn: None,
    verify_fn: None,
};

static FIELD_PANEL_TYPE: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "panel_type",
    display: "Panel Type",
    description: "Panel type / kind.",
    aliases: &["kind", "type"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: Some(ReadFn::Bare(edge_read_none)),
    write_fn: Some(WriteFn::Bare(edge_write_noop)),
    index_fn: None,
    verify_fn: None,
};

// ── FieldSet ──────────────────────────────────────────────────────────────────

static PANEL_FIELD_SET: LazyLock<FieldSet<PanelEntityType>> = LazyLock::new(|| {
    FieldSet::new(&[
        // stored
        &FIELD_UID,
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
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn new_panel_id() -> PanelId {
        PanelId::new(Uuid::new_v4()).expect("v4 is never nil")
    }

    fn sample_common() -> PanelCommonData {
        PanelCommonData {
            uid: "GP001".into(),
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
            data: sample_common(),
            code: id,
            time_slot: TimeRange::ScheduledWithDuration {
                start_time: NaiveDate::from_ymd_opt(2026, 6, 26)
                    .unwrap()
                    .and_hms_opt(14, 0, 0)
                    .unwrap(),
                duration: Duration::minutes(60),
            },
            parsed_uid: PanelUniqId::parse("GP001"),
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
    fn required_fields_are_uid_and_name() {
        let fs = PanelEntityType::field_set();
        let names: Vec<_> = fs.required_fields().map(|d| d.name).collect();
        assert_eq!(names, vec!["uid", "name"]);
    }

    // ── Stored field read/write ──────────────────────────────────────────

    #[test]
    fn read_uid_and_name() {
        let id = new_panel_id();
        let s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        assert_eq!(
            fs.read_field_value("uid", id, &s).unwrap(),
            Some(FieldValue::String("GP001".into()))
        );
        assert_eq!(
            fs.read_field_value("title", id, &s).unwrap(),
            Some(FieldValue::String("Panel Name".into()))
        );
    }

    #[test]
    fn write_uid() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("uid", id, &mut s, FieldValue::String("GW007".into()))
            .unwrap();
        assert_eq!(
            fs.read_field_value("uid", id, &s).unwrap(),
            Some(FieldValue::String("GW007".into()))
        );
    }

    #[test]
    fn write_description_uses_text_variant() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value(
            "description",
            id,
            &mut s,
            FieldValue::Text("updated bio".into()),
        )
        .unwrap();
        assert_eq!(
            fs.read_field_value("description", id, &s).unwrap(),
            Some(FieldValue::Text("updated bio".into()))
        );
    }

    #[test]
    fn write_optional_string_to_none_clears() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("cost", id, &mut s, FieldValue::None)
            .unwrap();
        assert_eq!(
            fs.read_field_value("cost", id, &s).unwrap(),
            Some(FieldValue::None)
        );
    }

    #[test]
    fn write_bool_and_i64() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("is_free", id, &mut s, FieldValue::Boolean(true))
            .unwrap();
        fs.write_field_value("capacity", id, &mut s, FieldValue::Integer(99))
            .unwrap();
        assert_eq!(
            fs.read_field_value("is_free", id, &s).unwrap(),
            Some(FieldValue::Boolean(true))
        );
        assert_eq!(
            fs.read_field_value("capacity", id, &s).unwrap(),
            Some(FieldValue::Integer(99))
        );
    }

    #[test]
    fn write_wrong_variant_is_error() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        let r = fs.write_field_value("uid", id, &mut s, FieldValue::Integer(1));
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
            Some(FieldValue::DateTime(_))
        ));
        assert!(matches!(
            fs.read_field_value("end_time", id, &s).unwrap(),
            Some(FieldValue::DateTime(_))
        ));
        assert_eq!(
            fs.read_field_value("duration", id, &s).unwrap(),
            Some(FieldValue::Duration(Duration::minutes(60)))
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
            FieldValue::Duration(Duration::minutes(90)),
        )
        .unwrap();
        assert_eq!(
            fs.read_field_value("duration", id, &s).unwrap(),
            Some(FieldValue::Duration(Duration::minutes(90)))
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
            FieldValue::String("2026-06-26T15:00:00".into()),
        )
        .unwrap();
        let expected = NaiveDate::from_ymd_opt(2026, 6, 26)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap();
        assert_eq!(
            fs.read_field_value("start_time", id, &s).unwrap(),
            Some(FieldValue::DateTime(expected))
        );
    }

    #[test]
    fn write_duration_from_integer_minutes() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        fs.write_field_value("duration", id, &mut s, FieldValue::Integer(120))
            .unwrap();
        assert_eq!(
            fs.read_field_value("duration", id, &s).unwrap(),
            Some(FieldValue::Duration(Duration::minutes(120)))
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
            Some(FieldValue::List(Vec::new()))
        );
        assert_eq!(
            fs.read_field_value("rooms", id, &s).unwrap(),
            Some(FieldValue::List(Vec::new()))
        );
        assert_eq!(
            fs.read_field_value("panel_type", id, &s).unwrap(),
            Some(FieldValue::None)
        );
    }

    #[test]
    fn write_edge_stub_is_noop() {
        let id = new_panel_id();
        let mut s = sched_with(id, sample_internal(id));
        let fs = PanelEntityType::field_set();
        // Should not error even though backing storage is not wired up.
        fs.write_field_value("add_presenters", id, &mut s, FieldValue::List(Vec::new()))
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
            FieldValue::List(Vec::new()),
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
    fn validate_detects_missing_uid_and_name() {
        let id = new_panel_id();
        let internal = PanelInternalData {
            data: PanelCommonData::default(),
            code: id,
            time_slot: TimeRange::Unspecified,
            parsed_uid: None,
        };
        let errors = PanelEntityType::validate(&internal);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::Required { field } if *field == "uid")));
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::Required { field } if *field == "name")));
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
