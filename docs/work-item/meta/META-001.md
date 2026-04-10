# Architecture Redesign: CRDT-backed Schedule System

## Summary

Meta work item tracking the full multi-phase redesign of the schedule system.

## Status

Blocked

## Priority

High

## Description

Redesign the cosam-sched schedule system from the ground up with:

- **Entity/field system** using a proc-macro (`#[derive(EntityFields)]`) for clean,
  type-safe data structures (ported from `feature/schedule-data` experiment)
- **CRDT-backed storage** enabling a handful of users to edit the schedule concurrently
  without a central database
- **Multi-year archive** support for jump-starting new conventions from prior years
- **Import/export** to and from the existing XLSX spreadsheet format
- **Widget JSON export** for the calendar display widget
- **Three application targets**: `cosam-convert` (format conversion), `cosam-modify`
  (CLI editing), `cosam-editor` (GUI editing)

## Work Items

- META-025: Phase 1 — Foundation
- META-026: Phase 2 — Core Data Model
- META-027: Phase 3 — CRDT Integration
- META-028: Phase 4 — File Formats & Import/Export
- META-029: Phase 5 — CLI Tools
- META-030: Phase 6 — GUI Editor
- META-031: Phase 7 — Sync & Multi-User

## Notes

- CRDT candidate: **automerge-rs** (document-oriented, good Rust support).
  Fallback: **crdts** (lower-level primitives). Design an abstraction layer
  so the backend can be swapped.
- GUI framework candidates: **iced** or **GPUI**. Decision deferred to Phase 6.
- JSON format: clean break from v10; the widget in this repository is v9-based.
