/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use crate::data::panel::{Panel, PanelPart, PanelSession};
use indexmap::IndexMap;
use std::default::Default;

fn create_test_panel_with_sessions(panel_id: &str, session_ids: &[&str]) -> Panel {
    let mut panel = Panel::new(panel_id.to_string());
    panel.name = "Test Panel".to_string();

    let mut part = PanelPart {
        part_num: None,
        description: None,
        note: None,
        prereq: None,
        alt_panelist: None,
        credited_presenters: Vec::new(),
        uncredited_presenters: Vec::new(),
        sessions: Vec::new(),
        change_state: crate::data::source_info::ChangeState::Unchanged,
    };

    for session_id in session_ids {
        let session = PanelSession {
            id: session_id.to_string(),
            session_num: None,
            description: None,
            note: None,
            prereq: None,
            alt_panelist: None,
            room_ids: Vec::new(),
            start_time: None,
            end_time: None,
            duration: 60,
            is_full: false,
            capacity: None,
            seats_sold: None,
            pre_reg_max: None,
            ticket_url: None,
            simple_tix_event: None,
            hide_panelist: false,
            credited_presenters: Vec::new(),
            uncredited_presenters: Vec::new(),
            notes_non_printing: None,
            workshop_notes: None,
            power_needs: None,
            sewing_machines: false,
            av_notes: None,
            source: None,
            change_state: crate::data::source_info::ChangeState::Unchanged,
            conflicts: Vec::new(),
            metadata: IndexMap::new(),
        };
        part.sessions.push(session);
    }

    panel.parts.push(part);
    panel
}

#[test]
fn test_extract_alpha_suffix() {
    assert_eq!(crate::data::post_process::extract_alpha_suffix("PL001"), "");
    assert_eq!(
        crate::data::post_process::extract_alpha_suffix("PL001A"),
        "A"
    );
    assert_eq!(
        crate::data::post_process::extract_alpha_suffix("PL001B"),
        "B"
    );
    assert_eq!(
        crate::data::post_process::extract_alpha_suffix("PL001AA"),
        "AA"
    );
    assert_eq!(
        crate::data::post_process::extract_alpha_suffix("GW006P1"),
        ""
    ); // P1 is session/part, not alpha suffix
    assert_eq!(
        crate::data::post_process::extract_alpha_suffix("GW006P1A"),
        "A"
    );
}

#[test]
fn test_strip_alpha_suffix() {
    assert_eq!(
        crate::data::post_process::strip_alpha_suffix("PL001"),
        "PL001"
    );
    assert_eq!(
        crate::data::post_process::strip_alpha_suffix("PL001A"),
        "PL001"
    );
    assert_eq!(
        crate::data::post_process::strip_alpha_suffix("PL001B"),
        "PL001"
    );
    assert_eq!(
        crate::data::post_process::strip_alpha_suffix("PL001AA"),
        "PL001"
    );
    assert_eq!(
        crate::data::post_process::strip_alpha_suffix("GW006P1"),
        "GW006P1"
    );
    assert_eq!(
        crate::data::post_process::strip_alpha_suffix("GW006P1A"),
        "GW006P1"
    );
}

#[test]
fn test_next_alpha_suffix() {
    assert_eq!(crate::data::post_process::next_alpha_suffix(""), "A");
    assert_eq!(crate::data::post_process::next_alpha_suffix("A"), "B");
    assert_eq!(crate::data::post_process::next_alpha_suffix("Y"), "Z");
    assert_eq!(crate::data::post_process::next_alpha_suffix("Z"), "AA");
    assert_eq!(crate::data::post_process::next_alpha_suffix("AA"), "AB");
    assert_eq!(crate::data::post_process::next_alpha_suffix("AZ"), "BA");
    assert_eq!(crate::data::post_process::next_alpha_suffix("ZZ"), "AAA");

    // Test skipping P and S
    assert_eq!(crate::data::post_process::next_alpha_suffix("O"), "Q"); // Skip P
    assert_eq!(crate::data::post_process::next_alpha_suffix("R"), "T"); // Skip S
}

#[test]
fn test_conflict_resolution_empty_suffix() {
    let mut schedule = crate::data::schedule::Schedule::default();
    let panel = create_test_panel_with_sessions("PL001", &["PL001", "PL001"]);
    schedule.panels.insert("PL001".to_string(), panel);

    crate::data::post_process::resolve_session_conflicts(&mut schedule);

    let panel = &schedule.panels["PL001"];
    let session_ids: Vec<String> = panel.parts[0]
        .sessions
        .iter()
        .map(|s| s.id.clone())
        .collect();

    // Should be PL001A and PL001B
    assert_eq!(session_ids.len(), 2);
    assert!(session_ids.contains(&"PL001A".to_string()));
    assert!(session_ids.contains(&"PL001B".to_string()));
}

#[test]
fn test_conflict_resolution_existing_suffix() {
    let mut schedule = crate::data::schedule::Schedule::default();
    let panel = create_test_panel_with_sessions("PL001", &["PL001B", "PL001B"]);
    schedule.panels.insert("PL001".to_string(), panel);

    crate::data::post_process::resolve_session_conflicts(&mut schedule);

    let panel = &schedule.panels["PL001"];
    let session_ids: Vec<String> = panel.parts[0]
        .sessions
        .iter()
        .map(|s| s.id.clone())
        .collect();

    // Should be PL001C and PL001D (B was in use, so advance)
    assert_eq!(session_ids.len(), 2);
    assert!(session_ids.contains(&"PL001C".to_string()));
    assert!(session_ids.contains(&"PL001D".to_string()));
}

#[test]
fn test_no_conflict_single_session() {
    let mut schedule = crate::data::schedule::Schedule::default();
    let panel = create_test_panel_with_sessions("PL001", &["PL001"]);
    schedule.panels.insert("PL001".to_string(), panel);

    crate::data::post_process::resolve_session_conflicts(&mut schedule);

    let panel = &schedule.panels["PL001"];
    assert_eq!(panel.parts[0].sessions.len(), 1);
    assert_eq!(panel.parts[0].sessions[0].id, "PL001");
}

#[test]
fn test_conflict_resolution_with_session_numbers() {
    let mut schedule = crate::data::schedule::Schedule::default();
    let mut panel = create_test_panel_with_sessions("PL001", &["PL001", "PL001"]);

    // Set different session numbers - should not conflict
    panel.parts[0].sessions[0].session_num = Some(1);
    panel.parts[0].sessions[1].session_num = Some(2);

    schedule.panels.insert("PL001".to_string(), panel);

    crate::data::post_process::resolve_session_conflicts(&mut schedule);

    let panel = &schedule.panels["PL001"];
    // Should not be resolved since session numbers are different
    assert_eq!(panel.parts[0].sessions.len(), 2);
    assert_eq!(panel.parts[0].sessions[0].id, "PL001");
    assert_eq!(panel.parts[0].sessions[1].id, "PL001");
    assert_eq!(panel.parts[0].sessions[0].session_num, Some(1));
    assert_eq!(panel.parts[0].sessions[1].session_num, Some(2));
}
