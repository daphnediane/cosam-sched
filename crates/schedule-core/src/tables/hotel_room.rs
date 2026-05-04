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

use crate::accessor_field_properties;
use crate::entity::{EntityId, EntityType, EntityUuid, UuidPreference};
use crate::field::set::FieldSet;
use crate::field::{CollectedField, CollectedHalfEdge, FieldDescriptor, NamedField};
use crate::query::converter::EntityStringResolver;
use crate::tables::event_room::{self, EventRoomEntityType, EventRoomId};
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
        read_field_fn: |schedule, uuid, field_name| {
            // SAFETY: uuid came from an existing HotelRoomEntityType entity.
            let id = unsafe { crate::entity::EntityId::<HotelRoomEntityType>::new_unchecked(uuid) };
            HotelRoomEntityType::field_set().read_field_value(field_name, id, schedule)
        },
        write_field_fn: |schedule, uuid, field_name, value| {
            // SAFETY: uuid came from an existing HotelRoomEntityType entity.
            let id = unsafe { crate::entity::EntityId::<HotelRoomEntityType>::new_unchecked(uuid) };
            HotelRoomEntityType::field_set().write_field_value(field_name, id, schedule, value)
        },
        build_fn: |schedule, uuid, fields| {
            crate::edit::builder::build_entity::<HotelRoomEntityType>(
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
            // SAFETY: uuid came from an existing HotelRoomEntityType entity.
            let id = unsafe { crate::entity::EntityId::<HotelRoomEntityType>::new_unchecked(uuid) };
            HotelRoomEntityType::field_set()
                .fields()
                .filter(|d| d.cb.read_fn.is_some() && d.cb.write_fn.is_some())
                .filter_map(|d| {
                    d.read(id, schedule).ok().flatten().map(|v| (d.name(), v))
                })
                .collect()
        },
        remove_fn: |schedule, uuid| {
            // SAFETY: uuid came from an existing HotelRoomEntityType entity.
            let id = unsafe { crate::entity::EntityId::<HotelRoomEntityType>::new_unchecked(uuid) };
            schedule.remove_entity::<HotelRoomEntityType>(id);
        },
        rehydrate_fn: |schedule, uuid| {
            crate::crdt::rehydrate_entity::<HotelRoomEntityType>(schedule, uuid)
        },
    }
}

// ── EntityBuildable ─────────────────────────────────────────────────────────────

impl crate::edit::builder::EntityBuildable for HotelRoomEntityType {
    fn default_data(id: EntityId<Self>) -> Self::InternalData {
        HotelRoomInternalData {
            id,
            data: HotelRoomCommonData::default(),
        }
    }
}

// ── EntityStringResolver implementation ─────────────────────────────────────────

impl EntityStringResolver for HotelRoomEntityType {
    fn entity_to_string(schedule: &crate::schedule::Schedule, id: EntityId<Self>) -> String {
        schedule
            .get_internal(id)
            .map(|data| data.data.hotel_room_name.clone())
            .unwrap_or_else(|| id.to_string())
    }
}

// ── Field descriptors ─────────────────────────────────────────────────────────

pub static FIELD_HOTEL_ROOM_NAME: crate::field::FieldDescriptor<HotelRoomEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        HotelRoomEntityType,
        hotel_room_name,
        name: "hotel_room_name",
        display: "Hotel Room Name",
        description: "Physical hotel room name / identifier.",
        aliases: &["name", "room_name"],
        cardinality: Single,
        item: String,
        example: "Ballroom East",
        order: 0,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: true,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_HOTEL_ROOM_NAME) }

pub static HALF_EDGE_EVENT_ROOMS: crate::edge::HalfEdgeDescriptor = {
    crate::edge::HalfEdgeDescriptor {
        data: crate::field::CommonFieldData {
            name: "event_rooms",
            display: "Event Rooms",
            description: "Event rooms contained within this hotel room.",
            aliases: &["event_room"],
            field_type: crate::value::FieldType(
                crate::value::FieldCardinality::List,
                crate::value::FieldTypeItem::EntityIdentifier(EventRoomEntityType::TYPE_NAME),
            ),
            example: "[]",
            order: 100,
        },
        edge_kind: crate::edge::EdgeKind::Target {
            source_fields: &[&event_room::HALF_EDGE_HOTEL_ROOMS],
        },
        entity_name: HotelRoomEntityType::TYPE_NAME,
    }
};
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_EVENT_ROOMS) }

/// Full edge from hotel room event rooms to event room hotel rooms
pub const EDGE_EVENT_ROOMS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_EVENT_ROOMS,
    far: &event_room::HALF_EDGE_HOTEL_ROOMS,
};

// ── FieldSet ──────────────────────────────────────────────────────────────────

static HOTEL_ROOM_FIELD_SET: LazyLock<FieldSet<HotelRoomEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ── Builder ───────────────────────────────────────────────────────────────────

crate::field::macros::define_entity_builder! {
    /// Typed builder for [`HotelRoomEntityType`] entities.
    HotelRoomBuilder for HotelRoomEntityType {
        /// Set the physical hotel room name (e.g. `"Ballroom East"`).  Required.
        with_hotel_room_name => FIELD_HOTEL_ROOM_NAME,
    }
}

// ── EntityMatcher ─────────────────────────────────────────────────────────────

impl crate::query::lookup::EntityScannable for HotelRoomEntityType {}

impl crate::query::lookup::EntityMatcher for HotelRoomEntityType {
    fn can_create(full: &str, partial: &str) -> crate::query::lookup::CanCreate {
        if partial.is_empty() {
            crate::query::lookup::CanCreate::No
        } else if full == partial {
            crate::query::lookup::CanCreate::Yes(crate::query::lookup::MatchConsumed::Full)
        } else {
            crate::query::lookup::CanCreate::Yes(crate::query::lookup::MatchConsumed::Partial)
        }
    }

    fn match_entity(
        query: &str,
        data: &HotelRoomInternalData,
    ) -> Option<crate::query::lookup::MatchPriority> {
        crate::query::lookup::string_match_priority(query, &data.data.hotel_room_name)
    }
}

// ── EntityCreatable ───────────────────────────────────────────────────────────

impl crate::query::lookup::EntityCreatable for HotelRoomEntityType {
    fn create_from_string(
        schedule: &mut crate::schedule::Schedule,
        s: &str,
    ) -> Result<EntityId<Self>, crate::query::lookup::LookupError> {
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
    use crate::query::lookup::{match_priority, EntityMatcher};
    use crate::schedule::Schedule;
    use uuid::Uuid;

    fn make_id() -> HotelRoomId {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        unsafe { HotelRoomId::new_unchecked(non_nil_uuid) }
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
        assert_eq!(fs.fields().count(), 1);
        assert_eq!(fs.half_edges().count(), 1);
        let required: Vec<_> = fs.required_fields().map(|d| d.name()).collect();
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
            Some(crate::field_empty_list!())
        );
    }

    #[test]
    fn test_entity_to_string_returns_hotel_room_name() {
        use crate::query::converter::EntityStringResolver;
        let id = make_id();
        let mut sched = Schedule::default();
        sched.insert(id, make_internal());
        let s = HotelRoomEntityType::entity_to_string(&sched, id);
        assert_eq!(s, "Ballroom East");
    }

    #[test]
    fn test_entity_to_string_fallback_to_uuid() {
        use crate::query::converter::EntityStringResolver;
        let id = make_id();
        let sched = Schedule::default();
        let s = HotelRoomEntityType::entity_to_string(&sched, id);
        assert_eq!(s, id.to_string());
    }

    #[test]
    fn test_lookup_or_create_single_creates_new_entity() {
        use crate::query::lookup::lookup_or_create_single;
        let mut sched = Schedule::default();
        let id =
            lookup_or_create_single::<HotelRoomEntityType>(&mut sched, "New Hotel Room").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.hotel_room_name, "New Hotel Room");
    }

    #[test]
    fn test_lookup_or_create_single_returns_existing() {
        use crate::query::lookup::lookup_or_create_single;
        let id = make_id();
        let mut sched = Schedule::default();
        sched.insert(id, make_internal());
        let found_id =
            lookup_or_create_single::<HotelRoomEntityType>(&mut sched, "Ballroom East").unwrap();
        assert_eq!(found_id, id);
    }

    // ── EntityCreatable ──────────────────────────────────────────────────────

    #[test]
    fn test_can_create_no_separator() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            HotelRoomEntityType::can_create("Grand Ballroom", "Grand Ballroom"),
            CanCreate::Yes(crate::query::lookup::MatchConsumed::Full)
        ));
    }

    #[test]
    fn test_can_create_with_separator() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            HotelRoomEntityType::can_create("Grand Ballroom, East Wing", "Grand Ballroom"),
            CanCreate::Yes(crate::query::lookup::MatchConsumed::Partial)
        ));
    }

    #[test]
    fn test_can_create_empty_partial_returns_no() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            HotelRoomEntityType::can_create("Grand Ballroom", ""),
            CanCreate::No
        ));
    }

    #[test]
    fn test_create_from_string_inserts_entity() {
        use crate::query::lookup::EntityCreatable;
        let mut sched = Schedule::default();
        let id = HotelRoomEntityType::create_from_string(&mut sched, "East Wing").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.hotel_room_name, "East Wing");
    }

    #[test]
    fn test_create_from_string_is_deterministic() {
        use crate::query::lookup::EntityCreatable;
        let mut sched1 = Schedule::default();
        let mut sched2 = Schedule::default();
        let id1 = HotelRoomEntityType::create_from_string(&mut sched1, "East Wing").unwrap();
        let id2 = HotelRoomEntityType::create_from_string(&mut sched2, "East Wing").unwrap();
        assert_eq!(id1, id2);
    }
}
