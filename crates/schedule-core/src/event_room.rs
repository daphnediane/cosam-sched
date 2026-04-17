/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoom entity — a scheduled room as it appears on the Schedule sheet.
//!
//! Three structs define the EventRoom entity:
//!
//! - [`EventRoomCommonData`] — user-facing fields from the Rooms sheet
//! - [`EventRoomInternalData`] — `EntityType::InternalData`
//! - [`EventRoomData`] — export/API view including flattened edge relationships
//!
//! Room hierarchy (event rooms inside hotel rooms) and scheduling links are
//! edge-backed computed stubs here, fully wired in FEATURE-018.

use crate::entity::{EntityId, EntityType, FieldSet};
use crate::field::{FieldDescriptor, MatchPriority, ReadFn, WriteFn};
use crate::field_macros::{edge_list_field_rw, opt_i64_field, req_string_field};
use crate::hotel_room::HotelRoomId;
use crate::panel::PanelId;
use crate::value::{CrdtFieldType, FieldValue, ValidationError};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── Type aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for EventRoom entities.
pub type EventRoomId = EntityId<EventRoomEntityType>;

// ── EventRoomCommonData ───────────────────────────────────────────────────────

/// User-facing event room fields from the Rooms sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRoomCommonData {
    /// Room code that appears in the Schedule sheet's Room column
    /// (required, indexed).
    pub room_name: String,

    /// Display name shown in the widget / public schedule. Indexed so
    /// searches by display name still find the room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_name: Option<String>,

    /// Sort key for room ordering. Values `>= 100` are hidden from the
    /// public schedule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_key: Option<i64>,
}

impl EventRoomCommonData {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.room_name.is_empty() {
            errors.push(ValidationError::Required { field: "room_name" });
        }
        errors
    }
}

// ── EventRoomInternalData ─────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
#[derive(Debug, Clone)]
pub struct EventRoomInternalData {
    pub id: EventRoomId,
    pub data: EventRoomCommonData,
}

// ── EventRoomData ─────────────────────────────────────────────────────────────

/// Export/API view produced by [`EventRoomEntityType::export`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRoomData {
    #[serde(flatten)]
    pub data: EventRoomCommonData,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub hotel_room_ids: Vec<HotelRoomId>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub panels: Vec<PanelId>,
}

// ── EventRoomEntityType ───────────────────────────────────────────────────────

/// Singleton type representing the EventRoom entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventRoomEntityType;

impl EntityType for EventRoomEntityType {
    type InternalData = EventRoomInternalData;
    type Data = EventRoomData;

    const TYPE_NAME: &'static str = "event_room";

    fn uuid_namespace() -> &'static uuid::Uuid {
        static NS: LazyLock<uuid::Uuid> =
            LazyLock::new(|| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, b"event_room"));
        &NS
    }

    fn field_set() -> &'static FieldSet<Self> {
        &EVENT_ROOM_FIELD_SET
    }

    fn export(internal: &Self::InternalData) -> Self::Data {
        EventRoomData {
            data: internal.data.clone(),
            hotel_room_ids: Vec::new(),
            panels: Vec::new(),
        }
    }

    fn validate(internal: &Self::InternalData) -> Vec<ValidationError> {
        internal.data.validate()
    }
}

// ── Stored field descriptors ──────────────────────────────────────────────────

req_string_field!(FIELD_ROOM_NAME, EventRoomEntityType, EventRoomInternalData, room_name,
    name: "room_name", display: "Room Name",
    desc: "Room code as it appears in the Schedule sheet's Room column.",
    aliases: &["room", "name"]);

/// Optional display name, indexed so name-based searches still find the room.
/// Hand-written because the uniform `opt_string_field!` macro does not install
/// an `index_fn`.
static FIELD_LONG_NAME: FieldDescriptor<EventRoomEntityType> = FieldDescriptor {
    name: "long_name",
    display: "Long Name",
    description: "Display name shown in the widget / public schedule.",
    aliases: &["display_name", "long"],
    required: false,
    crdt_type: CrdtFieldType::Scalar,
    read_fn: Some(ReadFn::Bare(|d: &EventRoomInternalData| {
        Some(match &d.data.long_name {
            Some(s) => FieldValue::String(s.clone()),
            None => FieldValue::None,
        })
    })),
    write_fn: Some(WriteFn::Bare(|d: &mut EventRoomInternalData, v| {
        if v.is_none() {
            d.data.long_name = None;
        } else {
            d.data.long_name = Some(v.into_string()?);
        }
        Ok(())
    })),
    index_fn: Some(|query, d: &EventRoomInternalData| {
        let long = d.data.long_name.as_deref()?;
        let q = query.to_lowercase();
        let v = long.to_lowercase();
        if v == q {
            Some(MatchPriority::Exact)
        } else if v.starts_with(&q) {
            Some(MatchPriority::Prefix)
        } else if v.contains(&q) {
            Some(MatchPriority::Contains)
        } else {
            None
        }
    }),
    verify_fn: None,
};

opt_i64_field!(FIELD_SORT_KEY, EventRoomEntityType, EventRoomInternalData, sort_key,
    name: "sort_key", display: "Sort Key",
    desc: "Ordering key; values >= 100 are hidden from the public schedule.",
    aliases: &["sort"]);

// ── Edge-backed computed field stubs (full wiring in FEATURE-018) ─────────────

edge_list_field_rw!(FIELD_HOTEL_ROOMS, EventRoomEntityType, EventRoomInternalData,
    name: "hotel_rooms", display: "Hotel Rooms",
    desc: "Hotel rooms that contain this event room.",
    aliases: &["hotel_room"]);

edge_list_field_rw!(FIELD_PANELS, EventRoomEntityType, EventRoomInternalData,
    name: "panels", display: "Panels",
    desc: "Panels scheduled in this event room.",
    aliases: &["panel"]);

// ── FieldSet ──────────────────────────────────────────────────────────────────

static EVENT_ROOM_FIELD_SET: LazyLock<FieldSet<EventRoomEntityType>> = LazyLock::new(|| {
    FieldSet::new(&[
        &FIELD_ROOM_NAME,
        &FIELD_LONG_NAME,
        &FIELD_SORT_KEY,
        &FIELD_HOTEL_ROOMS,
        &FIELD_PANELS,
    ])
});

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;
    use uuid::Uuid;

    fn make_id() -> EventRoomId {
        EventRoomId::new(Uuid::new_v4()).expect("v4 is never nil")
    }

    fn make_internal() -> EventRoomInternalData {
        EventRoomInternalData {
            data: EventRoomCommonData {
                room_name: "Panel 1".into(),
                long_name: Some("Grand Ballroom A".into()),
                sort_key: Some(10),
            },
            id: make_id(),
        }
    }

    fn schedule_with(id: EventRoomId, data: EventRoomInternalData) -> Schedule {
        let mut sched = Schedule::default();
        sched.insert(id, data);
        sched
    }

    #[test]
    fn test_field_set_count_and_required() {
        let fs = EventRoomEntityType::field_set();
        assert_eq!(fs.fields().count(), 5);
        let required: Vec<_> = fs.required_fields().map(|d| d.name).collect();
        assert_eq!(required, vec!["room_name"]);
    }

    #[test]
    fn test_field_set_aliases() {
        let fs = EventRoomEntityType::field_set();
        assert!(fs.get_by_name("room").is_some());
        assert!(fs.get_by_name("name").is_some()); // room_name alias
        assert!(fs.get_by_name("display_name").is_some()); // long_name alias
        assert!(fs.get_by_name("sort").is_some());
    }

    #[test]
    fn test_read_fields() {
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let fs = EventRoomEntityType::field_set();
        assert_eq!(
            fs.read_field_value("room_name", id, &sched).unwrap(),
            Some(FieldValue::String("Panel 1".into()))
        );
        assert_eq!(
            fs.read_field_value("long_name", id, &sched).unwrap(),
            Some(FieldValue::String("Grand Ballroom A".into()))
        );
        assert_eq!(
            fs.read_field_value("sort_key", id, &sched).unwrap(),
            Some(FieldValue::Integer(10))
        );
    }

    #[test]
    fn test_write_long_name_to_none() {
        let id = make_id();
        let mut sched = schedule_with(id, make_internal());
        let fs = EventRoomEntityType::field_set();
        fs.write_field_value("long_name", id, &mut sched, FieldValue::None)
            .unwrap();
        let value = fs.read_field_value("long_name", id, &sched).unwrap();
        assert_eq!(value, Some(FieldValue::None));
    }

    #[test]
    fn test_match_long_name_exact() {
        let data = make_internal();
        let fs = EventRoomEntityType::field_set();
        let priority = fs.match_index("grand ballroom a", &data);
        assert_eq!(priority, Some(MatchPriority::Exact));
    }

    #[test]
    fn test_match_long_name_absent_falls_through_to_room_name() {
        let mut internal = make_internal();
        internal.data.long_name = None;
        let fs = EventRoomEntityType::field_set();
        let priority = fs.match_index("panel", &internal);
        assert_eq!(priority, Some(MatchPriority::Prefix));
    }

    #[test]
    fn test_common_data_serde_roundtrip() {
        let original = EventRoomCommonData {
            room_name: "Panel 1".into(),
            long_name: Some("Grand Ballroom A".into()),
            sort_key: Some(10),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: EventRoomCommonData = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_validate_missing_room_name() {
        let data = EventRoomCommonData::default();
        let errors = data.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::Required { field } if field == "room_name"));
    }

    #[test]
    fn test_edge_stubs_return_empty_list() {
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let fs = EventRoomEntityType::field_set();
        assert_eq!(
            fs.read_field_value("hotel_rooms", id, &sched).unwrap(),
            Some(FieldValue::List(Vec::new()))
        );
        assert_eq!(
            fs.read_field_value("panels", id, &sched).unwrap(),
            Some(FieldValue::List(Vec::new()))
        );
    }
}
