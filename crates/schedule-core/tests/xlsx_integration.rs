/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Integration tests for XLSX import (FEATURE-028) and export (FEATURE-029).
//!
//! Import tests build minimal spreadsheets in memory using `umya-spreadsheet`,
//! write them to a temp file, run `import_xlsx`, then assert on the resulting
//! `Schedule` entities and edges.
//!
//! Export tests follow a round-trip approach: build a schedule via import,
//! export it with `export_xlsx`, re-import the exported file, then assert the
//! data is equivalent.

use std::path::PathBuf;

use schedule_core::tables::event_room::{self as event_room, EventRoomEntityType};
use schedule_core::tables::hotel_room::HotelRoomEntityType;
use schedule_core::tables::panel::{self, PanelEntityType};
use schedule_core::tables::panel_type::PanelTypeEntityType;
use schedule_core::tables::presenter::{self as presenter, PresenterEntityType};
use schedule_core::xlsx::{export_xlsx, import_xlsx, XlsxImportOptions};

// ── Spreadsheet builder helpers ───────────────────────────────────────────────

fn set_cell(ws: &mut umya_spreadsheet::Worksheet, col: u32, row: u32, val: &str) {
    ws.get_cell_mut((col, row)).set_value(val);
}

/// Write the workbook to a unique temp file and return its path.
fn write_temp(book: umya_spreadsheet::Spreadsheet) -> PathBuf {
    // Combine thread ID and full nanos-since-epoch to prevent collisions when
    // tests run in parallel on a thread pool.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tid = format!("{:?}", std::thread::current().id());
    let tid_hash: u64 = tid
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let path = std::env::temp_dir().join(format!("cosam_test_{nanos}_{tid_hash}.xlsx"));
    umya_spreadsheet::writer::xlsx::write(&book, &path).expect("write temp xlsx");
    path
}

fn cleanup(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_import_empty_book_returns_empty_schedule() {
    let mut book = umya_spreadsheet::new_file();
    let ws = book.get_sheet_mut(&0).unwrap();
    ws.set_name("Schedule");
    set_cell(ws, 1, 1, "Name"); // header row only, no data

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    assert_eq!(schedule.entity_count::<PanelEntityType>(), 0);
    assert_eq!(schedule.entity_count::<EventRoomEntityType>(), 0);
}

#[test]
fn test_import_panel_types_sheet() {
    let mut book = umya_spreadsheet::new_file();

    // PanelTypes sheet
    {
        let ws = book.new_sheet("PanelTypes").unwrap();
        // Row 1: headers
        set_cell(ws, 1, 1, "Prefix");
        set_cell(ws, 2, 1, "Panel Kind");
        set_cell(ws, 3, 1, "Is Workshop");
        // Row 2: GP
        set_cell(ws, 1, 2, "GP");
        set_cell(ws, 2, 2, "Guest Panel");
        // Row 3: FW
        set_cell(ws, 1, 3, "FW");
        set_cell(ws, 2, 3, "Fan Workshop");
        set_cell(ws, 3, 3, "Yes");
    }

    // Minimal Schedule sheet so import doesn't fail
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Name");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    assert_eq!(schedule.entity_count::<PanelTypeEntityType>(), 2);

    // Verify GP panel type
    let gp = schedule
        .iter_entities::<PanelTypeEntityType>()
        .find(|(_, d)| d.data.prefix == "GP")
        .map(|(_, d)| d.data.clone())
        .expect("GP panel type should exist");
    assert_eq!(gp.panel_kind, "Guest Panel");
    assert!(!gp.is_workshop);

    // Verify FW panel type
    let fw = schedule
        .iter_entities::<PanelTypeEntityType>()
        .find(|(_, d)| d.data.prefix == "FW")
        .map(|(_, d)| d.data.clone())
        .expect("FW panel type should exist");
    assert!(fw.is_workshop);
}

#[test]
fn test_import_rooms_sheet() {
    let mut book = umya_spreadsheet::new_file();

    // Rooms sheet
    {
        let ws = book.new_sheet("Rooms").unwrap();
        set_cell(ws, 1, 1, "Room Name");
        set_cell(ws, 2, 1, "Long Name");
        set_cell(ws, 3, 1, "Hotel Room");
        set_cell(ws, 4, 1, "Sort Key");

        set_cell(ws, 1, 2, "Panel Room 1");
        set_cell(ws, 2, 2, "Main Panel Room");
        set_cell(ws, 3, 2, "Hotel A");
        set_cell(ws, 4, 2, "10");

        set_cell(ws, 1, 3, "Workshop Room");
        set_cell(ws, 2, 3, "Workshop Space");
        set_cell(ws, 3, 3, "Hotel A");
        set_cell(ws, 4, 3, "20");
    }

    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Name");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    assert_eq!(schedule.entity_count::<EventRoomEntityType>(), 2);
    // Both rooms share the same hotel room → 1 HotelRoom entity.
    assert_eq!(schedule.entity_count::<HotelRoomEntityType>(), 1);

    let pr1 = schedule
        .iter_entities::<EventRoomEntityType>()
        .find(|(_, d)| d.data.room_name == "Panel Room 1")
        .map(|(_, d)| d.data.clone())
        .expect("Panel Room 1 should exist");
    assert_eq!(pr1.long_name.as_deref(), Some("Main Panel Room"));
    assert_eq!(pr1.sort_key, Some(10));
}

#[test]
fn test_import_rooms_pseudo_flag() {
    // Pseudo rooms (Is Pseudo = Yes) are imported into the schedule so that
    // panels assigned to them can still be read, but they carry is_pseudo=true
    // so the export layer can exclude them from the public output.
    let mut book = umya_spreadsheet::new_file();

    {
        let ws = book.new_sheet("Rooms").unwrap();
        set_cell(ws, 1, 1, "Room Name");
        set_cell(ws, 2, 1, "Is Pseudo");
        set_cell(ws, 1, 2, "Panel Room 1"); // real room
        set_cell(ws, 1, 3, "SPLITDAY");
        set_cell(ws, 2, 3, "Yes"); // pseudo
        set_cell(ws, 1, 4, "SPLITNIGHT");
        set_cell(ws, 2, 4, "Yes"); // pseudo
    }

    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Name");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    // All three rooms are imported (pseudo rooms are not dropped at import time).
    assert_eq!(schedule.entity_count::<EventRoomEntityType>(), 3);

    // Only the real room has is_pseudo = false.
    let pseudo_count = schedule
        .iter_entities::<EventRoomEntityType>()
        .filter(|(_, d)| d.data.is_pseudo)
        .count();
    assert_eq!(pseudo_count, 2);
}

#[test]
fn test_import_panels_basic_fields() {
    let mut book = umya_spreadsheet::new_file();

    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        // Headers
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "Start Time");
        set_cell(ws, 4, 1, "Duration");
        set_cell(ws, 5, 1, "Description");
        set_cell(ws, 6, 1, "Cost");
        // Row 1
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "Opening Ceremony");
        set_cell(ws, 3, 2, "6/27/2026 10:00");
        set_cell(ws, 4, 2, "60");
        set_cell(ws, 5, 2, "Welcome everyone");
        set_cell(ws, 6, 2, "Free");
        // Row 2
        set_cell(ws, 1, 3, "FW001");
        set_cell(ws, 2, 3, "Cosplay Workshop");
        set_cell(ws, 3, 3, "6/27/2026 14:00");
        set_cell(ws, 4, 3, "90");
        set_cell(ws, 6, 3, "$35");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    assert_eq!(schedule.entity_count::<PanelEntityType>(), 2);

    let gp001 = schedule
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "GP001")
        .map(|(_, d)| d.clone())
        .expect("GP001 should exist");
    assert_eq!(gp001.data.name, "Opening Ceremony");
    assert_eq!(gp001.data.description.as_deref(), Some("Welcome everyone"));
    assert!(gp001.data.is_free);
    assert_eq!(
        gp001.time_slot.duration().map(|d| d.num_minutes()),
        Some(60)
    );

    let fw001 = schedule
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "FW001")
        .map(|(_, d)| d.clone())
        .expect("FW001 should exist");
    assert_eq!(fw001.data.cost.as_deref(), Some("$35"));
    assert!(!fw001.data.is_free);
}

#[test]
fn test_import_presenter_columns_tagged() {
    let mut book = umya_spreadsheet::new_file();

    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "G:Alice Example");
        set_cell(ws, 4, 1, "G:Bob Smith");
        set_cell(ws, 5, 1, "P:Other");

        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "Guest Panel");
        set_cell(ws, 3, 2, "Yes"); // Alice attending
                                   // Bob not attending (empty)
        set_cell(ws, 5, 2, "Jane Doe, John Fan");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    // Alice + Jane + John should all be created.
    assert!(schedule.entity_count::<PresenterEntityType>() >= 3);

    let alice_exists = schedule
        .iter_entities::<PresenterEntityType>()
        .any(|(_, d)| d.data.name == "Alice Example");
    assert!(alice_exists, "Alice Example should be a presenter");

    // Panel should have credited presenters edge to Alice.
    let gp001_id = schedule
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "GP001")
        .map(|(id, _)| id)
        .unwrap();

    let credited: Vec<_> = schedule
        .connected_entities::<PresenterEntityType>(gp001_id, panel::EDGE_CREDITED_PRESENTERS)
        .into_iter()
        .collect();
    assert!(
        !credited.is_empty(),
        "GP001 should have credited presenters"
    );
    let credited_names: Vec<_> = credited
        .iter()
        .filter_map(|id| schedule.get_internal::<PresenterEntityType>(*id))
        .map(|d| d.data.name.as_str())
        .collect();
    assert!(credited_names.contains(&"Alice Example"));
}

#[test]
fn test_import_panel_room_edge() {
    let mut book = umya_spreadsheet::new_file();

    {
        let ws = book.new_sheet("Rooms").unwrap();
        set_cell(ws, 1, 1, "Room Name");
        set_cell(ws, 1, 2, "Panel Room 1");
    }

    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "Room");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "Main Panel");
        set_cell(ws, 3, 2, "Panel Room 1");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    let gp001_id = schedule
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "GP001")
        .map(|(id, _)| id)
        .unwrap();

    let rooms: Vec<_> = schedule
        .connected_entities::<EventRoomEntityType>(gp001_id, panel::EDGE_EVENT_ROOMS)
        .into_iter()
        .collect();
    assert_eq!(rooms.len(), 1, "GP001 should be linked to one room");
}

#[test]
fn test_import_idempotent() {
    let mut book = umya_spreadsheet::new_file();

    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "A Panel");
    }

    let path = write_temp(book);

    let schedule1 = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    let schedule2 = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    // Same entity counts and same panel UUIDs.
    assert_eq!(
        schedule1.entity_count::<PanelEntityType>(),
        schedule2.entity_count::<PanelEntityType>()
    );

    let uuid1 = schedule1
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "GP001")
        .map(|(id, _)| id)
        .unwrap();
    let uuid2 = schedule2
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "GP001")
        .map(|(id, _)| id)
        .unwrap();

    use schedule_core::entity::EntityUuid;
    assert_eq!(
        uuid1.entity_uuid(),
        uuid2.entity_uuid(),
        "Same panel should get the same UUID on re-import"
    );
}

#[test]
fn test_import_soft_deleted_rows_skipped() {
    let mut book = umya_spreadsheet::new_file();

    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "Active Panel");
        set_cell(ws, 1, 3, "*GP002"); // soft-deleted
        set_cell(ws, 2, 3, "Deleted Panel");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    assert_eq!(schedule.entity_count::<PanelEntityType>(), 1);
    let exists = schedule
        .iter_entities::<PanelEntityType>()
        .any(|(_, d)| d.code.full_id() == "GP001");
    assert!(exists);
}

#[test]
fn test_import_people_sheet_creates_presenters_with_rank() {
    let mut book = umya_spreadsheet::new_file();

    // People sheet
    {
        let ws = book.new_sheet("People").unwrap();
        set_cell(ws, 1, 1, "Person");
        set_cell(ws, 2, 1, "Classification");
        set_cell(ws, 3, 1, "Is Group");

        set_cell(ws, 1, 2, "Alice Example");
        set_cell(ws, 2, 2, "Guest");

        set_cell(ws, 1, 3, "UNC Staff");
        set_cell(ws, 2, 3, "Staff");
        set_cell(ws, 3, 3, "Yes"); // is_explicit_group
    }

    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Name");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    assert_eq!(schedule.entity_count::<PresenterEntityType>(), 2);

    let alice = schedule
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Alice Example")
        .map(|(_, d)| d.data.clone())
        .expect("Alice should exist");
    assert_eq!(
        alice.rank,
        schedule_core::tables::presenter::PresenterRank::Guest
    );

    let unc = schedule
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "UNC Staff")
        .map(|(_, d)| d.data.clone())
        .expect("UNC Staff should exist");
    assert_eq!(
        unc.rank,
        schedule_core::tables::presenter::PresenterRank::Staff
    );
    assert!(unc.is_explicit_group);
}

#[test]
fn test_import_people_rank_upgraded_by_schedule_presenter_column() {
    // Presenter appears in both People (as Panelist) and a Guest column on Schedule.
    // People table is read first; Schedule column's find_or_create_tagged_presenter
    // should upgrade the rank because Guest has higher priority than Panelist.
    let mut book = umya_spreadsheet::new_file();

    {
        let ws = book.new_sheet("People").unwrap();
        set_cell(ws, 1, 1, "Person");
        set_cell(ws, 2, 1, "Classification");
        set_cell(ws, 1, 2, "Jane Smith");
        set_cell(ws, 2, 2, "Panelist");
    }

    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "G:Jane Smith"); // Guest column — higher rank
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "A Panel");
        set_cell(ws, 3, 2, "Yes");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    // Jane should exist exactly once with Guest rank (upgraded by schedule column).
    let janes: Vec<_> = schedule
        .iter_entities::<PresenterEntityType>()
        .filter(|(_, d)| d.data.name == "Jane Smith")
        .collect();
    assert_eq!(janes.len(), 1, "Jane should appear exactly once");
    assert_eq!(
        janes[0].1.data.rank,
        schedule_core::tables::presenter::PresenterRank::Guest
    );
}

// ── Export helpers ────────────────────────────────────────────────────────────

fn export_to_temp(schedule: &schedule_core::schedule::Schedule) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tid = format!("{:?}", std::thread::current().id());
    let tid_hash: u64 = tid
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let path = std::env::temp_dir().join(format!("cosam_export_test_{nanos}_{tid_hash}.xlsx"));
    export_xlsx(schedule, &path).expect("export_xlsx should succeed");
    path
}

fn round_trip(schedule: schedule_core::schedule::Schedule) -> schedule_core::schedule::Schedule {
    let path = export_to_temp(&schedule);
    let reimported = import_xlsx(&path, &XlsxImportOptions::default())
        .expect("re-import of exported file should succeed");
    cleanup(&path);
    reimported
}

// ── Export / round-trip tests (FEATURE-029) ───────────────────────────────────

#[test]
fn test_export_round_trip_panel_types() {
    let mut book = umya_spreadsheet::new_file();
    {
        let ws = book.new_sheet("PanelTypes").unwrap();
        set_cell(ws, 1, 1, "Prefix");
        set_cell(ws, 2, 1, "Panel Kind");
        set_cell(ws, 3, 1, "Is Workshop");
        set_cell(ws, 4, 1, "Color");
        set_cell(ws, 1, 2, "GP");
        set_cell(ws, 2, 2, "Guest Panel");
        set_cell(ws, 4, 2, "#FF0000");
        set_cell(ws, 1, 3, "FW");
        set_cell(ws, 2, 3, "Fan Workshop");
        set_cell(ws, 3, 3, "Yes");
    }
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Name");
    }
    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    let rt = round_trip(schedule);

    assert_eq!(rt.entity_count::<PanelTypeEntityType>(), 2);
    let gp = rt
        .iter_entities::<PanelTypeEntityType>()
        .find(|(_, d)| d.data.prefix == "GP")
        .map(|(_, d)| d.data.clone())
        .expect("GP should survive round-trip");
    assert_eq!(gp.panel_kind, "Guest Panel");
    assert!(!gp.is_workshop);
    assert_eq!(gp.color.as_deref(), Some("#FF0000"));

    let fw = rt
        .iter_entities::<PanelTypeEntityType>()
        .find(|(_, d)| d.data.prefix == "FW")
        .map(|(_, d)| d.data.clone())
        .expect("FW should survive round-trip");
    assert!(fw.is_workshop);
}

#[test]
fn test_export_round_trip_rooms() {
    let mut book = umya_spreadsheet::new_file();
    {
        let ws = book.new_sheet("Rooms").unwrap();
        set_cell(ws, 1, 1, "Room Name");
        set_cell(ws, 2, 1, "Sort Key");
        set_cell(ws, 3, 1, "Long Name");
        set_cell(ws, 4, 1, "Hotel Room");
        set_cell(ws, 1, 2, "Ballroom A");
        set_cell(ws, 2, 2, "10");
        set_cell(ws, 3, 2, "Main Ballroom");
        set_cell(ws, 4, 2, "Grand Hotel");
    }
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Name");
    }
    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    let rt = round_trip(schedule);

    assert_eq!(rt.entity_count::<EventRoomEntityType>(), 1);
    let room = rt
        .iter_entities::<EventRoomEntityType>()
        .find(|(_, d)| d.data.room_name == "Ballroom A")
        .map(|(id, d)| (id, d.data.clone()))
        .expect("Ballroom A should survive round-trip");
    assert_eq!(room.1.long_name.as_deref(), Some("Main Ballroom"));
    assert_eq!(room.1.sort_key, Some(10));

    // Hotel room link should survive.
    let hotel_ids =
        rt.connected_entities::<HotelRoomEntityType>(room.0, event_room::EDGE_HOTEL_ROOMS);
    assert_eq!(
        hotel_ids.len(),
        1,
        "hotel room link should survive round-trip"
    );
    let hotel = rt
        .get_internal::<HotelRoomEntityType>(hotel_ids[0])
        .expect("hotel room entity should exist");
    assert_eq!(hotel.data.hotel_room_name, "Grand Hotel");
}

#[test]
fn test_export_round_trip_panels() {
    let mut book = umya_spreadsheet::new_file();
    {
        let ws = book.new_sheet("PanelTypes").unwrap();
        set_cell(ws, 1, 1, "Prefix");
        set_cell(ws, 2, 1, "Panel Kind");
        set_cell(ws, 1, 2, "GP");
        set_cell(ws, 2, 2, "Guest Panel");
    }
    {
        let ws = book.new_sheet("Rooms").unwrap();
        set_cell(ws, 1, 1, "Room Name");
        set_cell(ws, 1, 2, "Main Hall");
    }
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "Start Time");
        set_cell(ws, 4, 1, "Duration");
        set_cell(ws, 5, 1, "Room");
        set_cell(ws, 6, 1, "Description");
        set_cell(ws, 7, 1, "Note");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "Opening Ceremony");
        set_cell(ws, 3, 2, "2026-06-26T10:00:00");
        set_cell(ws, 4, 2, "60");
        set_cell(ws, 5, 2, "Main Hall");
        set_cell(ws, 6, 2, "Welcome to the con");
        set_cell(ws, 7, 2, "A note");
    }
    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    let rt = round_trip(schedule);

    assert_eq!(rt.entity_count::<PanelEntityType>(), 1);
    let panel = rt
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "GP001")
        .map(|(id, d)| (id, d.clone()))
        .expect("GP001 should survive round-trip");
    assert_eq!(panel.1.data.name, "Opening Ceremony");
    assert_eq!(
        panel.1.data.description.as_deref(),
        Some("Welcome to the con")
    );
    assert_eq!(panel.1.data.note.as_deref(), Some("A note"));
    assert_eq!(
        panel.1.time_slot.duration().map(|d| d.num_minutes()),
        Some(60)
    );
    assert!(
        panel.1.time_slot.start_time().is_some(),
        "start time should survive round-trip"
    );

    // Room link should survive.
    let room_ids = rt.connected_entities::<EventRoomEntityType>(panel.0, panel::EDGE_EVENT_ROOMS);
    assert_eq!(room_ids.len(), 1, "room link should survive round-trip");
    let room = rt
        .get_internal::<EventRoomEntityType>(room_ids[0])
        .expect("room entity should exist");
    assert_eq!(room.data.room_name, "Main Hall");
}

#[test]
fn test_export_round_trip_presenters() {
    // Alice has 3 panels (gets a named column), Bob has 1 panel (goes to Other).
    let mut book = umya_spreadsheet::new_file();
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "G:Alice Example");
        set_cell(ws, 4, 1, "G:Bob Smith");

        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "Panel One");
        set_cell(ws, 3, 2, "Yes");
        set_cell(ws, 4, 2, "Yes");

        set_cell(ws, 1, 3, "GP002");
        set_cell(ws, 2, 3, "Panel Two");
        set_cell(ws, 3, 3, "Yes");

        set_cell(ws, 1, 4, "GP003");
        set_cell(ws, 2, 4, "Panel Three");
        set_cell(ws, 3, 4, "Yes");
    }
    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    let rt = round_trip(schedule);

    assert_eq!(rt.entity_count::<PanelEntityType>(), 3);
    assert!(rt.entity_count::<PresenterEntityType>() >= 2);

    // Alice should still be credited on all three panels after round-trip.
    let alice_id = rt
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Alice Example")
        .map(|(id, _)| id)
        .expect("Alice should survive round-trip");

    let alice_panels =
        rt.connected_entities::<PanelEntityType>(alice_id, presenter::EDGE_CREDITED_PANELS);
    assert_eq!(
        alice_panels.len(),
        3,
        "Alice should be credited on all three panels after round-trip"
    );

    // Bob should survive round-trip even going through an Other column.
    let bob_exists = rt
        .iter_entities::<PresenterEntityType>()
        .any(|(_, d)| d.data.name == "Bob Smith");
    assert!(bob_exists, "Bob should survive round-trip via Other column");
}

#[test]
fn test_export_round_trip_people_sheet() {
    let mut book = umya_spreadsheet::new_file();
    {
        let ws = book.new_sheet("People").unwrap();
        set_cell(ws, 1, 1, "Person");
        set_cell(ws, 2, 1, "Classification");
        set_cell(ws, 3, 1, "Is Group");
        set_cell(ws, 4, 1, "Always Grouped");
        set_cell(ws, 5, 1, "Always Shown");

        set_cell(ws, 1, 2, "Alice Example");
        set_cell(ws, 2, 2, "Guest");

        set_cell(ws, 1, 3, "Fan Club");
        set_cell(ws, 2, 3, "Panelist");
        set_cell(ws, 3, 3, "Yes");

        set_cell(ws, 1, 4, "Bob Fan");
        set_cell(ws, 2, 4, "Fan Panelist");
        set_cell(ws, 4, 4, "Yes"); // always_grouped
    }
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Name");
    }
    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    let rt = round_trip(schedule);

    assert_eq!(rt.entity_count::<PresenterEntityType>(), 3);

    let alice = rt
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Alice Example")
        .map(|(_, d)| d.data.clone())
        .expect("Alice should survive round-trip");
    assert_eq!(
        alice.rank,
        schedule_core::tables::presenter::PresenterRank::Guest
    );

    let fan_club = rt
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Fan Club")
        .map(|(_, d)| d.data.clone())
        .expect("Fan Club should survive round-trip");
    assert!(fan_club.is_explicit_group);

    let bob = rt
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Bob Fan")
        .map(|(_, d)| d.data.clone())
        .expect("Bob Fan should survive round-trip");
    assert_eq!(
        bob.rank,
        schedule_core::tables::presenter::PresenterRank::FanPanelist
    );
    assert!(bob.always_grouped);
}
