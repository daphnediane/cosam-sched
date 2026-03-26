/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::Path;
use std::process::Command;

fn cosam_modify_bin() -> std::path::PathBuf {
    // cargo test builds the binary at this well-known location
    let mut path = std::env::current_exe()
        .expect("current_exe")
        .parent()
        .expect("parent of test binary")
        .parent()
        .expect("parent of deps dir")
        .to_path_buf();
    path.push("cosam-modify");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    assert!(
        path.exists(),
        "cosam-modify binary not found at {}",
        path.display()
    );
    path
}

/// Create a minimal but valid schedule JSON in the given directory.
/// Returns the path to the created file.
fn create_fixture(dir: &Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let json = serde_json::json!({
        "meta": {
            "title": "Integration Test Schedule",
            "generated": "2026-01-01T00:00:00Z",
            "version": 8,
            "variant": "full"
        },
        "rooms": [
            {
                "uid": 10,
                "short_name": "Main",
                "long_name": "Main Events",
                "hotel_room": "Salon F/G",
                "sort_key": 1
            },
            {
                "uid": 20,
                "short_name": "Workshop 1",
                "long_name": "Workshop Room 1",
                "hotel_room": "Salon A",
                "sort_key": 2
            }
        ],
        "presenters": [],
        "panels": {
            "test-panel-1": {
                "id": "test-panel-1",
                "name": "Armor 101",
                "description": "Original description",
                "note": "Original note",
                "parts": [{
                    "sessions": [{
                        "id": "test-panel-1-0-0",
                        "roomIds": [10],
                        "startTime": "2026-07-10T10:00:00",
                        "endTime": "2026-07-10T11:00:00",
                        "duration": 60,
                        "creditedPresenters": ["Alice", "Bob"]
                    }]
                }]
            }
        }
    });
    std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    path
}

/// Run cosam-modify with the given arguments, returning (stdout, stderr, exit code).
fn run_modify(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(cosam_modify_bin())
        .args(args)
        .output()
        .expect("failed to run cosam-modify");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

#[test]
fn test_cli_show_history_empty_json() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = create_fixture(dir.path(), "schedule.json");
    let path = fixture.to_str().unwrap();

    let (stdout, _stderr, code) = run_modify(&["--file", path, "--format", "json", "show-history"]);
    assert_eq!(code, 0, "show-history should succeed");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON output");
    assert_eq!(parsed["undoCount"], 0);
    assert_eq!(parsed["redoCount"], 0);
}

#[test]
fn test_cli_show_history_empty_human() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = create_fixture(dir.path(), "schedule.json");
    let path = fixture.to_str().unwrap();

    let (stdout, _stderr, code) = run_modify(&["--file", path, "show-history"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Undo stack: 0 item(s)"));
    assert!(stdout.contains("Redo stack: 0 item(s)"));
}

#[test]
fn test_cli_undo_persists_across_invocations() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = create_fixture(dir.path(), "schedule.json");
    let path = fixture.to_str().unwrap();

    // 1) Modify description — this saves the file with history
    let (_stdout, stderr, code) = run_modify(&[
        "--file",
        path,
        "--select",
        "panel",
        "id:=test-panel-1",
        "set",
        "description",
        "Changed description",
    ]);
    assert_eq!(code, 0, "set should succeed: {stderr}");

    // 2) Verify history persisted: undoCount should be 1
    let (stdout, _stderr, code) =
        run_modify(&["--file", path, "--format", "json", "show-history"]);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["undoCount"], 1, "should have 1 undo entry after set");
    assert_eq!(parsed["redoCount"], 0);

    // 3) Undo the modification
    let (_stdout, stderr, code) = run_modify(&["--file", path, "undo"]);
    assert_eq!(code, 0, "undo should succeed: {stderr}");
    assert!(
        stderr.contains("Undo applied"),
        "should confirm undo: {stderr}"
    );

    // 4) Verify history after undo: undoCount=0, redoCount=1
    let (stdout, _stderr, code) =
        run_modify(&["--file", path, "--format", "json", "show-history"]);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["undoCount"], 0, "undo stack should be empty");
    assert_eq!(parsed["redoCount"], 1, "redo stack should have 1 entry");
}

#[test]
fn test_cli_redo_persists_across_invocations() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = create_fixture(dir.path(), "schedule.json");
    let path = fixture.to_str().unwrap();

    // 1) Set → 2) Undo → 3) Redo → 4) Verify value is re-applied
    run_modify(&[
        "--file",
        path,
        "--select",
        "panel",
        "id:=test-panel-1",
        "set",
        "note",
        "New note",
    ]);
    run_modify(&["--file", path, "undo"]);

    // Verify redo stack has 1 entry
    let (stdout, _stderr, _) = run_modify(&["--file", path, "--format", "json", "show-history"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["redoCount"], 1);

    // Redo
    let (_stdout, stderr, code) = run_modify(&["--file", path, "redo"]);
    assert_eq!(code, 0, "redo should succeed: {stderr}");
    assert!(
        stderr.contains("Redo applied"),
        "should confirm redo: {stderr}"
    );

    // Verify undo stack restored
    let (stdout, _stderr, _) = run_modify(&["--file", path, "--format", "json", "show-history"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["undoCount"], 1, "redo should push back onto undo");
    assert_eq!(parsed["redoCount"], 0, "redo stack should be empty");
}

#[test]
fn test_cli_undo_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = create_fixture(dir.path(), "schedule.json");
    let path = fixture.to_str().unwrap();

    let (_stdout, stderr, code) = run_modify(&["--file", path, "undo"]);
    assert_eq!(code, 0);
    assert!(
        stderr.contains("Nothing to undo"),
        "should report nothing to undo: {stderr}"
    );
}

#[test]
fn test_cli_redo_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = create_fixture(dir.path(), "schedule.json");
    let path = fixture.to_str().unwrap();

    let (_stdout, stderr, code) = run_modify(&["--file", path, "redo"]);
    assert_eq!(code, 0);
    assert!(
        stderr.contains("Nothing to redo"),
        "should report nothing to redo: {stderr}"
    );
}

#[test]
fn test_cli_full_workflow() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = create_fixture(dir.path(), "schedule.json");
    let path = fixture.to_str().unwrap();

    // Step 1: Modify description
    let (_, stderr, code) = run_modify(&[
        "--file",
        path,
        "--select",
        "panel",
        "id:=test-panel-1",
        "set",
        "description",
        "Modified desc",
    ]);
    assert_eq!(code, 0, "set description: {stderr}");

    // Step 2: Verify the modification was saved to JSON
    let content = std::fs::read_to_string(path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        json["panels"]["test-panel-1"]["description"],
        "Modified desc"
    );

    // Step 3: Verify changeLog is present
    assert!(
        json.get("changeLog").is_some(),
        "changeLog should be present in saved JSON"
    );

    // Step 4: Undo
    let (_, stderr, code) = run_modify(&["--file", path, "undo"]);
    assert_eq!(code, 0, "undo: {stderr}");

    // Step 5: Verify original description restored in file
    let content = std::fs::read_to_string(path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        json["panels"]["test-panel-1"]["description"],
        "Original description",
        "undo should restore original description in saved file"
    );

    // Step 6: Redo
    let (_, stderr, code) = run_modify(&["--file", path, "redo"]);
    assert_eq!(code, 0, "redo: {stderr}");

    // Step 7: Verify re-applied description
    let content = std::fs::read_to_string(path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        json["panels"]["test-panel-1"]["description"],
        "Modified desc",
        "redo should re-apply the description change"
    );
}

#[test]
fn test_cli_multiple_modifications_and_undo() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = create_fixture(dir.path(), "schedule.json");
    let path = fixture.to_str().unwrap();

    // Apply 3 modifications
    run_modify(&[
        "--file",
        path,
        "--select",
        "panel",
        "id:=test-panel-1",
        "set",
        "description",
        "Desc A",
    ]);
    run_modify(&[
        "--file",
        path,
        "--select",
        "panel",
        "id:=test-panel-1",
        "set",
        "note",
        "Note B",
    ]);
    run_modify(&[
        "--file",
        path,
        "--select",
        "panel",
        "id:=test-panel-1",
        "add-presenter",
        "Charlie",
    ]);

    // Verify history shows 3 items
    let (stdout, _, _) = run_modify(&["--file", path, "--format", "json", "show-history"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["undoCount"], 3);

    // Undo all 3
    run_modify(&["--file", path, "undo"]);
    run_modify(&["--file", path, "undo"]);
    run_modify(&["--file", path, "undo"]);

    // Verify back to original
    let (stdout, _, _) = run_modify(&["--file", path, "--format", "json", "show-history"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["undoCount"], 0);
    assert_eq!(parsed["redoCount"], 3);

    let content = std::fs::read_to_string(path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        json["panels"]["test-panel-1"]["description"],
        "Original description"
    );
    assert_eq!(
        json["panels"]["test-panel-1"]["note"],
        "Original note"
    );
}
