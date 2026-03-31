/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Integration tests for the EntityFields derive macro against real entity types.
//!
//! These tests verify that the macro-generated code works correctly when used
//! from `schedule-data` where `crate::` paths resolve properly.

use schedule_data::entity::{Edge, EdgeType, Panel, PanelType, Presenter, Room};
use schedule_data::entity::{EntityState, EntityType};

// ---------------------------------------------------------------------------
// EntityType impl tests
// ---------------------------------------------------------------------------

#[test]
fn room_entity_type_name() {
    assert_eq!(Room::TYPE_NAME, "room");
}

#[test]
fn panel_entity_type_name() {
    assert_eq!(Panel::TYPE_NAME, "panel");
}

#[test]
fn presenter_entity_type_name() {
    assert_eq!(Presenter::TYPE_NAME, "presenter");
}

#[test]
fn panel_type_entity_type_name() {
    assert_eq!(PanelType::TYPE_NAME, "panel_type");
}

#[test]
fn edge_entity_type_name() {
    assert_eq!(Edge::TYPE_NAME, "edge");
}

// ---------------------------------------------------------------------------
// FieldSet tests — field_set() returns a populated set
// ---------------------------------------------------------------------------

#[test]
fn room_field_set_has_fields() {
    let fs = Room::field_set();
    assert!(!fs.fields.is_empty(), "Room should have fields");
    assert!(!fs.name_map.is_empty(), "Room should have a name map");
}

#[test]
fn room_field_set_required_fields() {
    let fs = Room::field_set();
    assert!(
        fs.is_required("long_name"),
        "long_name should be required on Room"
    );
    assert!(
        !fs.is_required("sort_key"),
        "sort_key should not be required on Room"
    );
}

#[test]
fn room_field_set_alias_lookup() {
    let fs = Room::field_set();
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
    let fs = Presenter::field_set();
    assert!(fs.get_field("name").is_some());
    assert!(fs.get_field("full_name").is_some());
    assert!(fs.get_field("display_name").is_some());
}

// ---------------------------------------------------------------------------
// Field read/write tests — Room
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn make_test_room() -> Room {
    Room {
        short_name: "Main".to_string(),
        long_name: "Main Ballroom".to_string(),
        hotel_room: "Ballroom A".to_string(),
        sort_key: 10,
        is_break: false,
    }
}

#[test]
fn room_read_string_field() {
    let fs = Room::field_set();

    // Find the short_name field and check NamedField metadata
    let field = fs.get_field("short_name").expect("short_name field exists");
    assert_eq!(field.name(), "short_name");
    assert_eq!(field.display_name(), "Room Name");
    assert_eq!(field.description(), "Short room name");
}

#[test]
fn room_read_bool_field() {
    let fs = Room::field_set();

    let field = fs.get_field("is_break").expect("is_break field exists");
    assert_eq!(field.name(), "is_break");
    assert_eq!(field.display_name(), "Is Break");
    assert_eq!(
        field.description(),
        "Whether this room is a virtual break room"
    );
}

#[test]
fn room_read_integer_field() {
    let fs = Room::field_set();

    let field = fs.get_field("sort_key").expect("sort_key field exists");
    assert_eq!(field.name(), "sort_key");
    assert_eq!(field.display_name(), "Sort Key");
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
    let fs = Edge::field_set();
    let field = fs.get_field("edge_type").expect("edge_type field exists");
    assert_eq!(field.name(), "edge_type");
    assert_eq!(field.display_name(), "Edge Type");
}

#[test]
fn edge_field_set_alias() {
    let fs = Edge::field_set();
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
fn room_all_field_names_includes_aliases() {
    let fs = Room::field_set();
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

// ---------------------------------------------------------------------------
// Indexable fields
// ---------------------------------------------------------------------------

#[test]
fn room_has_indexable_fields() {
    let fs = Room::field_set();
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
