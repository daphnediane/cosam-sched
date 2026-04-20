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

use crate::converter::EntityStringResolver;
use crate::entity::{EntityId, EntityType, UuidPreference};
use crate::field::{FieldDescriptor, ReadFn, WriteFn};
use crate::field_macros::{define_field, edge_list_field_rw, opt_i64_field, req_string_field};
use crate::field_set::FieldSet;
use crate::field_value;
use crate::hotel_room::{HotelRoomEntityType, HotelRoomId};
use crate::panel::{PanelEntityType, PanelId};
use crate::value::{CrdtFieldType, FieldCardinality, FieldType, FieldTypeItem, ValidationError};
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
    }
}
inventory::collect!(crate::entity::CollectedField<EventRoomEntityType>);

// ── EntityBuildable ─────────────────────────────────────────────────────────────

impl crate::builder::EntityBuildable for EventRoomEntityType {
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

req_string_field!(FIELD_ROOM_NAME, EventRoomEntityType, EventRoomInternalData, room_name,
    name: "room_name", display: "Room Name",
    desc: "Room code as it appears in the Schedule sheet's Room column.",
    aliases: &["room", "name"],
    example: "Panel 1",
    order: 0);

define_field!(
    /// Optional display name shown in the widget / public schedule.
    static FIELD_LONG_NAME: FieldDescriptor<EventRoomEntityType> = FieldDescriptor {
        name: "long_name",
        display: "Long Name",
        description: "Display name shown in the widget / public schedule.",
        aliases: &["display_name", "long"],
        required: false,
        crdt_type: CrdtFieldType::Scalar,
        field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::String),
        example: "Grand Ballroom A",
        order: 100,
        read_fn: Some(ReadFn::Bare(|d: &EventRoomInternalData| {
            d.data.long_name.as_ref().map(|s| field_value!(s.clone()))
        })),
        write_fn: Some(WriteFn::Bare(|d: &mut EventRoomInternalData, v| {
            if v.is_empty() {
                d.data.long_name = None;
            } else {
                d.data.long_name = Some(v.into_string()?);
            }
            Ok(())
        })),
        verify_fn: None,
    }
);

opt_i64_field!(FIELD_SORT_KEY, EventRoomEntityType, EventRoomInternalData, sort_key,
    name: "sort_key", display: "Sort Key",
    desc: "Ordering key; values >= 100 are hidden from the public schedule.",
    aliases: &["sort"],
    example: "10",
    order: 200);

// ── Edge-backed computed fields ───────────────────────────────────────────────

edge_list_field_rw!(FIELD_HOTEL_ROOMS, EventRoomEntityType, EventRoomInternalData, target: HotelRoomEntityType,
    name: "hotel_rooms", display: "Hotel Rooms",
    desc: "Hotel rooms that contain this event room.",
    aliases: &["hotel_room"],
    example: "[]",
    order: 300);

edge_list_field_rw!(FIELD_PANELS, EventRoomEntityType, EventRoomInternalData, target: PanelEntityType,
    name: "panels", display: "Panels",
    desc: "Panels scheduled in this event room.",
    aliases: &["panel"],
    example: "[]",
    order: 400);

// ── FieldSet ──────────────────────────────────────────────────────────────────

static EVENT_ROOM_FIELD_SET: LazyLock<FieldSet<EventRoomEntityType>> =
    LazyLock::new(FieldSet::from_inventory);

// ── Builder ───────────────────────────────────────────────────────────────────

crate::field_macros::define_entity_builder! {
    /// Typed builder for [`EventRoomEntityType`] entities (FEATURE-017).
    EventRoomBuilder for EventRoomEntityType {
        /// Set the room code as it appears in the Schedule sheet (e.g. `"Panel 1"`).
        /// Required.
        with_room_name   => FIELD_ROOM_NAME,
        /// Set the optional display name shown in the widget / public schedule.
        with_long_name   => FIELD_LONG_NAME,
        /// Set the sort key; values `>= 100` hide the room from the public schedule.
        with_sort_key    => FIELD_SORT_KEY,
        /// Replace the set of hotel rooms that contain this event room.
        with_hotel_rooms => FIELD_HOTEL_ROOMS,
        /// Replace the set of panels scheduled in this event room.
        with_panels      => FIELD_PANELS,
    }
}

// ── EntityMatcher ─────────────────────────────────────────────────────────────

impl crate::lookup::EntityScannable for EventRoomEntityType {}

impl crate::lookup::EntityMatcher for EventRoomEntityType {
    fn match_entity(
        query: &str,
        data: &EventRoomInternalData,
    ) -> Option<crate::lookup::MatchPriority> {
        use crate::lookup::string_match_priority;
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

    fn can_create(full: &str, partial: &str) -> crate::lookup::CanCreate {
        if partial.is_empty() {
            crate::lookup::CanCreate::No
        } else if full == partial {
            crate::lookup::CanCreate::Yes(crate::lookup::MatchConsumed::Full)
        } else {
            crate::lookup::CanCreate::Yes(crate::lookup::MatchConsumed::Partial)
        }
    }
}

// ── EntityCreatable ───────────────────────────────────────────────────────────

impl crate::lookup::EntityCreatable for EventRoomEntityType {
    fn create_from_string(
        schedule: &mut crate::schedule::Schedule,
        s: &str,
    ) -> Result<EntityId<Self>, crate::lookup::LookupError> {
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
    use crate::lookup::{match_priority, EntityMatcher};
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
        fs.write_field_value("long_name", id, &mut sched, field_value!(empty_list))
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
        use crate::converter::EntityStringResolver;
        let id = make_id();
        let sched = schedule_with(id, make_internal());
        let s = EventRoomEntityType::entity_to_string(&sched, id);
        assert_eq!(s, "Panel 1");
    }

    #[test]
    fn test_entity_to_string_fallback_to_uuid() {
        use crate::converter::EntityStringResolver;
        let id = make_id();
        let sched = Schedule::default();
        let s = EventRoomEntityType::entity_to_string(&sched, id);
        assert_eq!(s, id.to_string());
    }

    #[test]
    fn test_lookup_or_create_single_creates_new_entity() {
        use crate::lookup::lookup_or_create_single;
        let mut sched = Schedule::default();
        let id = lookup_or_create_single::<EventRoomEntityType>(&mut sched, "New Room").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.room_name, "New Room");
    }

    #[test]
    fn test_lookup_or_create_single_returns_existing() {
        use crate::lookup::lookup_or_create_single;
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
            Some(field_value!(empty_list))
        );
        assert_eq!(
            fs.read_field_value("panels", id, &sched).unwrap(),
            Some(field_value!(empty_list))
        );
    }

    // ── EntityCreatable ──────────────────────────────────────────────────────

    #[test]
    fn test_can_create_no_separator() {
        use crate::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            EventRoomEntityType::can_create("Panel 1", "Panel 1"),
            CanCreate::Yes(crate::lookup::MatchConsumed::Full)
        ));
    }

    #[test]
    fn test_can_create_with_separator() {
        use crate::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            EventRoomEntityType::can_create("Panel 1, Panel 2", "Panel 1"),
            CanCreate::Yes(crate::lookup::MatchConsumed::Partial)
        ));
    }

    #[test]
    fn test_can_create_empty_partial_returns_no() {
        use crate::lookup::{CanCreate, EntityMatcher};
        assert!(matches!(
            EventRoomEntityType::can_create("Panel 1", ""),
            CanCreate::No
        ));
    }

    #[test]
    fn test_create_from_string_inserts_entity() {
        use crate::lookup::EntityCreatable;
        let mut sched = Schedule::default();
        let id = EventRoomEntityType::create_from_string(&mut sched, "Main Hall").unwrap();
        let data = sched.get_internal(id).unwrap();
        assert_eq!(data.data.room_name, "Main Hall");
        assert_eq!(data.data.long_name.as_deref(), Some("Main Hall"));
    }

    #[test]
    fn test_create_from_string_is_deterministic() {
        use crate::lookup::EntityCreatable;
        let mut sched1 = Schedule::default();
        let mut sched2 = Schedule::default();
        let id1 = EventRoomEntityType::create_from_string(&mut sched1, "Main Hall").unwrap();
        let id2 = EventRoomEntityType::create_from_string(&mut sched2, "Main Hall").unwrap();
        assert_eq!(id1, id2);
    }
}
