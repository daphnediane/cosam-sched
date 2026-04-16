# Architecture Redesign: CRDT-backed Schedule System

## Summary

Meta work item tracking the full multi-phase redesign of the schedule system.

## Status

Open

## Priority

High

## Description

Redesign the cosam-sched schedule system from the ground up with:

- **Entity/field system** using generic field descriptors (`FieldDescriptor<E>`)
  for clean, type-safe data structures — entity `Data` struct declarations are
  hand-written and visible; proc-macros may be used for boilerplate (trait
  impls, field accessor singletons, builders) as long as they do not hide the
  struct definitions
- **CRDT-backed storage** (automerge) enabling concurrent offline editing
  without a central database
- **Multi-year archive** support for jump-starting new conventions from prior years
- **Import/export** to and from the existing XLSX spreadsheet format
- **Widget JSON export** for the calendar display widget
- **Three application targets**: `cosam-convert` (format conversion),
  `cosam-modify` (CLI editing), `cosam-editor` (GUI editing)

All entity field infrastructure lives in a single `schedule-core` crate,
replacing the old `schedule-field`, `schedule-data`, and `schedule-macro` crates.

## Work Items

- META-002: Phase 1 — Foundation
- META-003: Phase 2 — Core Data Model (schedule-core)
- META-004: Phase 3 — CRDT Integration
- META-005: Phase 4 — File Formats & Import/Export
- META-006: Phase 5 — CLI Tools
- META-007: Phase 6 — GUI Editor
- META-008: Phase 7 — Sync & Multi-User

## Notes

- CRDT library: **automerge** (single-library approach; see `docs/crdt-design.md`)
- GUI framework candidates: **iced** or **GPUI**. Decision deferred to Phase 6.
- Reference implementations: `v10-try3` (old proc-macro approach),
  `v9`/`v10-try1` (flat data model in schedule-core)
