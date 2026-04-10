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

## Phases

### Phase 1 — Foundation

- FEATURE-002: Cargo workspace setup with crate skeletons

### Phase 2 — Core Data Model

- FEATURE-003: EntityFields derive macro (schedule-macro)
- FEATURE-004: Field system (traits, FieldValue, FieldSet, validation)
- FEATURE-005: Core entity definitions
- FEATURE-006: UUID-based identity and typed ID wrappers
- FEATURE-007: Edge/relationship system
- FEATURE-008: Schedule container and EntityStorage
- FEATURE-009: Query system
- FEATURE-010: Edit command system with undo/redo history

### Phase 3 — CRDT Integration

- FEATURE-011: CRDT abstraction layer design
- FEATURE-012: CRDT-backed entity storage
- FEATURE-013: Change tracking and merge operations

### Phase 4 — File Formats & Import/Export

- FEATURE-014: Internal schedule file format
- FEATURE-015: Multi-year schedule archive support
- FEATURE-016: Widget display JSON export
- FEATURE-017: XLSX spreadsheet import
- FEATURE-018: XLSX spreadsheet export

### Phase 5 — CLI Tools

- CLI-019: cosam-convert
- CLI-020: cosam-modify

### Phase 6 — GUI Editor

- EDITOR-021: Framework selection and scaffold
- EDITOR-022: Schedule grid view and entity editing

### Phase 7 — Sync & Multi-User

- FEATURE-023: Peer-to-peer schedule sync protocol
- FEATURE-024: Merge conflict resolution UI

## Notes

- CRDT candidate: **automerge-rs** (document-oriented, good Rust support).
  Fallback: **crdts** (lower-level primitives). Design an abstraction layer
  so the backend can be swapped.
- GUI framework candidates: **iced** or **GPUI**. Decision deferred to Phase 6.
- JSON format: clean break from v10; the widget in this repository is v9-based.
