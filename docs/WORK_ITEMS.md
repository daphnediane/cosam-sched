# Cosplay America Schedule - Work Item

Updated on: Tue Apr 21 10:51:45 2026

## Completed

* [FEATURE-009] Set up the Cargo workspace root and create skeleton application crates.
* [FEATURE-010] Implement the universal `FieldValue` enum, error types, and CRDT field type annotation.
* [FEATURE-011] Implement the field trait hierarchy and generic `FieldDescriptor` type that replaces the old proc-macro's generated per-field unit structs.
* [FEATURE-012] Implement UUID-based entity identity with compile-time type-safe ID wrappers.
* [FEATURE-013] Implement the static `FieldSet` registry for per-entity-type field metadata lookup.
* [FEATURE-014] Implement the PanelType entity as the first proof of concept for the no-proc-macro field system.
* [FEATURE-015] Port `TimeRange` and implement the Panel entity with stored and computed time fields.
* [FEATURE-016] Implement the remaining core entity data structs and field descriptors.
* [FEATURE-017] Implement entity builders for constructing entity data with UUID assignment.
* [FEATURE-018] Implement typed relationship storage for entity-to-entity relationships.
* [FEATURE-019] Implement the `Schedule` struct and `EntityStorage` for managing all entities and relationships.
* [FEATURE-020] Implement field-based search, matching, and bulk update operations.
* [FEATURE-021] Implement a command-based edit system with full undo/redo support.
* [FEATURE-022] Make an `automerge::AutoCommit` document the authoritative storage inside
`Schedule`; the in-memory `HashMap` entity store becomes a derived cache.
* [FEATURE-023] Store relationships as automerge list fields on a canonical owner entity;
`RawEdgeMap` becomes a derived index rebuilt from these lists.
* [FEATURE-024] Expose automerge change tracking and merge through `Schedule`, and surface
concurrent scalar conflicts to the caller.
* [FEATURE-025] Define and implement the native save/load format for schedule documents.
* [FEATURE-038] Add a type-safe `FieldValueConverter<M>` trait and driver functions for converting
`FieldValue` inputs to typed Rust outputs via a work-queue iteration pattern.
* [FEATURE-043] Add a `verify` callback to `FieldDescriptor` for cross-field consistency checks after batch writes to computed fields.
* [FEATURE-046] Add `FieldSet::write_multiple()` for atomic batch field updates with verification support.
* [FEATURE-050] Add `FieldTypeItem` (scalar type tags) and `FieldType` (`Single`/`Optional`/`List`
wrappers) to `value.rs` as `Copy` type-level mirrors of `FieldValueItem`/`FieldValue`.
* [FEATURE-051] Add a `field_type: FieldType` field to `FieldDescriptor` and populate it in all
existing static field descriptors across every entity file.
* [FEATURE-057] Implement a transitive edge relationship cache to efficiently compute inclusive members, groups, panels, and other hierarchical relationships.
* [META-002] Phase tracker for project foundation and Cargo workspace setup.
* [META-003] Phase tracker for the entity/field system and core schedule data model in schedule-core.
* [META-004] Phase tracker for making an automerge CRDT document the authoritative storage
underneath `Schedule`.
* [META-048] Restructure `FieldValue` with proper cardinality, add `FieldTypeItem`/`FieldType`
enums, wire `FieldType` into `FieldDescriptor`, and implement the generic
`FieldValueConverter` system from IDEA-038.
* [REFACTOR-041] Replace the `EntityKind` enum with direct use of `EntityType::TYPE_NAME` strings,
following the v10-try3 design. This eliminates the central enum that required
modification for every new entity type.
* [REFACTOR-047] Extract the `macro_rules!` helpers from `panel.rs` into a shared `field_macros.rs`
and adopt them in `panel_type.rs` to eliminate per-entity boilerplate.
* [REFACTOR-049] Split the flat `FieldValue` enum into `FieldValueItem` (scalars only) and
`FieldValue` (`Single`/`List` wrappers), removing `None`,
`NonNilUuid`, and `EntityIdentifier` variants.
* [REFACTOR-052] Add `CollectedField<E>`, `RegisteredEntityType`, `order` field on `FieldDescriptor`,
`FieldSet::from_inventory`, and update field macros to self-submit via inventory.
* [REFACTOR-053] Replace the manual `FieldSet::new(&[...])` list in each entity type module with
`FieldSet::from_inventory()`, letting fields self-register via `inventory::submit!`.
* [REFACTOR-054] Register all entity types via `inventory::submit!` into a central `RegisteredEntityType`
collection, and expose a `registered_entity_types()` accessor.
* [REFACTOR-055] Add `define_field!` macro to bundle hand-written `FieldDescriptor` statics with
`inventory::submit!`, and add `IntoFieldValue` trait hierarchy for type-deduced
`field_value!(expr)` construction.

---

## Summary of Open Items

**Total open items:** 17

* **Meta / Project-Level**
  * [META-001] Meta work item tracking the full multi-phase redesign of the schedule system. (Blocked by [META-005], [META-006], [META-007], [META-008])
  * [META-005] Phase tracker for internal file format, multi-year archive, widget JSON, and
XLSX import/export. (Blocked by [META-004])
  * [META-006] Phase tracker for the cosam-convert and cosam-modify command-line applications. (Blocked by [META-005])
  * [META-007] Phase tracker for the cosam-editor desktop GUI application. (Blocked by [META-005])
  * [META-008] Phase tracker for peer-to-peer schedule synchronization and conflict resolution. (Blocked by [META-004])

* **Medium Priority**
  * [BUGFIX-045] In `scratch/field_update_logic.rs`, duration values are incorrectly stored as `FieldValue::Integer(minutes)` instead of `FieldValue::Duration(Duration)`.
  * [FEATURE-026] ([META-005]) Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.
  * [FEATURE-027] ([META-005]) Implement export of schedule data to the JSON format consumed by the calendar display widget.
  * [FEATURE-028] ([META-005]) Import schedule data from the existing XLSX spreadsheet format.
  * [FEATURE-029] ([META-005]) Export schedule data back to the XLSX spreadsheet format.
  * [FEATURE-056] Add computed/synthesized fields to public data structures to support widget JSON export.

* **Low Priority**
  * [CLI-030] ([META-006]) CLI tool for converting between schedule file formats (XLSX, JSON, widget JSON).
  * [CLI-031] ([META-006]) CLI tool for making batch edits to schedule data from the command line.
  * [EDITOR-032] ([META-007]) Select the GUI framework for cosam-editor and create the application scaffold.
  * [EDITOR-033] ([META-007]) Implement the main schedule grid view and entity editing UI in cosam-editor.
  * [FEATURE-034] ([META-008]) Define and implement the protocol for synchronizing schedule data between peers.
  * [FEATURE-035] ([META-008]) Provide UI for reviewing and resolving merge conflicts after sync.

---

## Placeholders

*No placeholders — all stubs have been promoted.*

Use `perl scripts/work-item-update.pl --create <PREFIX>` to add new stubs.

---

## Open BUGFIX Items

### [BUGFIX-045] BUGFIX-045: Duration stored as Integer instead of Duration in field_update_logic.rs

**Status:** Open

**Priority:** Medium

**Summary:** In `scratch/field_update_logic.rs`, duration values are incorrectly stored as `FieldValue::Integer(minutes)` instead of `FieldValue::Duration(Duration)`.

**Description:** The `FieldValue` enum has a dedicated `Duration(Duration)` variant for representing time durations. However, in `scratch/field_update_logic.rs`, duration values are being pushed as `FieldValue::Integer(new_duration_minutes)` instead of using the proper `FieldValue::Duration` variant with a `chrono::Duration`.

This is a type safety issue — durations should be typed as `Duration`, not raw integers, to ensure:

* Type-safe operations (can't accidentally add minutes to a count field)
* Proper serialization (duration format vs raw number)
* Clear semantic meaning in the type system

---

## Open CLI Items

### [CLI-030] cosam-convert: Format Conversion Tool

**Status:** Open

**Priority:** Low

**Summary:** CLI tool for converting between schedule file formats (XLSX, JSON, widget JSON).

**Part of:** [META-006]

**Description:** `cosam-convert` is a command-line application for importing and exporting
schedule data between supported formats.

---

### [CLI-031] cosam-modify: CLI Editing Tool

**Status:** Open

**Priority:** Low

**Summary:** CLI tool for making batch edits to schedule data from the command line.

**Part of:** [META-006]

**Description:** `cosam-modify` provides command-line access to the edit system for scripted
or batch modifications to schedule data.

---

## Open EDITOR Items

### [EDITOR-032] cosam-editor: GUI Framework Selection and Scaffold

**Status:** Open

**Priority:** Low

**Summary:** Select the GUI framework for cosam-editor and create the application scaffold.

**Part of:** [META-007]

**Description:** Evaluate and select between GUI framework candidates, then create the initial
application structure.

---

### [EDITOR-033] cosam-editor: Schedule Grid View and Entity Editing

**Status:** Open

**Priority:** Low

**Summary:** Implement the main schedule grid view and entity editing UI in cosam-editor.

**Part of:** [META-007]

**Description:** The core editing experience for the GUI application: a grid view showing
panels arranged by time and room, with inline editing of entity fields.

---

## Open FEATURE Items

### [FEATURE-026] Multi-Year Schedule Archive Support

**Status:** Open

**Priority:** Medium

**Summary:** Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.

**Part of:** [META-005]

**Description:** A schedule archive contains multiple years of convention data in one file,
enabling:

* **Jump-start**: Copy entities from a prior year to pre-populate the next
  convention (recurring panels, returning presenters, same rooms)
* **Historical reference**: View past schedules alongside the current one

---

### [FEATURE-027] Widget Display JSON Export

**Status:** Open

**Priority:** Medium

**Summary:** Implement export of schedule data to the JSON format consumed by the calendar display widget.

**Part of:** [META-005]

**Description:** The calendar widget renders schedule data from a JSON file. This work item
implements the export functionality that converts from the internal CRDT/field-system
format to the widget JSON display format (documented in `docs/widget-json-format.md`).

The export should use the public data structures (PanelTypeData, HotelRoomData, EventRoomData, etc.)
rather than InternalData, as these already contain synthesized fields like `inclusive_presenters`.
If public versions don't have data in the required format, computed fields should be added to the public data structure.

All items should use Uuid for identification. For break synthesis, Uuid v5 should be generated.
References between items should use Uuid instead of names or other IDs. Panels should have references
to both hotel and event rooms as separate records.

---

### [FEATURE-028] XLSX Spreadsheet Import

**Status:** Open

**Priority:** Medium

**Summary:** Import schedule data from the existing XLSX spreadsheet format.

**Part of:** [META-005]

**Description:** The primary data source is an Excel spreadsheet maintained by the convention
organizers. Import must handle the existing column layout.

---

### [FEATURE-029] XLSX Spreadsheet Export

**Status:** Open

**Priority:** Medium

**Summary:** Export schedule data back to the XLSX spreadsheet format.

**Part of:** [META-005]

**Description:** Export the schedule to an Excel spreadsheet matching the convention's expected
column layout, enabling round-trip with the import (FEATURE-028).

---

### [FEATURE-056] Synthesized Data Fields for Export

**Status:** Open

**Priority:** Medium

**Summary:** Add computed/synthesized fields to public data structures to support widget JSON export.

**Blocked By:** [FEATURE-019]

**Description:** The widget JSON export requires certain data that is not directly stored in the internal
entity structures but can be computed from existing fields. This work item adds computed
fields to the public data structures (PanelData, HotelRoomData, EventRoomData, etc.) to
make this data available for export.

Specific synthesized fields needed:

**PanelData:**

* `credited_presenters`: Formatted credit strings for display (hidePanelist, altPanelist, group resolution)
* `hotel_rooms`: Computed field that traverses event_rooms => hotel room edges (similar to inclusive_presenters traversal)

**Existing fields (no changes needed):**

* `inclusive_presenters`: Already exists as computed field (BFS over direct presenters + groups/members)
* `event_rooms`: Already exists as edge field to EventRoomEntityType

**PresenterData:**

* Verify existing fields meet export needs
* May need additional computed fields for bidirectional group membership

---

### [FEATURE-034] Peer-to-Peer Schedule Sync Protocol

**Status:** Open

**Priority:** Low

**Summary:** Define and implement the protocol for synchronizing schedule data between peers.

**Part of:** [META-008]

**Description:** Enable multiple users to edit the schedule concurrently and sync their changes
without a central server. Uses automerge's built-in sync protocol.

---

### [FEATURE-035] Merge Conflict Resolution UI

**Status:** Open

**Priority:** Low

**Summary:** Provide UI for reviewing and resolving merge conflicts after sync.

**Part of:** [META-008]

**Description:** When two peers edit the same field concurrently, the CRDT automatically picks
a winner (LWW), but the user should be able to review these decisions and
override them.

---

## Open META Items

### [META-001] Architecture Redesign: CRDT-backed Schedule System

**Status:** Open

**Priority:** High

**Summary:** Meta work item tracking the full multi-phase redesign of the schedule system.

**Blocked By:** [META-005], [META-006], [META-007], [META-008]

**Description:** Redesign the cosam-sched schedule system from the ground up with:

* **Entity/field system** using generic field descriptors (`FieldDescriptor<E>`)
  for clean, type-safe data structures — entity `Data` struct declarations are
  hand-written and visible; proc-macros may be used for boilerplate (trait
  impls, field accessor singletons, builders) as long as they do not hide the
  struct definitions
* **CRDT-backed storage** (automerge) enabling concurrent offline editing
  without a central database
* **Multi-year archive** support for jump-starting new conventions from prior years
* **Import/export** to and from the existing XLSX spreadsheet format
* **Widget JSON export** for the calendar display widget
* **Three application targets**: `cosam-convert` (format conversion),
  `cosam-modify` (CLI editing), `cosam-editor` (GUI editing)

All entity field infrastructure lives in a single `schedule-core` crate,
replacing the old `schedule-field`, `schedule-data`, and `schedule-macro` crates.

**Work Items:**

* META-002: Phase 1 — Foundation
* META-003: Phase 2 — Core Data Model (schedule-core)
* META-004: Phase 3 — CRDT Integration
* META-005: Phase 4 — File Formats & Import/Export
* META-006: Phase 5 — CLI Tools
* META-007: Phase 6 — GUI Editor
* META-008: Phase 7 — Sync & Multi-User

---

### [META-005] Phase 4 — File Formats & Import/Export

**Status:** Blocked

**Priority:** Medium

**Summary:** Phase tracker for internal file format, multi-year archive, widget JSON, and
XLSX import/export.

**Blocked By:** [META-004]

**Description:** Define and implement all file format support: the internal native format with
CRDT state, multi-year archive support, widget display JSON export, and
round-trip XLSX import/export for the convention spreadsheet workflow.

**Work Items:**

* FEATURE-025: Internal schedule file format (save/load)
* FEATURE-026: Multi-year schedule archive support
* FEATURE-027: Widget display JSON export
* FEATURE-028: XLSX spreadsheet import
* FEATURE-029: XLSX spreadsheet export

---

### [META-006] Phase 5 — CLI Tools

**Status:** Blocked

**Priority:** Low

**Summary:** Phase tracker for the cosam-convert and cosam-modify command-line applications.

**Blocked By:** [META-005]

**Description:** Implement the two CLI applications for format conversion and batch editing.
These applications wrap `schedule-core`'s import/export and edit command systems.

**Work Items:**

* CLI-030: cosam-convert: format conversion tool
* CLI-031: cosam-modify: CLI editing tool

---

### [META-007] Phase 6 — GUI Editor

**Status:** Blocked

**Priority:** Low

**Summary:** Phase tracker for the cosam-editor desktop GUI application.

**Blocked By:** [META-005]

**Description:** Select the GUI framework and implement the desktop schedule editor. Requires
the data model, edit command system, and file format support from earlier phases.

**Work Items:**

* EDITOR-032: cosam-editor: GUI framework selection and scaffold
* EDITOR-033: cosam-editor: schedule grid view and entity editing

---

### [META-008] Phase 7 — Sync & Multi-User

**Status:** Blocked

**Priority:** Low

**Summary:** Phase tracker for peer-to-peer schedule synchronization and conflict resolution.

**Blocked By:** [META-004]

**Description:** Implement the sync protocol and conflict resolution UI that allow multiple users
to exchange CRDT changes and reconcile concurrent edits to the same fields.

**Work Items:**

* FEATURE-034: Peer-to-peer schedule sync protocol
* FEATURE-035: Merge conflict resolution UI

---

---

[BUGFIX-045]: work-item/medium/BUGFIX-045.md
[CLI-030]: work-item/low/CLI-030.md
[CLI-031]: work-item/low/CLI-031.md
[EDITOR-032]: work-item/low/EDITOR-032.md
[EDITOR-033]: work-item/low/EDITOR-033.md
[FEATURE-009]: work-item/done/FEATURE-009.md
[FEATURE-010]: work-item/done/FEATURE-010.md
[FEATURE-011]: work-item/done/FEATURE-011.md
[FEATURE-012]: work-item/done/FEATURE-012.md
[FEATURE-013]: work-item/done/FEATURE-013.md
[FEATURE-014]: work-item/done/FEATURE-014.md
[FEATURE-015]: work-item/done/FEATURE-015.md
[FEATURE-016]: work-item/done/FEATURE-016.md
[FEATURE-017]: work-item/done/FEATURE-017.md
[FEATURE-018]: work-item/done/FEATURE-018.md
[FEATURE-019]: work-item/done/FEATURE-019.md
[FEATURE-020]: work-item/done/FEATURE-020.md
[FEATURE-021]: work-item/done/FEATURE-021.md
[FEATURE-022]: work-item/done/FEATURE-022.md
[FEATURE-023]: work-item/done/FEATURE-023.md
[FEATURE-024]: work-item/done/FEATURE-024.md
[FEATURE-025]: work-item/done/FEATURE-025.md
[FEATURE-026]: work-item/medium/FEATURE-026.md
[FEATURE-027]: work-item/medium/FEATURE-027.md
[FEATURE-028]: work-item/medium/FEATURE-028.md
[FEATURE-029]: work-item/medium/FEATURE-029.md
[FEATURE-034]: work-item/low/FEATURE-034.md
[FEATURE-035]: work-item/low/FEATURE-035.md
[FEATURE-038]: work-item/done/FEATURE-038.md
[FEATURE-043]: work-item/done/FEATURE-043.md
[FEATURE-046]: work-item/done/FEATURE-046.md
[FEATURE-050]: work-item/done/FEATURE-050.md
[FEATURE-051]: work-item/done/FEATURE-051.md
[FEATURE-056]: work-item/medium/FEATURE-056.md
[FEATURE-057]: work-item/done/FEATURE-057.md
[META-001]: work-item/meta/META-001.md
[META-002]: work-item/done/META-002.md
[META-003]: work-item/done/META-003.md
[META-004]: work-item/done/META-004.md
[META-005]: work-item/meta/META-005.md
[META-006]: work-item/meta/META-006.md
[META-007]: work-item/meta/META-007.md
[META-008]: work-item/meta/META-008.md
[META-048]: work-item/done/META-048.md
[REFACTOR-041]: work-item/done/REFACTOR-041.md
[REFACTOR-047]: work-item/done/REFACTOR-047.md
[REFACTOR-049]: work-item/done/REFACTOR-049.md
[REFACTOR-052]: work-item/done/REFACTOR-052.md
[REFACTOR-053]: work-item/done/REFACTOR-053.md
[REFACTOR-054]: work-item/done/REFACTOR-054.md
[REFACTOR-055]: work-item/done/REFACTOR-055.md
