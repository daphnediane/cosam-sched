/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Spike: prose field CRDT scenarios using the `automerge` crate.
//!
//! Demonstrates character-level concurrent editing (RGA/CRDT Text) for fields
//! where LWW-Register is insufficient: `description`, `note`,
//! `notes_non_printing`, `workshop_notes`, `av_notes`.
//!
//! ## automerge 0.5 API notes
//!
//! - `AutoCommit::new()` — creates a document; transactions are committed
//!   automatically on the next `fork()` / `merge()` / explicit commit.
//! - `put_object(parent, key, ObjType::Text)` (requires `Transactable`) —
//!   creates a Text CRDT nested inside a Map or at ROOT.
//! - `splice_text(obj, pos, del, insert)` — character-level edit:
//!   `pos` is the Unicode char index, `del` is chars to delete (isize),
//!   `insert` is the string to insert.
//! - `doc.text(obj)` (requires `ReadDoc`) — reads current text as `String`.
//! - `doc.fork()` — creates a causally-divergent replica.
//! - `doc.merge(&mut other)` — merges divergent edits; returns the new heads.
//!
//! ## Why automerge over crdts::LWWReg for prose
//!
//! LWW would discard one concurrent writer's entire prose edit.  RGA (used by
//! automerge Text) merges edits at character granularity: edits to non-
//! overlapping character ranges are both preserved; simultaneous inserts at the
//! same position are given a deterministic total order.

use automerge::transaction::Transactable;
use automerge::{AutoCommit, ObjId, ObjType, ReadDoc, ROOT};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Creates a new document with a panel entity map and a description Text CRDT
/// pre-populated with `initial_text`.  Returns the doc and the ObjId of the
/// description Text object.
fn panel_with_description(initial_text: &str) -> (AutoCommit, ObjId) {
    let mut doc = AutoCommit::new();
    let panel = doc.put_object(ROOT, "panel", ObjType::Map).unwrap();
    let desc = doc
        .put_object(&panel, "description", ObjType::Text)
        .unwrap();
    doc.splice_text(&desc, 0, 0, initial_text).unwrap();
    (doc, desc)
}

// ---------------------------------------------------------------------------
// Scenario P1 — Two actors edit different positions of the same description
// ---------------------------------------------------------------------------

/// When Actor A prepends a tag and Actor B appends a note concurrently,
/// both edits appear in the merged text (RGA / CRDT Text semantics).
#[test]
fn p1_edits_at_different_positions_both_survive() {
    let initial = "Alice presents: Introduction to Cosplay.";

    let (mut doc_a, desc_a) = panel_with_description(initial);
    let mut doc_b = doc_a.fork();
    let desc_b = desc_a.clone(); // same ObjId — shared causal history

    // Actor A prepends a session tag at character position 0
    doc_a.splice_text(&desc_a, 0, 0, "[Morning] ").unwrap();

    // Actor B appends a note at the end of the original string
    // (after the period, length of "Alice presents: Introduction to Cosplay.")
    let original_len = initial.chars().count();
    doc_b
        .splice_text(&desc_b, original_len, 0, " Registration required.")
        .unwrap();

    // Merge B into A
    doc_a.merge(&mut doc_b).unwrap();

    let merged = doc_a.text(&desc_a).unwrap();

    // Both edits must be present somewhere in the merged text
    assert!(
        merged.contains("[Morning]"),
        "A's prefix tag must survive merge; got: {merged:?}"
    );
    assert!(
        merged.contains("Registration required"),
        "B's appended note must survive merge; got: {merged:?}"
    );
    assert!(
        merged.contains("Alice presents"),
        "Original text must still be present; got: {merged:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario P2 — Global find-replace vs concurrent description edit
// ---------------------------------------------------------------------------

/// Real-world scenario: a guest name was inconsistently spelled across
/// descriptions.  One operator does a global find-replace on the name at the
/// start of the text.  Concurrently, a second operator edits a different part
/// of the same description (removes a trailing boilerplate sentence).
///
/// After merge both changes must be present: the corrected name AND the
/// removed boilerplate sentence.
#[test]
fn p2_find_replace_and_concurrent_edit_both_survive() {
    // Initial description with the misspelled name and trailing boilerplate
    let initial =
        "Cosplay Ant and Jane teach gold thread embroidery. Tickets available at the door.";

    let (mut doc_a, desc_a) = panel_with_description(initial);
    let mut doc_b = doc_a.fork();
    let desc_b = desc_a.clone();

    // Actor A: global find-replace "Cosplay Ant" → "Cosplay Aunt" at position 0
    // (delete 11 chars "Cosplay Ant", insert "Cosplay Aunt")
    let old_name = "Cosplay Ant";
    let new_name = "Cosplay Aunt";
    doc_a
        .splice_text(&desc_a, 0, old_name.chars().count() as isize, new_name)
        .unwrap();

    // Actor B: removes the trailing boilerplate " Tickets available at the door."
    // Original: "...gold thread embroidery. Tickets available at the door."
    // B works from its own view (still has "Cosplay Ant") — it does not see A's edit yet.
    let boilerplate = " Tickets available at the door.";
    let b_text = doc_b.text(&desc_b).unwrap();
    let keep_len = b_text.chars().count() - boilerplate.chars().count();
    doc_b
        .splice_text(&desc_b, keep_len, boilerplate.chars().count() as isize, "")
        .unwrap();

    // Merge: A incorporates B's edit
    doc_a.merge(&mut doc_b).unwrap();

    let merged = doc_a.text(&desc_a).unwrap();

    // The corrected name must be present (A's find-replace survived)
    assert!(
        merged.contains("Cosplay Aunt"),
        "Find-replace (Cosplay Aunt) must survive merge; got: {merged:?}"
    );

    // The old name must no longer appear as a standalone word
    assert!(
        !merged.contains("Cosplay Ant "),
        "Old misspelling must be gone; got: {merged:?}"
    );

    // The boilerplate must be gone (B's removal survived)
    assert!(
        !merged.contains("Tickets available at the door"),
        "B's removed boilerplate must be gone; got: {merged:?}"
    );

    // Core description text must remain
    assert!(
        merged.contains("gold thread embroidery"),
        "Core description text must be preserved; got: {merged:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario P3 — Concurrent inserts at the same character position
// ---------------------------------------------------------------------------

/// When two actors insert different text at the same position, automerge's RGA
/// algorithm places them in a deterministic total order.  Both inserts must
/// appear in the merged result (no data is lost).
#[test]
fn p3_concurrent_inserts_at_same_position_both_preserved() {
    let initial = "Panel description.";

    let (mut doc_a, desc_a) = panel_with_description(initial);
    let mut doc_b = doc_a.fork();
    let desc_b = desc_a.clone();

    // Both actors insert at position 0 concurrently
    doc_a.splice_text(&desc_a, 0, 0, "[TAG-A] ").unwrap();
    doc_b.splice_text(&desc_b, 0, 0, "[TAG-B] ").unwrap();

    // Merge both directions and verify convergence
    let mut doc_a2 = doc_a.fork();
    doc_a2.merge(&mut doc_b.fork()).unwrap();

    let mut doc_b2 = doc_b.fork();
    doc_b2.merge(&mut doc_a.fork()).unwrap();

    let text_from_a = doc_a2.text(&desc_a).unwrap();
    let text_from_b = doc_b2.text(&desc_b).unwrap();

    // Both replicas must converge to the same string
    assert_eq!(
        text_from_a, text_from_b,
        "Concurrent inserts at same position must converge"
    );

    // Both tags must be present
    assert!(
        text_from_a.contains("[TAG-A]"),
        "A's insert must survive; got: {text_from_a:?}"
    );
    assert!(
        text_from_a.contains("[TAG-B]"),
        "B's insert must survive; got: {text_from_a:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario P4 — Idempotent merge (X ∪ X = X)
// ---------------------------------------------------------------------------

/// Merging a document with a clone of itself must not duplicate any characters.
#[test]
fn p4_idempotent_merge() {
    let initial = "Sewing machine maintenance with Cosplay Aunt.";

    let (mut doc, desc) = panel_with_description(initial);

    let mut clone = doc.fork();
    doc.merge(&mut clone).unwrap();

    let merged = doc.text(&desc).unwrap();
    assert_eq!(
        merged, initial,
        "Idempotent merge must not alter or duplicate text; got: {merged:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario P5 — Computed-data contamination: prose field trimmed on edit
// ---------------------------------------------------------------------------

/// Demonstrates a workflow from the real con-scheduling use case: a description
/// was copy-pasted from a previous year and contains stale computed data
/// ("Sat 10:00am, Room 101").  One operator trims the stale suffix.
/// Concurrently another operator corrects a presenter name earlier in the text.
/// Both edits must survive so that neither change silently disappears.
#[test]
fn p5_trim_stale_computed_data_and_concurrent_name_fix() {
    let initial = "Cosplay Ant teaches sewing. Sat 10:00am, Room 101.";

    let (mut doc_a, desc_a) = panel_with_description(initial);
    let mut doc_b = doc_a.fork();
    let desc_b = desc_a.clone();

    // Actor A: fix the presenter name ("Cosplay Ant" → "Cosplay Aunt", pos 0)
    let old = "Cosplay Ant";
    let new = "Cosplay Aunt";
    doc_a
        .splice_text(&desc_a, 0, old.chars().count() as isize, new)
        .unwrap();

    // Actor B: remove the stale computed suffix " Sat 10:00am, Room 101."
    let b_text = doc_b.text(&desc_b).unwrap();
    let stale = " Sat 10:00am, Room 101.";
    let keep = b_text.chars().count() - stale.chars().count();
    doc_b
        .splice_text(&desc_b, keep, stale.chars().count() as isize, "")
        .unwrap();

    // Merge
    doc_a.merge(&mut doc_b).unwrap();
    let merged = doc_a.text(&desc_a).unwrap();

    assert!(
        merged.contains("Cosplay Aunt"),
        "Name correction must survive; got: {merged:?}"
    );
    assert!(
        !merged.contains("Sat 10:00am"),
        "Stale computed data must be removed; got: {merged:?}"
    );
    assert!(
        merged.contains("teaches sewing"),
        "Core description text must remain; got: {merged:?}"
    );
}
