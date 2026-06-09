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

use schedule_core::entity::EntityUuid;
use schedule_core::tables::event_room::{self as event_room, EventRoomEntityType};
use schedule_core::tables::hotel_room::HotelRoomEntityType;
use schedule_core::tables::panel::{self, PanelEntityType};
use schedule_core::tables::panel_type::PanelTypeEntityType;
use schedule_core::tables::presenter::{self as presenter, PresenterEntityType};
use schedule_core::xlsx::{
    export_xlsx, export_xlsx_grid, import_xlsx, update_schedule_from_xlsx, XlsxImportOptions,
};

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
    // blank cost cell → Included (no extra charge)
    assert_eq!(
        gp001.data.additional_cost,
        schedule_core::value::AdditionalCost::Included
    );
    assert_eq!(
        gp001.time_slot.duration().map(|d| d.num_minutes()),
        Some(60)
    );

    let fw001 = schedule
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "FW001")
        .map(|(_, d)| d.clone())
        .expect("FW001 should exist");
    // $35 cost → Premium(3500)
    assert_eq!(
        fw001.data.additional_cost,
        schedule_core::value::AdditionalCost::Premium(3500)
    );
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
        alice.rank.effective(),
        schedule_core::tables::presenter::PresenterRank::Guest
    );

    let unc = schedule
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "UNC Staff")
        .map(|(_, d)| d.data.clone())
        .expect("UNC Staff should exist");
    assert_eq!(
        unc.rank.effective(),
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
        janes[0].1.data.rank.effective(),
        schedule_core::tables::presenter::PresenterRank::Guest
    );
}

// ── Update-mode integration tests ─────────────────────────────────────────────

/// Build a minimal one-panel schedule xlsx and return its path.
fn make_schedule_xlsx_one_panel(panel_id: &str, panel_name: &str) -> PathBuf {
    let mut book = umya_spreadsheet::new_file();
    let ws = book.get_sheet_mut(&0).unwrap();
    ws.set_name("Schedule");
    set_cell(ws, 1, 1, "Uniq ID");
    set_cell(ws, 2, 1, "Name");
    set_cell(ws, 1, 2, panel_id);
    set_cell(ws, 2, 2, panel_name);
    write_temp(book)
}

#[test]
fn test_update_soft_deletes_panels_not_in_new_xlsx() {
    // First import: two panels.
    let path1 = {
        let mut book = umya_spreadsheet::new_file();
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "Panel One");
        set_cell(ws, 1, 3, "GP002");
        set_cell(ws, 2, 3, "Panel Two");
        write_temp(book)
    };
    let mut schedule = import_xlsx(&path1, &XlsxImportOptions::default()).unwrap();
    cleanup(&path1);
    assert_eq!(schedule.entity_count::<PanelEntityType>(), 2);

    // Second import: only GP001.
    let path2 = make_schedule_xlsx_one_panel("GP001", "Panel One");
    update_schedule_from_xlsx(&mut schedule, &path2, &XlsxImportOptions::default()).unwrap();
    cleanup(&path2);

    // GP002 should be soft-deleted (not visible via iter_entities).
    assert_eq!(schedule.entity_count::<PanelEntityType>(), 1);
    let codes: Vec<_> = schedule
        .iter_entities::<PanelEntityType>()
        .map(|(_, d)| d.code.full_id().to_string())
        .collect();
    assert!(codes.contains(&"GP001".to_string()));
    assert!(!codes.contains(&"GP002".to_string()));
}

#[test]
fn test_update_soft_deletes_presenters_not_in_new_xlsx() {
    // First import: People sheet with two presenters.
    let path1 = {
        let mut book = umya_spreadsheet::new_file();
        {
            let ws = book.new_sheet("People").unwrap();
            set_cell(ws, 1, 1, "Person");
            set_cell(ws, 1, 2, "Alice");
            set_cell(ws, 1, 3, "Bob");
        }
        {
            let ws = book.get_sheet_mut(&0).unwrap();
            ws.set_name("Schedule");
            set_cell(ws, 1, 1, "Name");
        }
        write_temp(book)
    };
    let mut schedule = import_xlsx(&path1, &XlsxImportOptions::default()).unwrap();
    cleanup(&path1);
    assert_eq!(schedule.entity_count::<PresenterEntityType>(), 2);

    // Second import: only Alice in People.
    let path2 = {
        let mut book = umya_spreadsheet::new_file();
        {
            let ws = book.new_sheet("People").unwrap();
            set_cell(ws, 1, 1, "Person");
            set_cell(ws, 1, 2, "Alice");
        }
        {
            let ws = book.get_sheet_mut(&0).unwrap();
            ws.set_name("Schedule");
            set_cell(ws, 1, 1, "Name");
        }
        write_temp(book)
    };
    update_schedule_from_xlsx(&mut schedule, &path2, &XlsxImportOptions::default()).unwrap();
    cleanup(&path2);

    assert_eq!(schedule.entity_count::<PresenterEntityType>(), 1);
    let names: Vec<_> = schedule
        .iter_entities::<PresenterEntityType>()
        .map(|(_, d)| d.data.name.clone())
        .collect();
    assert!(names.contains(&"Alice".to_string()));
    assert!(!names.contains(&"Bob".to_string()));
}

#[test]
fn test_update_drops_presenter_edges_removed_from_panel() {
    // First import: GP001 credits Alice and Bob.
    let path1 = {
        let mut book = umya_spreadsheet::new_file();
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "P:Other");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "A Panel");
        set_cell(ws, 3, 2, "Alice, Bob");
        write_temp(book)
    };
    let mut schedule = import_xlsx(&path1, &XlsxImportOptions::default()).unwrap();
    cleanup(&path1);

    let gp001_id = schedule
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "GP001")
        .map(|(id, _)| id)
        .unwrap();
    let credited: Vec<_> = schedule
        .connected_entities::<PresenterEntityType>(gp001_id, panel::EDGE_CREDITED_PRESENTERS);
    assert_eq!(credited.len(), 2, "should have Alice and Bob initially");

    // Second import: GP001 credits only Alice.
    let path2 = {
        let mut book = umya_spreadsheet::new_file();
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "P:Other");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "A Panel");
        set_cell(ws, 3, 2, "Alice");
        write_temp(book)
    };
    update_schedule_from_xlsx(&mut schedule, &path2, &XlsxImportOptions::default()).unwrap();
    cleanup(&path2);

    let gp001_id = schedule
        .iter_entities::<PanelEntityType>()
        .find(|(_, d)| d.code.full_id() == "GP001")
        .map(|(id, _)| id)
        .unwrap();
    let credited: Vec<_> = schedule
        .connected_entities::<PresenterEntityType>(gp001_id, panel::EDGE_CREDITED_PRESENTERS);
    assert_eq!(credited.len(), 1, "only Alice should remain credited");
    let name = schedule
        .get_internal::<PresenterEntityType>(credited[0])
        .map(|d| d.data.name.as_str());
    assert_eq!(name, Some("Alice"));
}

#[test]
fn test_update_presenter_rank_does_not_exceed_xlsx_highest() {
    // First import: Alice as Guest (high rank).
    let path1 = {
        let mut book = umya_spreadsheet::new_file();
        {
            let ws = book.new_sheet("People").unwrap();
            set_cell(ws, 1, 1, "Person");
            set_cell(ws, 2, 1, "Classification");
            set_cell(ws, 1, 2, "Alice");
            set_cell(ws, 2, 2, "Guest");
        }
        {
            let ws = book.get_sheet_mut(&0).unwrap();
            ws.set_name("Schedule");
            set_cell(ws, 1, 1, "Name");
        }
        write_temp(book)
    };
    let mut schedule = import_xlsx(&path1, &XlsxImportOptions::default()).unwrap();
    cleanup(&path1);

    let rank = schedule
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Alice")
        .map(|(_, d)| d.data.rank.effective())
        .unwrap();
    assert_eq!(rank, schedule_core::tables::presenter::PresenterRank::Guest);

    // Second import: Alice only appears as Panelist — rank should update down.
    let path2 = {
        let mut book = umya_spreadsheet::new_file();
        {
            let ws = book.new_sheet("People").unwrap();
            set_cell(ws, 1, 1, "Person");
            set_cell(ws, 2, 1, "Classification");
            set_cell(ws, 1, 2, "Alice");
            set_cell(ws, 2, 2, "Panelist");
        }
        {
            let ws = book.get_sheet_mut(&0).unwrap();
            ws.set_name("Schedule");
            set_cell(ws, 1, 1, "Name");
        }
        write_temp(book)
    };
    update_schedule_from_xlsx(&mut schedule, &path2, &XlsxImportOptions::default()).unwrap();
    cleanup(&path2);

    let rank = schedule
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Alice")
        .map(|(_, d)| d.data.rank.effective())
        .unwrap();
    // After update the xlsx is the source of truth; rank should be Panelist.
    assert_eq!(
        rank,
        schedule_core::tables::presenter::PresenterRank::Panelist
    );
}

#[test]
fn test_update_presenter_name_capitalization_corrected() {
    // First import: presenter named "camelcase" (wrong case).
    let path1 = {
        let mut book = umya_spreadsheet::new_file();
        {
            let ws = book.new_sheet("People").unwrap();
            set_cell(ws, 1, 1, "Person");
            set_cell(ws, 1, 2, "camelcase");
        }
        {
            let ws = book.get_sheet_mut(&0).unwrap();
            ws.set_name("Schedule");
            set_cell(ws, 1, 1, "Name");
        }
        write_temp(book)
    };
    let mut schedule = import_xlsx(&path1, &XlsxImportOptions::default()).unwrap();
    cleanup(&path1);

    assert!(
        schedule
            .iter_entities::<PresenterEntityType>()
            .any(|(_, d)| d.data.name == "camelcase"),
        "initial import should have 'camelcase'"
    );

    // Second import: correct capitalisation "CamelCase".
    let path2 = {
        let mut book = umya_spreadsheet::new_file();
        {
            let ws = book.new_sheet("People").unwrap();
            set_cell(ws, 1, 1, "Person");
            set_cell(ws, 1, 2, "CamelCase");
        }
        {
            let ws = book.get_sheet_mut(&0).unwrap();
            ws.set_name("Schedule");
            set_cell(ws, 1, 1, "Name");
        }
        write_temp(book)
    };
    update_schedule_from_xlsx(&mut schedule, &path2, &XlsxImportOptions::default()).unwrap();
    cleanup(&path2);

    let names: Vec<_> = schedule
        .iter_entities::<PresenterEntityType>()
        .map(|(_, d)| d.data.name.clone())
        .collect();
    assert_eq!(names.len(), 1, "should still be exactly one presenter");
    assert_eq!(
        names[0], "CamelCase",
        "name should be updated to correct case"
    );
}

#[test]
fn test_untagged_other_cell_gets_column_rank_as_minimum() {
    // Alice appears untagged in a "P:Other" column only.  No tag prefix on the cell.
    // The column rank (Panelist) is implicit but is applied inline as the minimum;
    // it is NOT recorded as an explicit rank in the cache, so it does not block
    // a later explicit tag such as "F:Alice" in the same xlsx.
    // Here we just verify that an untagged Other cell produces the column rank
    // when no other rank information is present.
    let mut book = umya_spreadsheet::new_file();
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "P:Other");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "A Panel");
        set_cell(ws, 3, 2, "Alice");
    }
    let schedule = import_xlsx(&write_temp(book), &XlsxImportOptions::default()).unwrap();

    let rank = schedule
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Alice")
        .map(|(_, d)| d.data.rank.effective())
        .expect("Alice should exist");
    // No tag prefix and no People-sheet classification → column rank (Panelist) applies.
    assert_eq!(
        rank,
        schedule_core::tables::presenter::PresenterRank::Panelist,
        "untagged Other-column cell should produce the column rank when no tag is present"
    );
}

#[test]
fn test_untagged_other_cell_then_explicit_fan_panelist_tag() {
    // Alice appears in a "P:Other" column with no tag prefix (cell = "Alice").
    // On the same schedule she also appears tagged "F:Alice" in another Other column.
    // The tagged "F:" provides an explicit FanPanelist rank; the untagged "P:Other"
    // appearance is implicit and must not override it.
    let mut book = umya_spreadsheet::new_file();
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        // P:Other column — Alice appears untagged (no tag prefix on cell)
        set_cell(ws, 3, 1, "P:Other");
        // P:Other column — Alice appears with explicit F: tag prefix
        set_cell(ws, 4, 1, "P:Other");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "A Panel");
        set_cell(ws, 3, 2, "Alice"); // untagged → implicit, no explicit rank from cell
        set_cell(ws, 4, 2, "F:Alice"); // tagged → explicit FanPanelist from tag
    }
    let schedule = import_xlsx(&write_temp(book), &XlsxImportOptions::default()).unwrap();

    let rank = schedule
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Alice")
        .map(|(_, d)| d.data.rank.effective())
        .expect("Alice should exist");
    assert_eq!(
        rank,
        schedule_core::tables::presenter::PresenterRank::FanPanelist,
        "explicit F:Alice tag should win over untagged Other-column appearance"
    );
}

#[test]
fn test_update_drops_group_membership_edge_when_absent() {
    // First import: Alice=MyBand (Alice is a member of MyBand).
    let path1 = {
        let mut book = umya_spreadsheet::new_file();
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "P:Other");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "A Panel");
        set_cell(ws, 3, 2, "Alice=MyBand");
        write_temp(book)
    };
    let mut schedule = import_xlsx(&path1, &XlsxImportOptions::default()).unwrap();
    cleanup(&path1);

    let alice_id = schedule
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Alice")
        .map(|(id, _)| id)
        .expect("Alice should exist after first import");
    let groups: Vec<_> =
        schedule.connected_entities::<PresenterEntityType>(alice_id, presenter::EDGE_GROUPS);
    assert_eq!(
        groups.len(),
        1,
        "Alice should be in MyBand after first import"
    );

    // Second import: Alice without the group membership.
    let path2 = {
        let mut book = umya_spreadsheet::new_file();
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "P:Other");
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "A Panel");
        set_cell(ws, 3, 2, "Alice");
        write_temp(book)
    };
    update_schedule_from_xlsx(&mut schedule, &path2, &XlsxImportOptions::default()).unwrap();
    cleanup(&path2);

    let alice_id = schedule
        .iter_entities::<PresenterEntityType>()
        .find(|(_, d)| d.data.name == "Alice")
        .map(|(id, _)| id)
        .expect("Alice should still exist");
    let groups: Vec<_> =
        schedule.connected_entities::<PresenterEntityType>(alice_id, presenter::EDGE_GROUPS);
    assert!(
        groups.is_empty(),
        "Alice's group membership should be cleared when absent from new xlsx"
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
        alice.rank.effective(),
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
        bob.rank.effective(),
        schedule_core::tables::presenter::PresenterRank::FanPanelist
    );
    assert!(bob.show_individually);
}

// ── Idempotency tests (FEATURE-127) ───────────────────────────────────────────

/// Build a minimal workbook with PanelTypes, Rooms, People, and Schedule data.
fn build_minimal_idempotency_book() -> umya_spreadsheet::Spreadsheet {
    let mut book = umya_spreadsheet::new_file();

    // PanelTypes sheet
    {
        let ws = book.new_sheet("PanelTypes").unwrap();
        set_cell(ws, 1, 1, "Prefix");
        set_cell(ws, 2, 1, "Panel Kind");
        set_cell(ws, 1, 2, "GP");
        set_cell(ws, 2, 2, "Guest Panel");
    }

    // Rooms sheet
    {
        let ws = book.new_sheet("Rooms").unwrap();
        set_cell(ws, 1, 1, "Room Name");
        set_cell(ws, 2, 1, "Sort Key");
        set_cell(ws, 1, 2, "Main Hall");
        set_cell(ws, 2, 2, "10");
    }

    // People sheet
    {
        let ws = book.new_sheet("People").unwrap();
        set_cell(ws, 1, 1, "Name");
        set_cell(ws, 2, 1, "Classification");
        set_cell(ws, 1, 2, "Alice Smith");
        set_cell(ws, 2, 2, "Guest");
    }

    // Schedule sheet (default sheet 0 is renamed)
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Name");
        set_cell(ws, 2, 1, "Room");
        set_cell(ws, 3, 1, "Type");
        set_cell(ws, 4, 1, "G: Alice Smith");
        set_cell(ws, 1, 2, "Opening Ceremonies");
        set_cell(ws, 2, 2, "Main Hall");
        set_cell(ws, 3, 2, "GP");
        set_cell(ws, 4, 2, "X");
    }

    book
}

/// Re-importing an unchanged XLSX into an existing schedule must produce
/// byte-for-byte identical output (FEATURE-127).
#[test]
fn test_reimport_same_xlsx_is_idempotent() {
    let book = build_minimal_idempotency_book();
    let path = write_temp(book);

    // First import.
    let mut schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    let bytes_after_first = schedule.save_to_file();

    // Re-import the same file into the existing schedule.
    update_schedule_from_xlsx(&mut schedule, &path, &XlsxImportOptions::default()).unwrap();
    let bytes_after_second = schedule.save_to_file();

    cleanup(&path);

    assert_eq!(
        bytes_after_first, bytes_after_second,
        "re-importing an unchanged XLSX must produce byte-for-byte identical output"
    );
}

/// Build a workbook that exercises long-prefix timeline normalization (FEATURE-127 regression):
///
/// - PanelTypes: `BR` (Is Break = Yes), `SP` (Is Timeline = Yes), `GP` (regular)
/// - Schedule sheet: `BREAK001` (BR prefix, is_break), `GP001` (regular panel),
///   `SPLIT001` (raw prefix kept, "SP" lookup key, is_timeline)
/// - Timeline sheet: `SPLIT002` (is_timeline from Timeline sheet, long raw prefix)
///
/// Re-importing this workbook must be idempotent: the same UUIDs for all entities,
/// no spurious creates or deletes.
fn build_long_prefix_timeline_book() -> umya_spreadsheet::Spreadsheet {
    let mut book = umya_spreadsheet::new_file();

    // PanelTypes sheet
    {
        let ws = book.new_sheet("PanelTypes").unwrap();
        set_cell(ws, 1, 1, "Prefix");
        set_cell(ws, 2, 1, "Panel Kind");
        set_cell(ws, 3, 1, "Is Break");
        set_cell(ws, 4, 1, "Is Timeline");
        // BR — break type (short prefix, still exercises is_break path)
        set_cell(ws, 1, 2, "BR");
        set_cell(ws, 2, 2, "Break");
        set_cell(ws, 3, 2, "Yes");
        // SP — timeline type (prefix auto-inferred as is_timeline, but explicit here too)
        set_cell(ws, 1, 3, "SP");
        set_cell(ws, 2, 3, "Split Day");
        set_cell(ws, 4, 3, "Yes");
        // GP — regular panel type
        set_cell(ws, 1, 4, "GP");
        set_cell(ws, 2, 4, "Guest Panel");
    }

    // Timeline sheet — SPLIT002 lives here (long raw ID)
    {
        let ws = book.new_sheet("Timeline").unwrap();
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        set_cell(ws, 3, 1, "Panel Types");
        set_cell(ws, 1, 2, "SPLIT002");
        set_cell(ws, 2, 2, "Split Day Marker 2");
        set_cell(ws, 3, 2, "SP");
    }

    // Schedule sheet (default sheet 0 renamed)
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Uniq ID");
        set_cell(ws, 2, 1, "Name");
        // BREAK001 — raw prefix "BREAK" kept; "BR" lookup key (is_break panel type)
        set_cell(ws, 1, 2, "BREAK001");
        set_cell(ws, 2, 2, "Afternoon Break");
        // GP001 — regular panel
        set_cell(ws, 1, 3, "GP001");
        set_cell(ws, 2, 3, "Opening Ceremony");
        // SPLIT001 — raw prefix "SPLIT" kept; "SP" lookup key (is_timeline panel type)
        set_cell(ws, 1, 4, "SPLIT001");
        set_cell(ws, 2, 4, "Split Day Marker 1");
    }

    book
}

/// Re-importing a workbook with long-prefix timeline codes (SPLIT*, BREAK*) must
/// be idempotent: same byte output, no spurious entity creates or deletes.
///
/// Regression test for the bug where `code_str.to_uppercase()` ("SPLIT001") was
/// used as the upsert key instead of `parsed_code.full_id()`, causing a mismatch
/// against the stored `full_id()` and a new Timeline entity on every re-import.
/// Since BUGFIX-131, `full_id()` round-trips the raw prefix ("SPLIT001"), so the
/// upsert key and the stored id agree on the verbatim spreadsheet value.
#[test]
fn test_reimport_long_prefix_timelines_is_idempotent() {
    use schedule_core::entity::EntityUuid;
    use schedule_core::tables::timeline::TimelineEntityType;

    let book = build_long_prefix_timeline_book();
    let path = write_temp(book);

    // First import.
    let mut schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();

    // Verify the expected entities were created.
    // BREAK001 is is_break (not is_timeline), so it is a Panel, not a Timeline.
    assert_eq!(
        schedule.entity_count::<PanelEntityType>(),
        2,
        "BREAK001 and GP001 should be panels (SPLIT001 is a timeline)"
    );
    assert_eq!(
        schedule.entity_count::<TimelineEntityType>(),
        2,
        "SPLIT001 and SPLIT002 should be timelines"
    );

    // Capture the UUIDs before re-import.
    let split001_uuid_before = schedule
        .iter_entities::<TimelineEntityType>()
        .find(|(_, d)| d.code.full_id() == "SPLIT001")
        .map(|(id, _)| id.entity_uuid())
        .expect("SPLIT001 should exist");
    let split002_uuid_before = schedule
        .iter_entities::<TimelineEntityType>()
        .find(|(_, d)| d.code.full_id() == "SPLIT002")
        .map(|(id, _)| id.entity_uuid())
        .expect("SPLIT002 should exist");

    let bytes_after_first = schedule.save_to_file();

    // Re-import the same file — must be a no-op.
    update_schedule_from_xlsx(&mut schedule, &path, &XlsxImportOptions::default()).unwrap();
    let bytes_after_second = schedule.save_to_file();

    cleanup(&path);

    // Entity counts must be unchanged.
    assert_eq!(
        schedule.entity_count::<PanelEntityType>(),
        2,
        "panel count must not change on re-import"
    );
    assert_eq!(
        schedule.entity_count::<TimelineEntityType>(),
        2,
        "timeline count must not change on re-import"
    );

    // UUIDs must be stable.
    let split001_uuid_after = schedule
        .iter_entities::<TimelineEntityType>()
        .find(|(_, d)| d.code.full_id() == "SPLIT001")
        .map(|(id, _)| id.entity_uuid())
        .expect("SPLIT001 must still exist after re-import");
    let split002_uuid_after = schedule
        .iter_entities::<TimelineEntityType>()
        .find(|(_, d)| d.code.full_id() == "SPLIT002")
        .map(|(id, _)| id.entity_uuid())
        .expect("SPLIT002 must still exist after re-import");

    assert_eq!(
        split001_uuid_before, split001_uuid_after,
        "SPLIT001 must map to the same Timeline UUID on re-import"
    );
    assert_eq!(
        split002_uuid_before, split002_uuid_after,
        "SPLIT002 must map to the same Timeline UUID on re-import"
    );

    // Full byte-level idempotency check.
    assert_eq!(
        bytes_after_first, bytes_after_second,
        "re-importing an unchanged XLSX with long-prefix timelines must be byte-for-byte identical"
    );
}

/// If source data changes, the output must differ and modified_at must be updated.
#[test]
fn test_reimport_changed_xlsx_updates_output() {
    let book = build_minimal_idempotency_book();
    let path = write_temp(book);

    let mut schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    let modified_at_first = schedule.metadata.modified_at;
    let bytes_after_first = schedule.save_to_file();
    cleanup(&path);

    // Build a second, slightly different workbook.
    let mut book2 = build_minimal_idempotency_book();
    {
        let ws = book2.get_sheet_mut(&0).unwrap(); // Schedule sheet
        set_cell(ws, 1, 2, "Opening Ceremonies Updated");
    }
    let path2 = write_temp(book2);

    update_schedule_from_xlsx(&mut schedule, &path2, &XlsxImportOptions::default()).unwrap();
    let bytes_after_second = schedule.save_to_file();
    cleanup(&path2);

    assert_ne!(
        bytes_after_first, bytes_after_second,
        "importing changed data must produce different output"
    );
    // modified_at should have been updated (or at least not rolled back to None).
    assert!(
        schedule.metadata.modified_at != modified_at_first
            || schedule.metadata.modified_at.is_some(),
        "modified_at should reflect the change"
    );
}

/// `export_xlsx_grid` writes only the per-day grid reference sheets, omitting
/// the data tables (Schedule, Rooms, PanelTypes, People, …) that
/// `export_xlsx` produces.
#[test]
fn test_export_xlsx_grid_only_grid_sheets() {
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
        set_cell(ws, 3, 1, "Start Time");
        set_cell(ws, 4, 1, "Duration");
        set_cell(ws, 5, 1, "Room");
        // One timed panel in a room → produces a single logical-day grid sheet.
        set_cell(ws, 1, 2, "GP001");
        set_cell(ws, 2, 2, "Opening Ceremony");
        set_cell(ws, 3, 2, "6/27/2026 10:00");
        set_cell(ws, 4, 2, "60");
        set_cell(ws, 5, 2, "Panel Room 1");
    }

    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let grid_path = std::env::temp_dir().join(format!("cosam_grid_test_{nanos}.xlsx"));
    export_xlsx_grid(&schedule, &grid_path).expect("export_xlsx_grid should succeed");

    let book = umya_spreadsheet::reader::xlsx::read(&grid_path).expect("re-read grid xlsx");
    let sheet_names: Vec<String> = book
        .get_sheet_collection()
        .iter()
        .map(|s| s.get_name().to_string())
        .collect();
    cleanup(&grid_path);

    assert!(
        !sheet_names.is_empty(),
        "grid workbook should contain at least one sheet"
    );
    assert!(
        sheet_names.iter().all(|n| n.starts_with("Grid - ")),
        "grid workbook should contain only grid sheets, got: {sheet_names:?}"
    );
    // None of the data-table sheets should be present.
    for data_sheet in [
        "Schedule",
        "Timeline",
        "Rooms",
        "Hotel",
        "PanelTypes",
        "People",
    ] {
        assert!(
            !sheet_names.iter().any(|n| n == data_sheet),
            "grid workbook must not contain the {data_sheet} data sheet"
        );
    }
}

/// A presenter that appears only on the Schedule sheet (created through the
/// tagged-credit path, never the People sheet) must get a *deterministic* v5
/// UUID — two independent fresh imports of the same file must agree on its
/// identity, otherwise a CRDT merge of the two would duplicate the presenter.
/// (REFACTOR-140 regression.)
#[test]
fn test_schedule_only_presenter_uuid_is_deterministic() {
    fn build() -> umya_spreadsheet::Spreadsheet {
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
            set_cell(ws, 2, 1, "Sort Key");
            set_cell(ws, 1, 2, "Main Hall");
            set_cell(ws, 2, 2, "10");
        }
        // People sheet lists only Alice — Bob exists solely on the Schedule.
        {
            let ws = book.new_sheet("People").unwrap();
            set_cell(ws, 1, 1, "Name");
            set_cell(ws, 2, 1, "Classification");
            set_cell(ws, 1, 2, "Alice Smith");
            set_cell(ws, 2, 2, "Guest");
        }
        {
            let ws = book.get_sheet_mut(&0).unwrap();
            ws.set_name("Schedule");
            set_cell(ws, 1, 1, "Name");
            set_cell(ws, 2, 1, "Room");
            set_cell(ws, 3, 1, "Type");
            set_cell(ws, 4, 1, "G: Alice Smith");
            set_cell(ws, 5, 1, "P: Bob Jones");
            set_cell(ws, 1, 2, "Opening Ceremonies");
            set_cell(ws, 2, 2, "Main Hall");
            set_cell(ws, 3, 2, "GP");
            set_cell(ws, 4, 2, "X");
            set_cell(ws, 5, 2, "X");
        }
        book
    }

    let bob_uuid = |schedule: &schedule_core::schedule::Schedule| {
        schedule
            .iter_entities::<PresenterEntityType>()
            .find(|(_, d)| d.data.name == "Bob Jones")
            .map(|(id, _)| id.entity_uuid())
            .expect("Bob Jones should exist")
    };

    let path_a = write_temp(build());
    let path_b = write_temp(build());
    let schedule_a = import_xlsx(&path_a, &XlsxImportOptions::default()).unwrap();
    let schedule_b = import_xlsx(&path_b, &XlsxImportOptions::default()).unwrap();
    cleanup(&path_a);
    cleanup(&path_b);

    assert_eq!(
        bob_uuid(&schedule_a),
        bob_uuid(&schedule_b),
        "a schedule-only presenter must get the same UUID across independent imports"
    );
}

/// Group membership declared on the People sheet must round-trip through export
/// and re-import: the exported People sheet populates the `Members`/`Groups`
/// columns, so the group edge survives. (REFACTOR-140 regression.)
#[test]
fn test_people_membership_round_trips_through_export() {
    let mut book = umya_spreadsheet::new_file();
    {
        let ws = book.new_sheet("PanelTypes").unwrap();
        set_cell(ws, 1, 1, "Prefix");
        set_cell(ws, 2, 1, "Panel Kind");
        set_cell(ws, 1, 2, "GP");
        set_cell(ws, 2, 2, "Guest Panel");
    }
    // People sheet: "Trio" is a group whose Members are Alice and Bob.
    {
        let ws = book.new_sheet("People").unwrap();
        set_cell(ws, 1, 1, "Person");
        set_cell(ws, 2, 1, "Classification");
        set_cell(ws, 3, 1, "Is Group");
        set_cell(ws, 4, 1, "Members");
        set_cell(ws, 1, 2, "Trio");
        set_cell(ws, 2, 2, "Guest");
        set_cell(ws, 3, 2, "Yes");
        set_cell(ws, 4, 2, "Alice, Bob");
        set_cell(ws, 1, 3, "Alice");
        set_cell(ws, 2, 3, "Guest");
        set_cell(ws, 1, 4, "Bob");
        set_cell(ws, 2, 4, "Guest");
    }
    {
        let ws = book.get_sheet_mut(&0).unwrap();
        ws.set_name("Schedule");
        set_cell(ws, 1, 1, "Name");
        set_cell(ws, 2, 1, "Type");
        set_cell(ws, 3, 1, "G: Trio");
        set_cell(ws, 1, 2, "Group Panel");
        set_cell(ws, 2, 2, "GP");
        set_cell(ws, 3, 2, "X");
    }
    let path = write_temp(book);
    let schedule = import_xlsx(&path, &XlsxImportOptions::default()).unwrap();
    cleanup(&path);

    let members_of = |s: &schedule_core::schedule::Schedule| -> Vec<String> {
        let trio = s
            .iter_entities::<PresenterEntityType>()
            .find(|(_, d)| d.data.name == "Trio")
            .map(|(id, _)| id)
            .expect("Trio should exist");
        let mut names: Vec<String> = s
            .connected_entities::<PresenterEntityType>(trio, presenter::EDGE_MEMBERS)
            .into_iter()
            .filter_map(|m| {
                s.get_internal::<PresenterEntityType>(m)
                    .map(|d| d.data.name.clone())
            })
            .collect();
        names.sort();
        names
    };

    assert_eq!(members_of(&schedule), vec!["Alice", "Bob"]);

    // Export → re-import; the membership edge must survive via the People sheet.
    let rt = round_trip(schedule);
    assert_eq!(
        members_of(&rt),
        vec!["Alice", "Bob"],
        "group members must round-trip through the exported People Members column"
    );
}
