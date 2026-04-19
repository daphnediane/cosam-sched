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
//! The reverse `event_rooms` lookup is an edge-backed computed field wired
//! through `Schedule::edges_from`.

use crate::converter::EntityStringResolver;
use crate::entity::{EntityId, EntityType, FieldSet, UuidPreference};
use crate::event_room::{EventRoomEntityType, EventRoomId};
use crate::field_macros::{edge_list_field, req_string_field};
use crate::value::ConversionError;
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
    /// Event rooms contained within this hotel room — from edge maps.
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
        type_id: || std::any::TypeId::of::<HotelRoomInternalData>(),
    }
}
inventory::collect!(crate::entity::CollectedField<HotelRoomEntityType>);

// ── EntityStringResolver implementation ─────────────────────────────────────────

impl EntityStringResolver for HotelRoomEntityType {
    fn entity_to_string(schedule: &crate::schedule::Schedule, id: EntityId<Self>) -> String {
        schedule
            .get_internal(id)
            .map(|data| data.data.hotel_room_name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    fn lookup_or_create_string(
        schedule: &mut crate::schedule::Schedule,
        s: &str,
    ) -> Result<EntityId<Self>, ConversionError> {
        // Try default lookup first (UUID parsing, then match_index)
        if let Some(id) = Self::lookup_string(schedule, s) {
            return Ok(id);
        }

        // If not found, create a new HotelRoom with the hotel_room_name
        let id = EntityId::from_preference(UuidPreference::FromV5 {
            name: s.to_string(),
        });

        let internal_data = HotelRoomInternalData {
            id,
            data: HotelRoomCommonData {
                hotel_room_name: s.to_string(),
            },
        };

        schedule.insert(id, internal_data);
        Ok(id)
    }
}

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

// ── EntityMatcher ─────────────────────────────────────────────────────────────

impl crate::lookup::EntityMatcher for HotelRoomEntityType {
    fn match_entity(
        query: &str,
        data: &HotelRoomInternalData,
    ) -> Option<crate::lookup::MatchPriority> {
        crate::lookup::string_match_priority(query, &data.data.hotel_room_name)
    }
}

// ── EntityCreatable ───────────────────────────────────────────────────────────

impl crate::lookup::EntityCreatable for HotelRoomEntityType {
    fn can_create(full: &str, partial: &str) -> crate::lookup::CanCreate {
        if partial.is_empty() {
            crate::lookup::CanCreate::No
        } else if full == partial {
            crate::lookup::CanCreate::FromFull
        } else {
            crate::lookup::CanCreate::FromPartial
        }
    }

    fn create_from_string(
        schedule: &mut crate::schedule::Schedule,
        s: &str,
    ) -> Result<EntityId<Self>, crate::lookup::LookupError> {
        let id = EntityId::from_preference(UuidPreference::FromV5 {
            name: s.to_string(),
        });
        schedule.insert(
            id,
            HotelRoomInternalData {
                id,
                data: HotelRoomCommonData {
                    hotel_room_name: s.to_string(),
                },
            },
        );
        Ok(id)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_value;
    use crate::lookup::{match_priority, EntityMatcher};
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
    fn test_match_entity() {
        let data = make_internal();
        assert_eq!(
            HotelRoomEntityType::match_entity("ballroom east", &data),
            Some(match_priority::EXACT_MATCH)
        );
        assert_eq!(
            HotelRoomEntityType::match_entity("ball", &data),
            Some(match_priority::STRONG_MATCH)
        );
        assert_eq!(HotelRoomEntityType::match_entity("nope", &data), None);
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

    #[test]
    fn test_entity_to_string_returns_hotel_room_name() {
        use crate::converter::EntityStringResolver;
        let id = make_id();
        let mut sched = Schedule::default();
        sched.insert(id, make_internal());
        let s = HotelRoomEntityType::entity_to_string(&sched, id);
        assert_eq!(s, "Ballroom East");
    }

    #[test]
    fn test_entity_to_string_fallback_to_uuid() {
        use crate::converter::EntityStringResolver;
        let id = make_id();
        let sched = Schedule::default();
        let s = HotelRoomEntityType::entity_to_string(&sched, id);
        assert_eq!(s, id.to_string());
    }

    #[test]
    fn test_lookup_or_create_string_creates_new_entity() {
        use crate::converter::EntityStringResolver;
        let mut sched = Schedule::default();
        let id =
            HotelRoomEntityType::lookup_or_create_string(&mut sched, "New Hotel Room").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.hotel_room_name, "New Hotel Room");
    }

    #[test]
    fn test_lookup_or_create_string_returns_existing() {
        use crate::converter::EntityStringResolver;
        let id = make_id();
        let mut sched = Schedule::default();
        sched.insert(id, make_internal());
        let found_id =
            HotelRoomEntityType::lookup_or_create_string(&mut sched, "Ballroom East").unwrap();
        assert_eq!(found_id, id);
    }

    // ── EntityCreatable ──────────────────────────────────────────────────────

    #[test]
    fn test_can_create_no_separator() {
        use crate::lookup::{CanCreate, EntityCreatable};
        assert!(matches!(
            HotelRoomEntityType::can_create("Grand Ballroom", "Grand Ballroom"),
            CanCreate::FromFull
        ));
    }

    #[test]
    fn test_can_create_with_separator() {
        use crate::lookup::{CanCreate, EntityCreatable};
        assert!(matches!(
            HotelRoomEntityType::can_create("Grand Ballroom, East Wing", "Grand Ballroom"),
            CanCreate::FromPartial
        ));
    }

    #[test]
    fn test_can_create_empty_partial_returns_no() {
        use crate::lookup::{CanCreate, EntityCreatable};
        assert!(matches!(
            HotelRoomEntityType::can_create("Grand Ballroom", ""),
            CanCreate::No
        ));
    }

    #[test]
    fn test_create_from_string_inserts_entity() {
        use crate::lookup::EntityCreatable;
        let mut sched = Schedule::default();
        let id = HotelRoomEntityType::create_from_string(&mut sched, "East Wing").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.hotel_room_name, "East Wing");
    }

    #[test]
    fn test_create_from_string_is_deterministic() {
        use crate::lookup::EntityCreatable;
        let mut sched1 = Schedule::default();
        let mut sched2 = Schedule::default();
        let id1 = HotelRoomEntityType::create_from_string(&mut sched1, "East Wing").unwrap();
        let id2 = HotelRoomEntityType::create_from_string(&mut sched2, "East Wing").unwrap();
        assert_eq!(id1, id2);
    }
}
