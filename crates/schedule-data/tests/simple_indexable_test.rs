/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Simple tests for indexable field functionality

use schedule_data::entity::panel::Panel;
use schedule_data::entity::EntityType;
use schedule_data::field::traits::{IndexableField, MatchStrength};

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

        // Test exact match for UID field (priority 220)
        if field.index_priority() == 220 {
            let result = field.match_field("panel-123", &panel);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), MatchStrength::ExactMatch);
        }

        // Test exact match for name field (priority 210) - using custom closure
        if field.index_priority() == 210 {
            let result = field.match_field("advanced rust programming", &panel);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), MatchStrength::ExactMatch);

            // Test contains match
            let result = field.match_field("rust", &panel);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), MatchStrength::StrongMatch);
        }
    }
}

#[test]
fn test_match_strength_ordering() {
    // Verify that MatchStrength enum has correct ordering
    assert!(MatchStrength::ExactMatch > MatchStrength::StrongMatch);
    assert!(MatchStrength::StrongMatch > MatchStrength::WeakMatch);
    assert!(MatchStrength::WeakMatch > MatchStrength::NotMatch);
}
