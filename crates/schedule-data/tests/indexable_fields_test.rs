/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Tests for indexable field functionality and match strength scoring

use schedule_data::entity::panel::PanelData;
use schedule_data::field::traits::{match_priority, IndexableField};

#[test]
fn test_panel_uid_exact_match() {
    let panel = PanelData {
        entity_id: 0,
        uid: "panel-123".to_string(),
        base_uid: None,
        part_num: None,
        session_num: None,
        name: "Test Panel".to_string(),
        panel_type_uid: None,
        description: None,
        note: None,
        prereq: None,
        time_range: schedule_data::time::TimeRange::default(),
        cost: None,
        capacity: None,
        pre_reg_max: None,
        difficulty: None,
        ticket_url: None,
        simple_tix_event: None,
        have_ticket_image: None,
        is_free: false,
        is_kids: false,
        is_full: false,
        hide_panelist: false,
        sewing_machines: false,
        alt_panelist: None,
        seats_sold: None,
        notes_non_printing: None,
        workshop_notes: None,
        power_needs: None,
        av_notes: None,
        presenters: Vec::new(),
        event_room: None,
        panel_type: None,
    };

    let uid_field = schedule_data::entity::panel::UidField;

    // Test exact match
    let result = uid_field.match_field("panel-123", &panel);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 220); // Scaled exact match: (255 * 220) / 255 = 220

    // Test case sensitivity (should be exact match for UID field)
    let result = uid_field.match_field("Panel-123", &panel);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 220); // Scaled exact match: (255 * 220) / 255 = 220

    // Test no match
    let result = uid_field.match_field("panel-456", &panel);
    assert!(result.is_none());

    // Test empty query
    let result = uid_field.match_field("", &panel);
    assert!(result.is_none());
}

#[test]
fn test_panel_name_match_strengths() {
    let panel = PanelData {
        entity_id: 0,
        uid: "panel-123".to_string(),
        base_uid: None,
        part_num: None,
        session_num: None,
        name: "Advanced Rust Programming".to_string(),
        panel_type_uid: None,
        description: None,
        note: None,
        prereq: None,
        time_range: schedule_data::time::TimeRange::default(),
        cost: None,
        capacity: None,
        pre_reg_max: None,
        difficulty: None,
        ticket_url: None,
        simple_tix_event: None,
        have_ticket_image: None,
        is_free: false,
        is_kids: false,
        is_full: false,
        hide_panelist: false,
        sewing_machines: false,
        alt_panelist: None,
        seats_sold: None,
        notes_non_printing: None,
        workshop_notes: None,
        power_needs: None,
        av_notes: None,
        presenters: Vec::new(),
        event_room: None,
        panel_type: None,
    };

    let name_field = schedule_data::entity::panel::NameField;

    // Test exact match (case insensitive)
    let result = name_field.match_field("advanced rust programming", &panel);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 210); // Scaled exact match: (255 * 210) / 255 = 210

    // Test exact match (different case)
    let result = name_field.match_field("ADVANCED RUST PROGRAMMING", &panel);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 210); // Scaled exact match: (255 * 210) / 255 = 210

    // Test contains match
    let result = name_field.match_field("rust", &panel);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 82); // Scaled average match: (100 * 210) / 255 = 82

    // Test starts with match
    let result = name_field.match_field("adv", &panel);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 164); // Scaled strong match: (200 * 210) / 255 = 164

    // Test no match
    let result = name_field.match_field("python", &panel);
    assert!(result.is_none());

    // Test empty query
    let result = name_field.match_field("", &panel);
    assert!(result.is_none());
}

#[test]
fn test_indexable_field_priority() {
    let uid_field = schedule_data::entity::panel::UidField;
    let name_field = schedule_data::entity::panel::NameField;

    // UID should have higher priority (220) than name (210)
    assert!(uid_field.index_priority() > name_field.index_priority());
    assert_eq!(uid_field.index_priority(), 220);
    assert_eq!(name_field.index_priority(), 210);
}

#[test]
fn test_match_priority_ordering() {
    // Verify that match priority constants have correct ordering
    assert!(match_priority::EXACT_MATCH > match_priority::STRONG_MATCH);
    assert!(match_priority::STRONG_MATCH > match_priority::AVERAGE_MATCH);
    assert!(match_priority::AVERAGE_MATCH > match_priority::WEAK_MATCH);
    assert!(match_priority::WEAK_MATCH > match_priority::NO_MATCH);
}

#[test]
fn test_panel_uid_priority_over_name() {
    let uid_field = schedule_data::entity::panel::UidField;
    let name_field = schedule_data::entity::panel::NameField;

    // UID field should have higher priority
    assert!(uid_field.index_priority() > name_field.index_priority());

    // This means in ranked results, UID matches should come before name matches
    // when both have the same match strength
}
