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
//! edge-backed computed fields wired through `Schedule::edges_from` /
//! `Schedule::edges_to`.

use crate::accessor_field_properties;
use crate::edge::EdgeKind;
use crate::entity::{EntityId, EntityType, EntityUuid, FieldSet, UuidPreference};
use crate::field::{CollectedField, CollectedHalfEdge, FieldDescriptor, NamedField};
use crate::query::converter::EntityStringResolver;
use crate::tables::hotel_room::{self, HotelRoomEntityType, HotelRoomId};
use crate::tables::panel::{self, PanelEntityType, PanelId};
use crate::value::ValidationError;
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

inventory::submit! {
    crate::entity::RegisteredEntityType {
        type_name: EventRoomEntityType::TYPE_NAME,
        uuid_namespace: EventRoomEntityType::uuid_namespace,
        type_id: || std::any::TypeId::of::<EventRoomInternalData>(),
        read_field_fn: |schedule, uuid, field_name| {
            // SAFETY: uuid came from an existing EventRoomEntityType entity.
            let id = unsafe { crate::entity::EntityId::<EventRoomEntityType>::new_unchecked(uuid) };
            EventRoomEntityType::field_set().read_field_value(field_name, id, schedule)
        },
        write_field_fn: |schedule, uuid, field_name, value| {
            // SAFETY: uuid came from an existing EventRoomEntityType entity.
            let id = unsafe { crate::entity::EntityId::<EventRoomEntityType>::new_unchecked(uuid) };
            EventRoomEntityType::field_set().write_field_value(field_name, id, schedule, value)
        },
        build_fn: |schedule, uuid, fields| {
            crate::edit::builder::build_entity::<EventRoomEntityType>(
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
            use crate::field::ReadableField;
            // SAFETY: uuid came from an existing EventRoomEntityType entity.
            let id = unsafe { crate::entity::EntityId::<EventRoomEntityType>::new_unchecked(uuid) };
            EventRoomEntityType::field_set()
                .fields()
                .filter(|d| d.cb.read_fn.is_some() && d.cb.write_fn.is_some())
                .filter_map(|d| {
                    d.read(id, schedule).ok().flatten().map(|v| (d.name(), v))
                })
                .collect()
        },
        remove_fn: |schedule, uuid| {
            // SAFETY: uuid came from an existing EventRoomEntityType entity.
            let id = unsafe { crate::entity::EntityId::<EventRoomEntityType>::new_unchecked(uuid) };
            schedule.remove_entity::<EventRoomEntityType>(id);
        },
        rehydrate_fn: |schedule, uuid| {
            crate::crdt::rehydrate_entity::<EventRoomEntityType>(schedule, uuid)
        },
    }
}

// ── EntityBuildable ─────────────────────────────────────────────────────────────

impl crate::edit::builder::EntityBuildable for EventRoomEntityType {
    fn default_data(id: EntityId<Self>) -> Self::InternalData {
        EventRoomInternalData {
            id,
            data: EventRoomCommonData::default(),
        }
    }
}

// ── EntityStringResolver implementation ─────────────────────────────────────────

impl EntityStringResolver for EventRoomEntityType {
    fn entity_to_string(schedule: &crate::schedule::Schedule, id: EntityId<Self>) -> String {
        schedule
            .get_internal(id)
            .map(|data| data.data.room_name.clone())
            .unwrap_or_else(|| id.to_string())
    }
}

// ── Stored field descriptors ──────────────────────────────────────────────────

pub static FIELD_ROOM_NAME: FieldDescriptor<EventRoomEntityType> = {
    let (data, cb) = accessor_field_properties! {
        EventRoomEntityType,
        room_name,
        name: "room_name",
        display: "Room Name",
        description: "Room code as it appears in the Schedule sheet's Room column.",
        aliases: &["room", "name"],
        cardinality: Single,
        item: String,
        example: "Panel 1",
        order: 0,
    };
    FieldDescriptor {
        data,
        required: true,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_ROOM_NAME) }

/// Optional display name shown in the widget / public schedule.
pub static FIELD_LONG_NAME: FieldDescriptor<EventRoomEntityType> = {
    let (data, cb) = accessor_field_properties! {
        EventRoomEntityType,
        long_name,
        name: "long_name",
        display: "Long Name",
        description: "Display name shown in the widget / public schedule.",
        aliases: &["display_name", "long"],
        cardinality: Optional,
        item: String,
        example: "Grand Ballroom A",
        order: 100,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_LONG_NAME) }

pub static FIELD_SORT_KEY: FieldDescriptor<EventRoomEntityType> = {
    let (data, cb) = accessor_field_properties! {
        EventRoomEntityType,
        sort_key,
        name: "sort_key",
        display: "Sort Key",
        description: "Ordering key; values >= 100 are hidden from the public schedule.",
        aliases: &["sort"],
        cardinality: Optional,
        item: Integer,
        example: "10",
        order: 200,
    };
    FieldDescriptor {
        data,
        required: false,
        edge_kind: EdgeKind::NonEdge,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_SORT_KEY) }

// ── Edge-backed computed fields ─────────────────────────────────────

pub static HALF_EDGE_HOTEL_ROOMS: crate::edge::HalfEdgeDescriptor<EventRoomEntityType> = {
    let (data, cb, edge_kind) = crate::edge_field_properties! {
        EventRoomEntityType,
        target: HotelRoomEntityType,
        target_field: &hotel_room::HALF_EDGE_EVENT_ROOMS,
        name: "hotel_rooms",
        display: "Hotel Rooms",
        description: "Hotel rooms that contain this event room.",
        aliases: &["hotel_room"],
        example: "[]",
        order: 300,
    };
    crate::edge::HalfEdgeDescriptor {
        data,
        edge_kind,
        cb,
    }
};
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_HOTEL_ROOMS) }

pub static HALF_EDGE_PANELS: crate::edge::HalfEdgeDescriptor<EventRoomEntityType> = {
    let (data, cb, edge_kind) = crate::edge_field_properties! {
        EventRoomEntityType,
        target: PanelEntityType,
        source_fields: &[&panel::HALF_EDGE_EVENT_ROOMS],
        name: "panels",
        display: "Panels",
        description: "Panels scheduled in this event room.",
        aliases: &["panel"],
        example: "[]",
        order: 400,
    };
    crate::edge::HalfEdgeDescriptor {
        data,
        edge_kind,
        cb,
    }
};
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_PANELS) }

/// Full edge from event room hotel rooms to hotel room event rooms
pub const EDGE_HOTEL_ROOMS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_HOTEL_ROOMS,
    far: &hotel_room::HALF_EDGE_EVENT_ROOMS,
};

/// Full edge from event room panels to panel event rooms
pub const EDGE_PANELS: crate::edge::FullEdge = crate::edge::FullEdge {
    near: &HALF_EDGE_PANELS,
    far: &panel::HALF_EDGE_EVENT_ROOMS,
};

// ── FieldSet ──────────────────────────────────────────────────────────────────

static EVENT_ROOM_FIELD_SET: LazyLock<FieldSet<EventRoomEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ── Builder ───────────────────────────────────────────────────────────────────

crate::field::macros::define_entity_builder! {
    /// Typed builder for [`EventRoomEntityType`] entities.
    EventRoomBuilder for EventRoomEntityType {
        /// Set the room code as it appears in the Schedule sheet (e.g. `"Panel 1"`).
        /// Required.
        with_room_name   => FIELD_ROOM_NAME,
        /// Set the optional display name shown in the widget / public schedule.
        with_long_name   => FIELD_LONG_NAME,
        /// Set the sort key; values `>= 100` hide the room from the public schedule.
        with_sort_key    => FIELD_SORT_KEY,
        /// Replace the set of hotel rooms that contain this event room.
        with_hotel_rooms => HALF_EDGE_HOTEL_ROOMS,
        /// Replace the set of panels scheduled in this event room.
        with_panels      => HALF_EDGE_PANELS,
    }
}

// ── EntityMatcher ─────────────────────────────────────────────────────────────

impl crate::query::lookup::EntityScannable for EventRoomEntityType {}

impl crate::query::lookup::EntityMatcher for EventRoomEntityType {
    fn match_entity(
        query: &str,
        data: &EventRoomInternalData,
    ) -> Option<crate::query::lookup::MatchPriority> {
        use crate::query::lookup::string_match_priority;
        let long = data.data.long_name.as_deref().unwrap_or("");
        [
            string_match_priority(query, &data.data.room_name),
            if long.is_empty() {
                None
            } else {
                string_match_priority(query, long)
            },
        ]
        .into_iter()
        .flatten()
        .max()
    }

    fn can_create(full: &str, partial: &str) -> crate::query::lookup::CanCreate {
        if partial.is_empty() {
            crate::query::lookup::CanCreate::No
        } else if full == partial {
            crate::query::lookup::CanCreate::Yes(crate::query::lookup::MatchConsumed::Full)
        } else {
            crate::query::lookup::CanCreate::Yes(crate::query::lookup::MatchConsumed::Partial)
        }
    }
}

// ── EntityCreatable ───────────────────────────────────────────────────────────

impl crate::query::lookup::EntityCreatable for EventRoomEntityType {
    fn create_from_string(
        schedule: &mut crate::schedule::Schedule,
        s: &str,
    ) -> Result<EntityId<Self>, crate::query::lookup::LookupError> {
        let id = EntityId::from_preference(UuidPreference::FromV5 {
            name: s.to_string(),
        });
        schedule.insert(
            id,
            EventRoomInternalData {
                id,
                data: EventRoomCommonData {
                    room_name: s.to_string(),
                    long_name: Some(s.to_string()),
                    sort_key: None,
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

    fn make_id() -> EventRoomId {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        unsafe { EventRoomId::new_unchecked(non_nil_uuid) }
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
        assert_eq!(fs.fields().count(), 3);
        assert_eq!(fs.half_edges().count(), 2);
        let required: Vec<_> = fs.required_fields().map(|d| d.name()).collect();
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
            Some(field_value!("Panel 1"))
        );
        assert_eq!(
            fs.read_field_value("long_name", id, &sched).unwrap(),
            Some(field_value!("Grand Ballroom A"))
        );
        assert_eq!(
            fs.read_field_value("sort_key", id, &sched).unwrap(),
            Some(field_value!(10))
        );
    }

    #[test]
    fn test_write_long_name_to_none() {
        let id = make_id();
        let mut sched = schedule_with(id, make_internal());
        let fs = EventRoomEntityType::field_set();
        fs.write_field_value("long_name", id, &mut sched, crate::field_empty_list!())
            .unwrap();
        let value = fs.read_field_value("long_name", id, &sched).unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_match_long_name_exact() {
        let data = make_internal();
        let priority = EventRoomEntityType::match_entity("grand ballroom a", &data);
        assert_eq!(priority, Some(match_priority::EXACT_MATCH));
    }

    #[test]
    fn test_match_long_name_absent_falls_through_to_room_name() {
        let mut internal = make_internal();
        internal.data.long_name = None;
        let priority = EventRoomEntityType::match_entity("panel", &internal);
        assert_eq!(priority, Some(match_priority::STRONG_MATCH));
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
    fn test_entity_to_string_returns_room_name() {
        use crate::query::converter::EntityStringResolver;
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let s = EventRoomEntityType::entity_to_string(&sched, id);
        assert_eq!(s, "Panel 1");
    }

    #[test]
    fn test_entity_to_string_fallback_to_uuid() {
        use crate::query::converter::EntityStringResolver;
        let id = make_id();
        let sched = Schedule::default();
        let s = EventRoomEntityType::entity_to_string(&sched, id);
        assert_eq!(s, id.to_string());
    }

    #[test]
    fn test_lookup_or_create_single_creates_new_entity() {
        use crate::query::lookup::lookup_or_create_single;
        let mut sched = Schedule::default();
        let id = lookup_or_create_single::<EventRoomEntityType>(&mut sched, "New Room").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.room_name, "New Room");
    }

    #[test]
    fn test_lookup_or_create_single_returns_existing() {
        use crate::query::lookup::lookup_or_create_single;
        let id = make_id();
        let mut sched = schedule_with(id, make_internal());
        let found_id =
            lookup_or_create_single::<EventRoomEntityType>(&mut sched, "Panel 1").unwrap();
        assert_eq!(found_id, id);
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
            Some(crate::field_empty_list!())
        );
        assert_eq!(
            fs.read_field_value("panels", id, &sched).unwrap(),
            Some(crate::field_empty_list!())
        );
    }

    // ── EntityCreatable ──────────────────────────────────────────────────────

    #[test]
    fn test_can_create_no_separator() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            EventRoomEntityType::can_create("Panel 1", "Panel 1"),
            CanCreate::Yes(crate::query::lookup::MatchConsumed::Full)
        ));
    }

    #[test]
    fn test_can_create_with_separator() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            EventRoomEntityType::can_create("Panel 1, Panel 2", "Panel 1"),
            CanCreate::Yes(crate::query::lookup::MatchConsumed::Partial)
        ));
    }

    #[test]
    fn test_can_create_empty_partial_returns_no() {
        use crate::query::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            EventRoomEntityType::can_create("Panel 1", ""),
            CanCreate::No
        ));
    }

    #[test]
    fn test_create_from_string_inserts_entity() {
        use crate::query::lookup::EntityCreatable;
        let mut sched = Schedule::default();
        let id = EventRoomEntityType::create_from_string(&mut sched, "Main Hall").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.room_name, "Main Hall");
        assert_eq!(data.data.long_name.as_deref(), Some("Main Hall"));
    }

    #[test]
    fn test_create_from_string_is_deterministic() {
        use crate::query::lookup::EntityCreatable;
        let mut sched1 = Schedule::default();
        let mut sched2 = Schedule::default();
        let id1 = EventRoomEntityType::create_from_string(&mut sched1, "Main Hall").unwrap();
        let id2 = EventRoomEntityType::create_from_string(&mut sched2, "Main Hall").unwrap();
        assert_eq!(id1, id2);
    }
}
