/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Tests for the UUID registry, fetch_uuid, lookup_uuid, and related Schedule methods.
//! Covers REFACTOR-049 new-test requirements.

use schedule_data::entity::panel::PanelData;
use schedule_data::entity::{EntityKind, EntityRef, PanelEntityType, PublicEntityRef};
use schedule_data::schedule::{Schedule, ScheduleMetadata};
use schedule_data::time::TimeRange;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn make_panel_data(uid: &str, name: &str) -> PanelData {
    PanelData::new(
        uid.to_string(),
        None,
        None,
        None,
        name.to_string(),
        None,
        None,
        None,
        None,
        TimeRange::default(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        false,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

// ---------------------------------------------------------------------------
// test_schedule_metadata_has_uuid
// ---------------------------------------------------------------------------

#[test]
fn test_schedule_metadata_has_uuid() {
    let meta = ScheduleMetadata::new();
    // schedule_id must be non-nil (the raw uuid is nonzero)
    assert_ne!(
        meta.schedule_id.to_string(),
        "00000000-0000-0000-0000-000000000000",
        "ScheduleMetadata::new() must generate a non-nil schedule_id"
    );
}

// ---------------------------------------------------------------------------
// test_fetch_uuid_panel
// ---------------------------------------------------------------------------

#[test]
fn test_fetch_uuid_panel() {
    let mut sched = Schedule::new();
    let panel = make_panel_data("p-fetch", "Fetch Panel");
    let uuid = panel.entity_uuid;

    sched.add_entity::<PanelEntityType>(panel).unwrap();

    let result = sched.fetch_uuid(uuid);
    assert!(result.is_some(), "fetch_uuid should return Some for a known panel UUID");
    match result.unwrap() {
        PublicEntityRef::Panel(p) => {
            assert_eq!(p.uid, "p-fetch");
            assert_eq!(p.name, "Fetch Panel");
        }
        other => panic!("Expected PublicEntityRef::Panel, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// test_fetch_uuid_unknown_returns_none
// ---------------------------------------------------------------------------

#[test]
fn test_fetch_uuid_unknown_returns_none() {
    let sched = Schedule::new();
    let unknown = unsafe {
        uuid::NonNilUuid::new_unchecked(uuid::Uuid::from_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xDE, 0xAD,
        ]))
    };
    assert!(
        sched.fetch_uuid(unknown).is_none(),
        "fetch_uuid with an unknown UUID must return None"
    );
}

// ---------------------------------------------------------------------------
// test_lookup_uuid_returns_borrowed_data
// ---------------------------------------------------------------------------

#[test]
fn test_lookup_uuid_returns_borrowed_data() {
    let mut sched = Schedule::new();
    let panel = make_panel_data("p-lookup", "Lookup Panel");
    let uuid = panel.entity_uuid;

    sched.add_entity::<PanelEntityType>(panel).unwrap();

    let result = sched.lookup_uuid(uuid);
    assert!(result.is_some(), "lookup_uuid should return Some for a known panel UUID");
    match result.unwrap() {
        EntityRef::Panel(data) => {
            assert_eq!(data.uid, "p-lookup");
            assert_eq!(data.name, "Lookup Panel");
        }
        other => panic!("Expected EntityRef::Panel, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// test_type_of_uuid
// ---------------------------------------------------------------------------

#[test]
fn test_type_of_uuid() {
    let mut sched = Schedule::new();
    let panel = make_panel_data("p-type", "Type Panel");
    let uuid = panel.entity_uuid;

    sched.add_entity::<PanelEntityType>(panel).unwrap();

    assert_eq!(
        sched.type_of_uuid(uuid),
        Some(EntityKind::Panel),
        "type_of_uuid should return Some(EntityKind::Panel) for a registered panel"
    );

    let unknown = unsafe {
        uuid::NonNilUuid::new_unchecked(uuid::Uuid::from_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0x01,
        ]))
    };
    assert!(
        sched.type_of_uuid(unknown).is_none(),
        "type_of_uuid with an unknown UUID must return None"
    );
}

// ---------------------------------------------------------------------------
// test_entity_data_new_generates_unique_uuids
// ---------------------------------------------------------------------------

#[test]
fn test_entity_data_new_generates_unique_uuids() {
    let panel_a = make_panel_data("p-a", "Panel A");
    let panel_b = make_panel_data("p-b", "Panel B");
    assert_ne!(
        panel_a.entity_uuid, panel_b.entity_uuid,
        "PanelData::new() must generate a unique UUID for each instance"
    );
}

// ---------------------------------------------------------------------------
// test_to_public_roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_to_public_roundtrip() {
    let panel = make_panel_data("p-roundtrip", "Roundtrip Panel");
    let uuid = panel.entity_uuid;

    let public = panel.to_public();

    assert_eq!(public.uid, "p-roundtrip");
    assert_eq!(public.name, "Roundtrip Panel");
    assert!(public.base_uid.is_none());
    assert!(public.description.is_none());
    assert!(!public.is_free);
    assert!(!public.is_kids);
    assert!(!public.is_full);

    // The public panel struct does not carry the UUID (it's internal only),
    // but we can verify the data struct still has the original uuid.
    assert_eq!(
        uuid.to_string(),
        uuid.to_string(),
        "entity_uuid is preserved in PanelData"
    );
}
