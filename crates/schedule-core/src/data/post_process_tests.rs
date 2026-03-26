/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use crate::data::panel::Panel;
use crate::data::panel_set::PanelSet;
use std::default::Default;

fn create_test_panelset_with_panels(base_id: &str, panel_ids: &[&str]) -> PanelSet {
    let mut ps = PanelSet::new(base_id);
    for &panel_id in panel_ids {
        let mut panel = Panel::new(panel_id, base_id);
        panel.name = "Test Panel".to_string();
        ps.panels.push(panel);
    }
    ps
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
    let ps = create_test_panelset_with_panels("PL001", &["PL001", "PL001"]);
    schedule.panel_sets.insert("PL001".to_string(), ps);

    crate::data::post_process::resolve_session_conflicts(&mut schedule);

    let ps = &schedule.panel_sets["PL001"];
    let panel_ids: Vec<String> = ps.panels.iter().map(|p| p.id.clone()).collect();

    // Should be PL001A and PL001B
    assert_eq!(panel_ids.len(), 2);
    assert!(panel_ids.contains(&"PL001A".to_string()));
    assert!(panel_ids.contains(&"PL001B".to_string()));
}

#[test]
fn test_conflict_resolution_existing_suffix() {
    let mut schedule = crate::data::schedule::Schedule::default();
    let ps = create_test_panelset_with_panels("PL001", &["PL001B", "PL001B"]);
    schedule.panel_sets.insert("PL001".to_string(), ps);

    crate::data::post_process::resolve_session_conflicts(&mut schedule);

    let ps = &schedule.panel_sets["PL001"];
    let panel_ids: Vec<String> = ps.panels.iter().map(|p| p.id.clone()).collect();

    // Should be PL001C and PL001D (B was in use, so advance)
    assert_eq!(panel_ids.len(), 2);
    assert!(panel_ids.contains(&"PL001C".to_string()));
    assert!(panel_ids.contains(&"PL001D".to_string()));
}

#[test]
fn test_no_conflict_single_session() {
    let mut schedule = crate::data::schedule::Schedule::default();
    let ps = create_test_panelset_with_panels("PL001", &["PL001"]);
    schedule.panel_sets.insert("PL001".to_string(), ps);

    crate::data::post_process::resolve_session_conflicts(&mut schedule);

    let ps = &schedule.panel_sets["PL001"];
    assert_eq!(ps.panels.len(), 1);
    assert_eq!(ps.panels[0].id, "PL001");
}

#[test]
fn test_conflict_resolution_with_session_numbers() {
    let mut schedule = crate::data::schedule::Schedule::default();
    let mut ps = create_test_panelset_with_panels("PL001", &["PL001", "PL001"]);

    // Different session_nums → these are NOT duplicates; ids left as-is
    ps.panels[0].session_num = Some(1);
    ps.panels[1].session_num = Some(2);

    schedule.panel_sets.insert("PL001".to_string(), ps);

    crate::data::post_process::resolve_session_conflicts(&mut schedule);

    let ps = &schedule.panel_sets["PL001"];
    assert_eq!(ps.panels.len(), 2);
    assert_eq!(ps.panels[0].id, "PL001");
    assert_eq!(ps.panels[1].id, "PL001");
    assert_eq!(ps.panels[0].session_num, Some(1));
    assert_eq!(ps.panels[1].session_num, Some(2));
}
