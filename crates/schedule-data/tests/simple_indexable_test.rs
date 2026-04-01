/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Simple tests for indexable field functionality

use schedule_data::entity::panel::Panel;
use schedule_data::entity::EntityType;
use schedule_data::field::traits::{match_priority, IndexableField};

#[test]
fn test_panel_indexable_functionality() {
    let panel = Panel {
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
    };

    // Test that we can access the field set and indexable fields
    let field_set = schedule_data::entity::PanelEntityType::field_set();
    let indexable_fields = field_set.indexable_fields;

    // Should have exactly 2 indexable fields
    assert_eq!(indexable_fields.len(), 2);

    // Test that both fields are indexable and have correct priorities
    let mut priorities: Vec<u8> = indexable_fields
        .iter()
        .map(|f| f.index_priority())
        .collect();
    priorities.sort_by(|a, b| b.cmp(a)); // Sort descending

    // Highest priority should be 220 (UID), then 210 (name)
    assert_eq!(priorities[0], 220);
    assert_eq!(priorities[1], 210);

    // Test matching functionality on the indexable fields
    for field in indexable_fields {
        assert!(field.is_indexable());

        // Test exact match for UID field (priority 220) - should return scaled value
        if field.index_priority() == 220 {
            let result = field.match_field("panel-123", &panel);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), 220); // Scaled exact match: (255 * 220) / 255 = 220
        }

        // Test exact match for name field (priority 210) - using custom closure
        if field.index_priority() == 210 {
            let result = field.match_field("advanced rust programming", &panel);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), 210); // Scaled exact match: (255 * 210) / 255 = 210

            // Test contains match (will be AverageMatch due to word boundary)
            let result = field.match_field("rust", &panel);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), 82); // Scaled average match: (100 * 210) / 255 = 82

            // Test starts with match - "adv" should match "Advanced Rust Programming" as StrongMatch
            let result = field.match_field("adv", &panel);
            assert!(
                result.is_some(),
                "Should match 'adv' in 'Advanced Rust Programming' as StrongMatch"
            );
            assert_eq!(result.unwrap(), 164); // Scaled strong match: (200 * 210) / 255 = 164

            // Test that "ann" should NOT match "Advanced Rust Programming" (not a word boundary)
            let result = field.match_field("ann", &panel);
            assert!(
                result.is_none(),
                "Should not match 'ann' in 'Advanced Rust Programming'"
            );

            // Test that "shop" would match "Work Shop Room" but not "Workshop"
            let workshop_panel = Panel {
                name: "Work Shop Room-3".to_string(),
                ..panel.clone()
            };
            let result = field.match_field("shop", &workshop_panel);
            assert!(
                result.is_some(),
                "Should match 'shop' as word boundary in 'Work Shop Room-3'"
            );
            assert_eq!(result.unwrap(), 82); // Scaled average match: (100 * 210) / 255 = 82

            let workshop_panel2 = Panel {
                name: "Workshop rooming".to_string(),
                ..panel.clone()
            };
            let result = field.match_field("shop", &workshop_panel2);
            assert!(
                result.is_some(),
                "Should match 'shop' in 'Workshop rooming' as WeakMatch"
            );
            assert_eq!(result.unwrap(), 41); // Scaled weak match: (50 * 210) / 255 = 41
        }
    }
}

#[test]
fn test_match_priority_ordering() {
    // Verify that match priority constants have correct ordering
    assert!(match_priority::EXACT_MATCH > match_priority::STRONG_MATCH);
    assert!(match_priority::STRONG_MATCH > match_priority::AVERAGE_MATCH);
    assert!(match_priority::AVERAGE_MATCH > match_priority::WEAK_MATCH);
    assert!(match_priority::WEAK_MATCH > match_priority::NO_MATCH);
}
