# Cosplay America Schedule - Work Item

Updated on: Tue Apr 14 18:55:54 2026

## Completed

* [BUGFIX-044] `PanelEntityType::resolve_field_value` returns errors for `EntityIdentifier` and prefixed UUID string inputs even when the panel exists in storage.
* [FEATURE-002] Set up the Cargo workspace root and create skeleton crates for all planned components.
* [FEATURE-003] Implement the `#[derive(EntityFields)]` proc-macro in the `schedule-macro` crate.
* [FEATURE-004] Implement the field trait hierarchy, universal `FieldValue` enum, `FieldSet` registry,
and validation infrastructure.
* [FEATURE-005] Define the core domain entity structs using the `EntityFields` derive macro.
* [FEATURE-006] Implement UUID-based entity identity with compile-time type-safe ID wrappers.
* [FEATURE-007] Implement typed edge storage for entity-to-entity relationships.
* [FEATURE-008] Implement the `Schedule` struct and `EntityStorage` for managing all entities
and relationships.
* [FEATURE-010] Implement a command-based edit system with full undo/redo support.
* [FEATURE-011] Design the abstraction layer between the entity/field system and the CRDT backend.
* [FEATURE-034] Implement proper delegation pattern for `Schedule` convenience methods,
moving business logic to entity-specific implementations.
* [FEATURE-045] Rename combine-workitems.pl to work-item-update.pl and add a --create flag
to generate properly numbered placeholder work item files from templates.
* [META-025] Phase tracker for project foundation and Cargo workspace setup.
* [META-035] Update system documentation to describe the virtual edge design and create
work items for implementation phases REFACTOR-036, REFACTOR-037, REFACTOR-038.
* [REFACTOR-032] Rename the `from`/`to` endpoint naming on `DirectedEdge` to `left`/`right`
throughout the codebase.
* [REFACTOR-036] Add stored relationship fields to Panel, EventRoom, and Presenter; remove
edge-backed computed field closures.
* [REFACTOR-037] Add entity type insertion/removal hooks and per-relationship reverse lookup
indexes to EntityStorage; remove all edge HashMap and EdgeIndex infrastructure.
* [REFACTOR-038] Update Schedule convenience methods to use field access and reverse indexes;
remove DirectedEdge trait, edge macro attributes, edge EntityKind/EntityUUID
variants, and delete the five edge entity files.
* [REFACTOR-041] Replace `EdgeReverseMap<L, R>` with a generic bidirectional `EdgeMap<L, R>` and migrate all call sites.
* [REFACTOR-042] Add a method to entity data structs that returns the typed ID directly, avoiding repeated `XId::from_uuid(entity.uuid())` boilerplate.

---

## Superseded / Rejected

* [FEATURE-033] (Superseded) Unify `add_entity` and `add_edge` into a single insertion path, moving
EdgeIndex (and any per-type cache) maintenance responsibility into each
`EntityType` implementation.

---

## Summary of Open Items

**Total open items:** 21

* **Meta / Project-Level**
  * [META-001] Meta work item tracking the full multi-phase redesign of the schedule system. (Blocked by [META-026], [META-027], [META-028], [META-029], [META-030], [META-031])
  * [META-026] Phase tracker for the entity/field/macro system and core schedule data model.
  * [META-027] Phase tracker for adding CRDT-backed storage underneath the entity/field system.
  * [META-028] Phase tracker for internal file format, multi-year archive, widget JSON, and
XLSX import/export. (Blocked by [META-026], [META-027])
  * [META-029] Phase tracker for the cosam-convert and cosam-modify command-line applications. (Blocked by [META-028])
  * [META-030] Phase tracker for the cosam-editor desktop GUI application. (Blocked by [META-028])
  * [META-031] Phase tracker for peer-to-peer schedule synchronization and conflict resolution. (Blocked by [META-027])

* **Medium Priority**
  * [FEATURE-009] ([META-026]) Implement field-based search, matching, and bulk update operations.
  * [FEATURE-012] ([META-027]) Replace direct `HashMap` entity storage with CRDT-backed storage.
  * [FEATURE-013] ([META-027]) Implement change tracking, diff computation, and merge for CRDT documents.
  * [FEATURE-014] ([META-028]) Define and implement the native save/load format for schedule documents.
  * [FEATURE-015] ([META-028]) Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.
  * [FEATURE-016] ([META-028]) Implement export of schedule data to the JSON format consumed by the calendar
display widget.
  * [FEATURE-017] ([META-028]) Import schedule data from the existing XLSX spreadsheet format.
  * [FEATURE-018] ([META-028]) Export schedule data back to the XLSX spreadsheet format.

* **Low Priority**
  * [CLI-019] ([META-029]) CLI tool for converting between schedule file formats (XLSX, JSON, widget JSON).
  * [CLI-020] ([META-029]) CLI tool for making batch edits to schedule data from the command line.
  * [EDITOR-021] ([META-030]) Select the GUI framework for cosam-editor and create the application scaffold.
  * [EDITOR-022] ([META-030]) Implement the main schedule grid view and entity editing UI in cosam-editor.
  * [FEATURE-023] ([META-031]) Define and implement the protocol for synchronizing schedule data between peers.
  * [FEATURE-024] ([META-031]) Provide UI for reviewing and resolving merge conflicts after sync.

---

## Placeholders

*No placeholders — all stubs have been promoted.*

Use `perl scripts/work-item-update.pl --create <PREFIX>` to add new stubs.

---

## Open CLI Items

### [CLI-019] cosam-convert: Format Conversion Tool

**Status:** Open

**Priority:** Low

**Summary:** CLI tool for converting between schedule file formats (XLSX, JSON, widget JSON).

**Part of:** [META-029]

**Description:** `cosam-convert` is a command-line application for importing and exporting
schedule data between supported formats.

---

### [CLI-020] cosam-modify: CLI Editing Tool

**Status:** Open

**Priority:** Low

**Summary:** CLI tool for making batch edits to schedule data from the command line.

**Part of:** [META-029]

**Description:** `cosam-modify` provides command-line access to the edit system for scripted
or batch modifications to schedule data.

---

## Open EDITOR Items

### [EDITOR-021] cosam-editor: GUI Framework Selection and Scaffold

**Status:** Open

**Priority:** Low

**Summary:** Select the GUI framework for cosam-editor and create the application scaffold.

**Part of:** [META-030]

**Description:** Evaluate and select between GUI framework candidates, then create the initial
application structure.

---

### [EDITOR-022] cosam-editor: Schedule Grid View and Entity Editing

**Status:** Open

**Priority:** Low

**Summary:** Implement the main schedule grid view and entity editing UI in cosam-editor.

**Part of:** [META-030]

**Description:** The core editing experience for the GUI application.

---

## Open FEATURE Items

### [FEATURE-009] Query System

**Status:** In Progress

**Priority:** Medium

**Summary:** Implement field-based search, matching, and bulk update operations.

**Part of:** [META-026]

**Description:** The query system enables finding and updating entities using field-based
criteria rather than direct UUID access.

---

### [FEATURE-012] CRDT-backed Entity Storage

**Status:** Open

**Priority:** Medium

**Summary:** Replace direct `HashMap` entity storage with CRDT-backed storage.

**Part of:** [META-027]

**Description:** Implement the CRDT abstraction layer (FEATURE-011) with a concrete backend,
replacing the in-memory `HashMap<NonNilUuid, Data>` collections with
CRDT-backed equivalents.

---

### [FEATURE-013] Change Tracking and Merge Operations

**Status:** Open

**Priority:** Medium

**Summary:** Implement change tracking, diff computation, and merge for CRDT documents.

**Part of:** [META-027]

**Description:** Build on the CRDT storage (FEATURE-012) to provide:

---

### [FEATURE-014] Internal Schedule File Format

**Status:** Open

**Priority:** Medium

**Summary:** Define and implement the native save/load format for schedule documents.

**Part of:** [META-028]

**Description:** The internal format is used for saving and loading schedule state, including
CRDT history for sync support.

---

### [FEATURE-015] Multi-Year Schedule Archive Support

**Status:** Open

**Priority:** Medium

**Summary:** Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.

**Part of:** [META-028]

**Description:** A schedule archive contains multiple years of convention data in one file,
enabling:

* **Jump-start**: Copy entities from a prior year to pre-populate the next
  convention (e.g., recurring panels, returning presenters, same rooms)
* **Historical reference**: View past schedules alongside the current one
* **Widget display**: Optionally serve multi-year data to the calendar widget

---

### [FEATURE-016] Widget Display JSON Export

**Status:** Open

**Priority:** Medium

**Summary:** Implement export of schedule data to the JSON format consumed by the calendar
display widget.

**Part of:** [META-028]

**Description:** The calendar widget (in `widget/`) renders schedule data from a JSON file.
This work item defines and implements the new export format.

---

### [FEATURE-017] XLSX Spreadsheet Import

**Status:** Open

**Priority:** Medium

**Summary:** Import schedule data from the existing XLSX spreadsheet format.

**Part of:** [META-028]

**Description:** The primary data source is an Excel spreadsheet maintained by the convention
organizers. Import must handle the existing column layout documented in
`docs/spreadsheet-format.md`.

---

### [FEATURE-018] XLSX Spreadsheet Export

**Status:** Open

**Priority:** Medium

**Summary:** Export schedule data back to the XLSX spreadsheet format.

**Part of:** [META-028]

**Description:** Export the schedule to an Excel spreadsheet matching the convention's expected
column layout, enabling round-trip with the import (FEATURE-017).

---

### [FEATURE-023] Peer-to-Peer Schedule Sync Protocol

**Status:** Open

**Priority:** Low

**Summary:** Define and implement the protocol for synchronizing schedule data between peers.

**Part of:** [META-031]

**Description:** Enable multiple users to edit the schedule concurrently and sync their changes
without a central server.

---

### [FEATURE-024] Merge Conflict Resolution UI

**Status:** Open

**Priority:** Low

**Summary:** Provide UI for reviewing and resolving merge conflicts after sync.

**Part of:** [META-031]

**Description:** When two peers edit the same field concurrently, the CRDT automatically picks
a winner (typically last-writer-wins), but the user should be able to review
these decisions and override them.

---

## Open META Items

### [META-001] Architecture Redesign: CRDT-backed Schedule System

**Status:** Blocked

**Priority:** High

**Summary:** Meta work item tracking the full multi-phase redesign of the schedule system.

**Blocked By:** [META-026], [META-027], [META-028], [META-029], [META-030], [META-031]

**Description:** Redesign the cosam-sched schedule system from the ground up with:

* **Entity/field system** using a proc-macro (`#[derive(EntityFields)]`) for clean,
  type-safe data structures (ported from `feature/schedule-data` experiment)
* **CRDT-backed storage** enabling a handful of users to edit the schedule concurrently
  without a central database
* **Multi-year archive** support for jump-starting new conventions from prior years
* **Import/export** to and from the existing XLSX spreadsheet format
* **Widget JSON export** for the calendar display widget
* **Three application targets**: `cosam-convert` (format conversion), `cosam-modify`
  (CLI editing), `cosam-editor` (GUI editing)

**Work Items:**

* META-025: Phase 1 — Foundation
* META-026: Phase 2 — Core Data Model
* META-027: Phase 3 — CRDT Integration
* META-028: Phase 4 — File Formats & Import/Export
* META-029: Phase 5 — CLI Tools
* META-030: Phase 6 — GUI Editor
* META-031: Phase 7 — Sync & Multi-User

---

### [META-026] Phase 2 — Core Data Model

**Status:** In Progress

**Priority:** High

**Summary:** Phase tracker for the entity/field/macro system and core schedule data model.

**Description:** Port and refine the entity/field/macro system from `feature/schedule-data` into
the new workspace. This is the largest and most foundational phase.

**Work Items:**

* FEATURE-003: EntityFields derive macro (schedule-macro)
* FEATURE-004: Field system (traits, FieldValue, FieldSet, validation)
* FEATURE-005: Core entity definitions
* FEATURE-006: UUID-based identity and typed ID wrappers
* FEATURE-007: Edge/relationship system
* FEATURE-008: Schedule container and EntityStorage
* FEATURE-034: Schedule method delegation to entity types
* FEATURE-009: Query system

---

### [META-027] Phase 3 — CRDT Integration

**Status:** In Progress

**Priority:** Medium

**Summary:** Phase tracker for adding CRDT-backed storage underneath the entity/field system.

**Description:** Design and implement the CRDT abstraction layer and replace the direct HashMap
entity storage with a CRDT-backed equivalent. This enables concurrent offline
editing and eventual merge without a central server.

**Work Items:**

* FEATURE-010: Edit command system with undo/redo history
* FEATURE-011: CRDT abstraction layer design
* FEATURE-012: CRDT-backed entity storage
* FEATURE-013: Change tracking and merge operations

---

### [META-028] Phase 4 — File Formats & Import/Export

**Status:** Blocked

**Priority:** Medium

**Summary:** Phase tracker for internal file format, multi-year archive, widget JSON, and
XLSX import/export.

**Blocked By:** [META-026], [META-027]

**Description:** Define and implement all file format support: the internal native format with
CRDT state, multi-year archive support, widget display JSON export, and
round-trip XLSX import/export for the convention spreadsheet workflow.

**Work Items:**

* FEATURE-014: Internal schedule file format (save/load)
* FEATURE-015: Multi-year schedule archive support
* FEATURE-016: Widget display JSON export
* FEATURE-017: XLSX spreadsheet import
* FEATURE-018: XLSX spreadsheet export

---

### [META-029] Phase 5 — CLI Tools

**Status:** Blocked

**Priority:** Low

**Summary:** Phase tracker for the cosam-convert and cosam-modify command-line applications.

**Blocked By:** [META-028]

**Description:** Implement the two CLI applications for format conversion and batch editing.
These applications wrap the `schedule-data` crate's import/export and edit
command systems.

**Work Items:**

* CLI-019: cosam-convert: format conversion tool
* CLI-020: cosam-modify: CLI editing tool

---

### [META-030] Phase 6 — GUI Editor

**Status:** Blocked

**Priority:** Low

**Summary:** Phase tracker for the cosam-editor desktop GUI application.

**Blocked By:** [META-028]

**Description:** Select the GUI framework and implement the desktop schedule editor. Requires
the data model, edit command system, and file format support from earlier phases.

**Work Items:**

* EDITOR-021: cosam-editor: GUI framework selection and scaffold
* EDITOR-022: cosam-editor: schedule grid view and entity editing

---

### [META-031] Phase 7 — Sync & Multi-User

**Status:** Blocked

**Priority:** Low

**Summary:** Phase tracker for peer-to-peer schedule synchronization and conflict resolution.

**Blocked By:** [META-027]

**Description:** Implement the sync protocol and conflict resolution UI that allow multiple users
to exchange CRDT changes and reconcile concurrent edits to the same fields.

**Work Items:**

* FEATURE-023: Peer-to-peer schedule sync protocol
* FEATURE-024: Merge conflict resolution UI

---

---

[BUGFIX-044]: work-item/done/BUGFIX-044.md
[CLI-019]: work-item/low/CLI-019.md
[CLI-020]: work-item/low/CLI-020.md
[EDITOR-021]: work-item/low/EDITOR-021.md
[EDITOR-022]: work-item/low/EDITOR-022.md
[FEATURE-002]: work-item/done/FEATURE-002.md
[FEATURE-003]: work-item/done/FEATURE-003.md
[FEATURE-004]: work-item/done/FEATURE-004.md
[FEATURE-005]: work-item/done/FEATURE-005.md
[FEATURE-006]: work-item/done/FEATURE-006.md
[FEATURE-007]: work-item/done/FEATURE-007.md
[FEATURE-008]: work-item/done/FEATURE-008.md
[FEATURE-009]: work-item/medium/FEATURE-009.md
[FEATURE-010]: work-item/done/FEATURE-010.md
[FEATURE-011]: work-item/done/FEATURE-011.md
[FEATURE-012]: work-item/medium/FEATURE-012.md
[FEATURE-013]: work-item/medium/FEATURE-013.md
[FEATURE-014]: work-item/medium/FEATURE-014.md
[FEATURE-015]: work-item/medium/FEATURE-015.md
[FEATURE-016]: work-item/medium/FEATURE-016.md
[FEATURE-017]: work-item/medium/FEATURE-017.md
[FEATURE-018]: work-item/medium/FEATURE-018.md
[FEATURE-023]: work-item/low/FEATURE-023.md
[FEATURE-024]: work-item/low/FEATURE-024.md
[FEATURE-033]: work-item/rejected/FEATURE-033.md
[FEATURE-034]: work-item/done/FEATURE-034.md
[FEATURE-045]: work-item/done/FEATURE-045.md
[META-001]: work-item/meta/META-001.md
[META-025]: work-item/done/META-025.md
[META-026]: work-item/meta/META-026.md
[META-027]: work-item/meta/META-027.md
[META-028]: work-item/meta/META-028.md
[META-029]: work-item/meta/META-029.md
[META-030]: work-item/meta/META-030.md
[META-031]: work-item/meta/META-031.md
[META-035]: work-item/done/META-035.md
[REFACTOR-032]: work-item/done/REFACTOR-032.md
[REFACTOR-036]: work-item/done/REFACTOR-036.md
[REFACTOR-037]: work-item/done/REFACTOR-037.md
[REFACTOR-038]: work-item/done/REFACTOR-038.md
[REFACTOR-041]: work-item/done/REFACTOR-041.md
[REFACTOR-042]: work-item/done/REFACTOR-042.md
