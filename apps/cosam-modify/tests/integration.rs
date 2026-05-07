/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Integration tests for cosam-modify. (CLI-098)
//!
//! Each test invokes the compiled binary via `std::process::Command`.

use std::path::PathBuf;
use std::process::Command;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn binary() -> PathBuf {
    // Navigate from the integration-test binary to the cosam-modify binary.
    // cargo places both in the same target/<profile>/ directory (integration
    // tests go into target/<profile>/deps/, so pop one level to reach the
    // profile directory where the main binary lives).
    let mut path = std::env::current_exe().expect("failed to get current executable");
    path.pop(); // remove test binary name
    if path.ends_with("deps") {
        path.pop(); // deps/ → target/<profile>/
    }
    path.push(format!("cosam-modify{}", std::env::consts::EXE_SUFFIX));
    path
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("failed to run cosam-modify")
}

/// Return a unique temp file path with the given extension.
fn tmp(ext: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tid = format!("{:?}", std::thread::current().id()).replace(['(', ')', ' '], "");
    std::env::temp_dir().join(format!("cosam_test_{nanos}_{tid}{ext}"))
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// ── Help ──────────────────────────────────────────────────────────────────────

#[test]
fn test_help_exits_zero_and_prints_usage() {
    let out = run(&["--help"]);
    assert_eq!(out.status.code(), Some(0), "expected exit 0 for --help");
    let text = stdout(&out);
    assert!(
        text.contains("USAGE"),
        "expected USAGE in help output:\n{text}"
    );
    assert!(
        text.contains("--file"),
        "expected --file in help output:\n{text}"
    );
}

#[test]
fn test_short_help_flag() {
    let out = run(&["-h"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).contains("USAGE"));
}

// ── Error handling ────────────────────────────────────────────────────────────

#[test]
fn test_missing_file_arg_exits_one() {
    let out = run(&["--select", "panel_type", "list"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit 1 when --file is missing"
    );
}

#[test]
fn test_unknown_command_exits_one() {
    let f = tmp(".cosam");
    let out = run(&["--file", f.to_str().unwrap(), "--new", "frobnicate"]);
    assert_eq!(out.status.code(), Some(1));
}

// ── Create → list round-trip (all entity types) ───────────────────────────────

#[test]
fn test_panel_type_create_list() {
    let f = tmp(".cosam");
    let create = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "panel_type",
        "create",
        "prefix=GP",
        "kind=Guest Panel",
    ]);
    assert_eq!(
        create.status.code(),
        Some(0),
        "create failed:\n{}",
        stdout(&create)
    );

    let list = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "panel_type",
        "list",
    ]);
    assert_eq!(list.status.code(), Some(0));
    let out = stdout(&list);
    assert!(out.contains("GP"), "expected 'GP' in list:\n{out}");
    assert!(
        out.contains("Guest Panel"),
        "expected 'Guest Panel' in list:\n{out}"
    );
    let _ = std::fs::remove_file(&f);
}

#[test]
fn test_presenter_create_list() {
    let f = tmp(".cosam");
    let create = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "presenter",
        "create",
        "name=Alice",
    ]);
    assert_eq!(create.status.code(), Some(0));

    let list = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "presenter",
        "list",
    ]);
    assert_eq!(list.status.code(), Some(0));
    let out = stdout(&list);
    assert!(out.contains("Alice"), "expected 'Alice' in list:\n{out}");
    let _ = std::fs::remove_file(&f);
}

#[test]
fn test_event_room_create_list() {
    let f = tmp(".cosam");
    let create = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "event_room",
        "create",
        "name=Ballroom",
    ]);
    assert_eq!(create.status.code(), Some(0));

    let list = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "event_room",
        "list",
    ]);
    assert_eq!(list.status.code(), Some(0));
    assert!(stdout(&list).contains("Ballroom"));
    let _ = std::fs::remove_file(&f);
}

#[test]
fn test_hotel_room_create_list() {
    let f = tmp(".cosam");
    let create = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "hotel_room",
        "create",
        "name=Room 101",
    ]);
    assert_eq!(create.status.code(), Some(0));

    let list = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "hotel_room",
        "list",
    ]);
    assert_eq!(list.status.code(), Some(0));
    assert!(stdout(&list).contains("Room 101"));
    let _ = std::fs::remove_file(&f);
}

#[test]
fn test_panel_create_list() {
    let f = tmp(".cosam");
    // Create panel_type first so the prefix exists, then create a panel.
    let create = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "panel_type",
        "create",
        "prefix=GP",
        "kind=Guest",
        "--",
        "--select",
        "panel",
        "create",
        "code=GP001",
        "name=Test Panel",
    ]);
    assert_eq!(
        create.status.code(),
        Some(0),
        "create failed:\n{}",
        stdout(&create)
    );

    let list = run(&["--file", f.to_str().unwrap(), "--select", "panel", "list"]);
    assert_eq!(list.status.code(), Some(0));
    let out = stdout(&list);
    assert!(out.contains("GP001"), "expected 'GP001' in list:\n{out}");
    assert!(
        out.contains("Test Panel"),
        "expected 'Test Panel' in list:\n{out}"
    );
    let _ = std::fs::remove_file(&f);
}

// ── Set → get round-trip ──────────────────────────────────────────────────────

#[test]
fn test_set_get_field() {
    let f = tmp(".cosam");
    // Create a panel_type, then set its panel_kind, then get to verify.
    let create = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "panel_type",
        "create",
        "prefix=GP",
        "kind=Initial",
    ]);
    assert_eq!(create.status.code(), Some(0));

    let set = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "panel_type",
        "GP",
        "set",
        "kind",
        "Updated Kind",
    ]);
    assert_eq!(set.status.code(), Some(0), "set failed:\n{}", stdout(&set));

    let get = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "panel_type",
        "get",
        "GP",
    ]);
    assert_eq!(get.status.code(), Some(0));
    let out = stdout(&get);
    assert!(
        out.contains("Updated Kind"),
        "expected updated value in get:\n{out}"
    );
    let _ = std::fs::remove_file(&f);
}

// ── Delete ────────────────────────────────────────────────────────────────────

#[test]
fn test_delete() {
    let f = tmp(".cosam");
    let setup = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "panel_type",
        "create",
        "prefix=GP",
        "kind=Guest",
        "--",
        "--select",
        "panel_type",
        "create",
        "prefix=WP",
        "kind=Workshop",
    ]);
    assert_eq!(setup.status.code(), Some(0));

    let del = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "panel_type",
        "delete",
        "GP",
    ]);
    assert_eq!(
        del.status.code(),
        Some(0),
        "delete failed:\n{}",
        stdout(&del)
    );

    let list = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "panel_type",
        "list",
    ]);
    let out = stdout(&list);
    assert!(
        !out.contains("GP"),
        "GP should be gone after delete:\n{out}"
    );
    assert!(out.contains("WP"), "WP should still be present:\n{out}");
    let _ = std::fs::remove_file(&f);
}

// ── Add-edge / remove-edge ────────────────────────────────────────────────────

#[test]
fn test_add_remove_edge() {
    let f = tmp(".cosam");

    // Create all entities and add the edge in a single invocation.
    let setup = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "panel_type",
        "create",
        "prefix=GP",
        "kind=Guest",
        "--",
        "--select",
        "presenter",
        "create",
        "name=Alice",
        "--",
        "--select",
        "panel",
        "create",
        "code=GP001",
        "name=EdgeTest",
        "--",
        "--select",
        "panel",
        "GP001",
        "add-edge",
        "credited_presenters",
        "Alice",
    ]);
    assert_eq!(
        setup.status.code(),
        Some(0),
        "setup failed:\n{}",
        stdout(&setup)
    );

    // Get the panel and verify credited_presenters is present (non-empty).
    let get = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "panel",
        "get",
        "GP001",
    ]);
    assert_eq!(get.status.code(), Some(0));
    let out = stdout(&get);
    assert!(
        out.contains("credited_presenters"),
        "expected credited_presenters in get output:\n{out}"
    );
    // The field should have a non-empty value (some UUID).
    let has_value = out
        .lines()
        .any(|l| l.trim_start().starts_with("credited_presenters:") && l.contains('-'));
    assert!(
        has_value,
        "credited_presenters should have a UUID value:\n{out}"
    );

    // Remove the edge.
    let remove = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "panel",
        "GP001",
        "remove-edge",
        "credited_presenters",
        "Alice",
    ]);
    assert_eq!(
        remove.status.code(),
        Some(0),
        "remove-edge failed:\n{}",
        stdout(&remove)
    );

    // After removal, get should show no credited_presenters value.
    let get2 = run(&[
        "--file",
        f.to_str().unwrap(),
        "--select",
        "panel",
        "get",
        "GP001",
    ]);
    let out2 = stdout(&get2);
    let still_has_value = out2
        .lines()
        .any(|l| l.trim_start().starts_with("credited_presenters:") && l.contains('-'));
    assert!(
        !still_has_value,
        "credited_presenters should be empty after remove:\n{out2}"
    );

    let _ = std::fs::remove_file(&f);
}

// ── Undo / redo ───────────────────────────────────────────────────────────────

#[test]
fn test_undo_reverts_create() {
    let f = tmp(".cosam");
    // Create then undo in same invocation; list should be empty.
    let out = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "panel_type",
        "create",
        "prefix=GP",
        "kind=Guest",
        "--",
        "undo",
        "--",
        "--select",
        "panel_type",
        "list",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "undo run failed:\n{}",
        stdout(&out)
    );
    let text = stdout(&out);
    assert!(
        text.trim().is_empty(),
        "list should be empty after undo:\n{text}"
    );
    let _ = std::fs::remove_file(&f);
}

#[test]
fn test_redo_reapplies_after_undo() {
    let f = tmp(".cosam");
    // Create, undo, redo — final list should contain the entity.
    let out = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "panel_type",
        "create",
        "prefix=GP",
        "kind=Guest",
        "--",
        "undo",
        "--",
        "redo",
        "--",
        "--select",
        "panel_type",
        "list",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "redo run failed:\n{}",
        stdout(&out)
    );
    let text = stdout(&out);
    assert!(text.contains("GP"), "GP should be back after redo:\n{text}");
    let _ = std::fs::remove_file(&f);
}

#[test]
fn test_undo_empty_stack_exits_one() {
    let f = tmp(".cosam");
    let out = run(&["--file", f.to_str().unwrap(), "--new", "undo"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit 1 for undo on empty stack"
    );
    let _ = std::fs::remove_file(&f);
}

#[test]
fn test_show_history() {
    let f = tmp(".cosam");
    // After two creates, undo depth should be 2, redo depth 0.
    let out = run(&[
        "--file",
        f.to_str().unwrap(),
        "--new",
        "--select",
        "panel_type",
        "create",
        "prefix=GP",
        "kind=Guest",
        "--",
        "--select",
        "panel_type",
        "create",
        "prefix=WP",
        "kind=Workshop",
        "--",
        "show-history",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.contains("undo: 2"), "expected undo depth 2:\n{text}");
    assert!(text.contains("redo: 0"), "expected redo depth 0:\n{text}");
    let _ = std::fs::remove_file(&f);
}

// ── XLSX → binary conversion ──────────────────────────────────────────────────

/// Load an xlsx schedule directly and list its panels.
///
/// Ignored by default because it requires `input/` xlsx fixtures.
/// Run with: `cargo test -p cosam-modify -- --ignored`
#[test]
#[ignore]
fn test_xlsx_to_binary_conversion() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let xlsx = manifest.join("../../input/2024 Schedule.xlsx");
    if !xlsx.exists() {
        eprintln!("skipping: xlsx fixture not found at {}", xlsx.display());
        return;
    }

    // The binary auto-detects xlsx by magic bytes. List panels directly from xlsx.
    let out = Command::new(binary())
        .args([
            "--file",
            xlsx.to_str().unwrap(),
            "--select",
            "panel",
            "list",
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "xlsx list failed:\n{}",
        stdout(&out)
    );
    let panels = stdout(&out);
    assert!(
        !panels.trim().is_empty(),
        "expected panels from xlsx:\n{panels}"
    );
}
