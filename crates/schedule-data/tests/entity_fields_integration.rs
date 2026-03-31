/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Integration tests for the EntityFields derive macro against real entity types.
//!
//! These tests verify that the macro-generated code works correctly when used
//! from `schedule-data` where `crate::` paths resolve properly.

use schedule_data::entity::{Edge, EdgeType, EventRoom, HotelRoom};
use schedule_data::entity::{
    EdgeEntityType, EventRoomEntityType, HotelRoomEntityType, PanelEntityType, PanelTypeEntityType,
    PresenterEntityType,
};
use schedule_data::entity::{EntityState, EntityType};

// ---------------------------------------------------------------------------
// EntityType impl tests
// ---------------------------------------------------------------------------

#[test]
fn event_room_entity_type_name() {
    assert_eq!(EventRoomEntityType::TYPE_NAME, "event_room");
}

#[test]
fn hotel_room_entity_type_name() {
    assert_eq!(HotelRoomEntityType::TYPE_NAME, "hotel_room");
}

#[test]
fn panel_entity_type_name() {
    assert_eq!(PanelEntityType::TYPE_NAME, "panel");
}

#[test]
fn presenter_entity_type_name() {
    assert_eq!(PresenterEntityType::TYPE_NAME, "presenter");
}

#[test]
fn panel_type_entity_type_name() {
    assert_eq!(PanelTypeEntityType::TYPE_NAME, "panel_type");
}

#[test]
fn edge_entity_type_name() {
    assert_eq!(EdgeEntityType::TYPE_NAME, "edge");
}

// ---------------------------------------------------------------------------
// FieldSet tests — field_set() returns a populated set
// ---------------------------------------------------------------------------

#[test]
fn event_room_field_set_has_fields() {
    let fs = EventRoomEntityType::field_set();
    assert!(!fs.fields.is_empty(), "EventRoom should have fields");
    assert!(!fs.name_map.is_empty(), "EventRoom should have a name map");
}

#[test]
fn hotel_room_field_set_has_fields() {
    let fs = HotelRoomEntityType::field_set();
    assert!(!fs.fields.is_empty(), "HotelRoom should have fields");
    assert!(!fs.name_map.is_empty(), "HotelRoom should have a name map");
}

#[test]
fn event_room_field_set_required_fields() {
    let fs = EventRoomEntityType::field_set();
    assert!(
        fs.is_required("long_name"),
        "long_name should be required on EventRoom"
    );
    assert!(
        !fs.is_required("is_break"),
        "is_break should not be required on EventRoom"
    );
}

#[test]
fn event_room_field_set_alias_lookup() {
    let fs = EventRoomEntityType::field_set();
    // Primary name
    assert!(
        fs.get_field("short_name").is_some(),
        "should find short_name by primary name"
    );
    // Alias
    assert!(
        fs.get_field("short").is_some(),
        "should find short_name via alias 'short'"
    );
    assert!(
        fs.get_field("room_name").is_some(),
        "should find short_name via alias 'room_name'"
    );
    // Non-existent
    assert!(
        fs.get_field("nonexistent_field").is_none(),
        "nonexistent field should return None"
    );
}

#[test]
fn presenter_field_set_alias_lookup() {
    let fs = PresenterEntityType::field_set();
    assert!(fs.get_field("name").is_some());
    assert!(fs.get_field("full_name").is_some());
    assert!(fs.get_field("display_name").is_some());
}

// ---------------------------------------------------------------------------
// Field read/write tests — EventRoom
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn make_test_event_room() -> EventRoom {
    EventRoom {
        short_name: "Main".to_string(),
        long_name: "Main Ballroom".to_string(),
        is_break: false,
    }
}

#[allow(dead_code)]
fn make_test_hotel_room() -> HotelRoom {
    HotelRoom {
        hotel_room: "Ballroom A".to_string(),
        sort_key: 10,
    }
}

#[test]
fn event_room_read_string_field() {
    let fs = EventRoomEntityType::field_set();

    // Find the short_name field and check NamedField metadata
    let field = fs.get_field("short_name").expect("short_name field exists");
    assert_eq!(field.name(), "short_name");
    assert_eq!(field.display_name(), "Room Name");
    assert_eq!(field.description(), "Short room name");
}

#[test]
fn event_room_read_bool_field() {
    let fs = EventRoomEntityType::field_set();

    let field = fs.get_field("is_break").expect("is_break field exists");
    assert_eq!(field.name(), "is_break");
    assert_eq!(field.display_name(), "Is Break");
    assert_eq!(
        field.description(),
        "Whether this room is a virtual break room"
    );
}

#[test]
fn hotel_room_read_integer_field() {
    let fs = HotelRoomEntityType::field_set();

    let field = fs.get_field("sort_key").expect("sort_key field exists");
    assert_eq!(field.name(), "sort_key");
    assert_eq!(field.display_name(), "Sort Key");
}

#[test]
fn hotel_room_read_hotel_room_field() {
    let fs = HotelRoomEntityType::field_set();

    let field = fs.get_field("hotel_room").expect("hotel_room field exists");
    assert_eq!(field.name(), "hotel_room");
    assert_eq!(field.display_name(), "Hotel Room");
    assert_eq!(field.description(), "Physical hotel room");
}

// ---------------------------------------------------------------------------
// Field read/write tests — Edge (computed fields)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn make_test_edge() -> Edge {
    Edge {
        from_uid: 1,
        to_uid: 2,
        edge_type: EdgeType::PanelToPresenter,
        metadata: std::collections::HashMap::new(),
    }
}

#[test]
fn edge_field_set_has_computed_field() {
    let fs = EdgeEntityType::field_set();
    let field = fs.get_field("edge_type").expect("edge_type field exists");
    assert_eq!(field.name(), "edge_type");
    assert_eq!(field.display_name(), "Edge Type");
}

#[test]
fn edge_field_set_alias() {
    let fs = EdgeEntityType::field_set();
    // "type" is an alias for edge_type
    assert!(
        fs.get_field("type").is_some(),
        "should find edge_type via alias 'type'"
    );
    assert!(
        fs.get_field("edgeType").is_some(),
        "should find edge_type via alias 'edgeType'"
    );
}

// ---------------------------------------------------------------------------
// all_field_names
// ---------------------------------------------------------------------------

#[test]
fn event_room_all_field_names_includes_aliases() {
    let fs = EventRoomEntityType::field_set();
    let names = fs.all_field_names();
    // Should include both primary names and aliases
    assert!(names.contains(&"short_name"), "should contain primary name");
    assert!(names.contains(&"short"), "should contain alias 'short'");
    assert!(
        names.contains(&"room_name"),
        "should contain alias 'room_name'"
    );
    assert!(
        names.contains(&"long_name"),
        "should contain primary 'long_name'"
    );
}

#[test]
fn hotel_room_all_field_names_includes_aliases() {
    let fs = HotelRoomEntityType::field_set();
    let names = fs.all_field_names();
    // Should include both primary names and aliases
    assert!(names.contains(&"hotel_room"), "should contain primary name");
    assert!(names.contains(&"hotel"), "should contain alias 'hotel'");
    assert!(
        names.contains(&"location"),
        "should contain alias 'location'"
    );
    assert!(
        names.contains(&"sort_key"),
        "should contain primary 'sort_key'"
    );
    assert!(names.contains(&"sort"), "should contain alias 'sort'");
    assert!(names.contains(&"order"), "should contain alias 'order'");
}

// ---------------------------------------------------------------------------
// Indexable fields
// ---------------------------------------------------------------------------

#[test]
fn event_room_has_indexable_fields() {
    let fs = EventRoomEntityType::field_set();
    let indexable = fs.get_indexable_fields();
    // NOTE: The macro parses #[indexable] but does not yet generate
    // IndexableField trait impls. This test documents the current state
    // and should be updated when IndexableField generation is implemented.
    assert_eq!(
        indexable.len(),
        0,
        "Indexable field generation not yet implemented in macro"
    );
}

#[test]
fn hotel_room_has_indexable_fields() {
    let fs = HotelRoomEntityType::field_set();
    let indexable = fs.get_indexable_fields();
    // NOTE: The macro parses #[indexable] but does not yet generate
    // IndexableField trait impls. This test documents the current state
    // and should be updated when IndexableField generation is implemented.
    assert_eq!(
        indexable.len(),
        0,
        "Indexable field generation not yet implemented in macro"
    );
}

// ---------------------------------------------------------------------------
// EntityState default
// ---------------------------------------------------------------------------

#[test]
fn entity_state_default_is_active() {
    assert_eq!(EntityState::default(), EntityState::Active);
}
