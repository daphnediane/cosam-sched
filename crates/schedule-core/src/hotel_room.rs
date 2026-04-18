/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! HotelRoom entity — physical room as listed in the Rooms sheet's
//! "Hotel Room" column. One hotel room can contain multiple event rooms.
//!
//! Three structs define the HotelRoom entity:
//!
//! - [`HotelRoomCommonData`] — the hotel room name
//! - [`HotelRoomInternalData`] — `EntityType::InternalData`
//! - [`HotelRoomData`] — export/API view including flattened edge relationships
//!
//! The reverse `event_rooms` lookup is an edge-backed computed stub here and
//! fully wired in FEATURE-018.

use crate::entity::{EntityId, EntityType, FieldSet};
use crate::event_room::{EventRoomEntityType, EventRoomId};
use crate::field_macros::{edge_list_field, req_string_field};
use crate::value::ValidationError;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── Type aliases ──────────────────────────────────────────────────────────────

/// Type-safe identifier for HotelRoom entities.
pub type HotelRoomId = EntityId<HotelRoomEntityType>;

// ── HotelRoomCommonData ───────────────────────────────────────────────────────

/// User-facing hotel room fields from the Rooms sheet's Hotel Room column.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotelRoomCommonData {
    /// Physical room name (required, indexed).
    pub hotel_room_name: String,
}

impl HotelRoomCommonData {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.hotel_room_name.is_empty() {
            errors.push(ValidationError::Required {
                field: "hotel_room_name",
            });
        }
        errors
    }
}

// ── HotelRoomInternalData ─────────────────────────────────────────────────────

/// Runtime storage struct; the field system operates on this.
#[derive(Debug, Clone)]
pub struct HotelRoomInternalData {
    pub id: HotelRoomId,
    pub data: HotelRoomCommonData,
}

// ── HotelRoomData ─────────────────────────────────────────────────────────────

/// Export/API view produced by [`HotelRoomEntityType::export`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotelRoomData {
    #[serde(flatten)]
    pub data: HotelRoomCommonData,
    /// Event rooms contained within this hotel room — from edge maps
    /// (deferred to FEATURE-018).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub event_rooms: Vec<EventRoomId>,
}

// ── HotelRoomEntityType ───────────────────────────────────────────────────────

/// Singleton type representing the HotelRoom entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HotelRoomEntityType;

impl EntityType for HotelRoomEntityType {
    type InternalData = HotelRoomInternalData;
    type Data = HotelRoomData;

    const TYPE_NAME: &'static str = "hotel_room";

    fn uuid_namespace() -> &'static uuid::Uuid {
        static NS: LazyLock<uuid::Uuid> =
            LazyLock::new(|| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, b"hotel_room"));
        &NS
    }

    fn field_set() -> &'static FieldSet<Self> {
        &HOTEL_ROOM_FIELD_SET
    }

    fn export(internal: &Self::InternalData) -> Self::Data {
        HotelRoomData {
            data: internal.data.clone(),
            event_rooms: Vec::new(),
        }
    }

    fn validate(internal: &Self::InternalData) -> Vec<ValidationError> {
        internal.data.validate()
    }
}

inventory::submit! {
    crate::entity::RegisteredEntityType {
        type_name: HotelRoomEntityType::TYPE_NAME,
        uuid_namespace: HotelRoomEntityType::uuid_namespace,
    }
}
inventory::collect!(crate::entity::CollectedField<HotelRoomEntityType>);

// ── Field descriptors ─────────────────────────────────────────────────────────

req_string_field!(FIELD_HOTEL_ROOM_NAME, HotelRoomEntityType, HotelRoomInternalData, hotel_room_name,
    name: "hotel_room_name", display: "Hotel Room Name",
    desc: "Physical hotel room name / identifier.",
    aliases: &["name", "room_name"],
    example: "Ballroom East",
    order: 0);

edge_list_field!(FIELD_EVENT_ROOMS, HotelRoomEntityType, HotelRoomInternalData, target: EventRoomEntityType,
    name: "event_rooms", display: "Event Rooms",
    desc: "Event rooms contained within this hotel room.",
    aliases: &["event_room"],
    example: "[]",
    order: 100);

// ── FieldSet ──────────────────────────────────────────────────────────────────

static HOTEL_ROOM_FIELD_SET: LazyLock<FieldSet<HotelRoomEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::MatchPriority;
    use crate::field_value;
    use crate::schedule::Schedule;
    use uuid::Uuid;

    fn make_id() -> HotelRoomId {
        HotelRoomId::new(Uuid::new_v4()).expect("v4 is never nil")
    }

    fn make_internal() -> HotelRoomInternalData {
        HotelRoomInternalData {
            data: HotelRoomCommonData {
                hotel_room_name: "Ballroom East".into(),
            },
            id: make_id(),
        }
    }

    #[test]
    fn test_field_set_count_and_required() {
        let fs = HotelRoomEntityType::field_set();
        assert_eq!(fs.fields().count(), 2);
        let required: Vec<_> = fs.required_fields().map(|d| d.name).collect();
        assert_eq!(required, vec!["hotel_room_name"]);
    }

    #[test]
    fn test_field_set_aliases() {
        let fs = HotelRoomEntityType::field_set();
        assert!(fs.get_by_name("name").is_some());
        assert!(fs.get_by_name("room_name").is_some());
        assert!(fs.get_by_name("event_room").is_some());
    }

    #[test]
    fn test_read_and_write_name() {
        let id = make_id();
        let mut sched = Schedule::default();
        sched.insert(id, make_internal());

        let fs = HotelRoomEntityType::field_set();
        assert_eq!(
            fs.read_field_value("hotel_room_name", id, &sched).unwrap(),
            Some(field_value!("Ballroom East"))
        );

        fs.write_field_value(
            "hotel_room_name",
            id,
            &mut sched,
            field_value!("Ballroom West"),
        )
        .unwrap();
        assert_eq!(
            fs.read_field_value("hotel_room_name", id, &sched).unwrap(),
            Some(field_value!("Ballroom West"))
        );
    }

    #[test]
    fn test_match_index() {
        let data = make_internal();
        let fs = HotelRoomEntityType::field_set();
        assert_eq!(
            fs.match_index("ballroom east", &data),
            Some(MatchPriority::Exact)
        );
        assert_eq!(fs.match_index("ball", &data), Some(MatchPriority::Prefix));
        assert_eq!(fs.match_index("nope", &data), None);
    }

    #[test]
    fn test_common_data_serde_roundtrip() {
        let original = HotelRoomCommonData {
            hotel_room_name: "Main Hall".into(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: HotelRoomCommonData = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_validate_missing_name() {
        let data = HotelRoomCommonData::default();
        let errors = data.validate();
        assert_eq!(errors.len(), 1);
        assert!(
            matches!(errors[0], ValidationError::Required { field } if field == "hotel_room_name")
        );
    }

    #[test]
    fn test_edge_stub_returns_empty_list() {
        let id = make_id();
        let mut sched = Schedule::default();
        sched.insert(id, make_internal());
        let fs = HotelRoomEntityType::field_set();
        assert_eq!(
            fs.read_field_value("event_rooms", id, &sched).unwrap(),
            Some(field_value!(empty_list))
        );
    }
}
