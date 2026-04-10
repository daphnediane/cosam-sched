# Cosplay America Schedule - Work Item

Updated on: Fri Apr 10 15:50:57 2026

## Summary of Open Items

**Total open items:** 31

* **Meta / Project-Level**
  * [META-001] Meta work item tracking the full multi-phase redesign of the schedule system. (Blocked by [META-025], [META-026], [META-027], [META-028], [META-029], [META-030], [META-031])
  * [META-025] Phase tracker for project foundation and Cargo workspace setup.
  * [META-026] Phase tracker for the entity/field/macro system and core schedule data model. (Blocked by [META-025])
  * [META-027] Phase tracker for adding CRDT-backed storage underneath the entity/field system. (Blocked by [META-026])
  * [META-028] Phase tracker for internal file format, multi-year archive, widget JSON, and
XLSX import/export. (Blocked by [META-026], [META-027])
  * [META-029] Phase tracker for the cosam-convert and cosam-modify command-line applications. (Blocked by [META-028])
  * [META-030] Phase tracker for the cosam-editor desktop GUI application. (Blocked by [META-028])
  * [META-031] Phase tracker for peer-to-peer schedule synchronization and conflict resolution. (Blocked by [META-027])

* **High Priority**
  * [FEATURE-002] ([META-025]) Set up the Cargo workspace root and create skeleton crates for all planned components.
  * [FEATURE-003] ([META-026]) Implement the `#[derive(EntityFields)]` proc-macro in the `schedule-macro` crate.
  * [FEATURE-004] ([META-026]) Implement the field trait hierarchy, universal `FieldValue` enum, `FieldSet` registry,
and validation infrastructure.
  * [FEATURE-005] ([META-026]) Define the core domain entity structs using the `EntityFields` derive macro.
  * [FEATURE-006] ([META-026]) Implement UUID-based entity identity with compile-time type-safe ID wrappers.
  * [FEATURE-007] ([META-026]) Implement typed edge storage for entity-to-entity relationships.
  * [FEATURE-008] ([META-026]) Implement the `Schedule` struct and `EntityStorage` for managing all entities
and relationships.
  * [FEATURE-010] ([META-026]) Implement a command-based edit system with full undo/redo support.

* **Medium Priority**
  * [FEATURE-009] ([META-026]) Implement field-based search, matching, and bulk update operations.
  * [FEATURE-011] ([META-027]) Design the abstraction layer between the entity/field system and the CRDT backend.
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

## Next Available IDs

The following ID numbers are available for new items:

**Available:** 032, 033, 034, 035, 036, 037, 038, 039, 040, 041

**Highest used:** 31

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

### [FEATURE-002] Cargo Workspace Setup With Crate Skeletons

**Status:** Open

**Priority:** High

**Summary:** Set up the Cargo workspace root and create skeleton crates for all planned components.

**Part of:** [META-025]

**Description:** Initialize `cosam_sched` as a Cargo workspace with the following layout:

```text
Cargo.toml              (workspace root)
crates/
  schedule-data/        (core data model, entities, fields, storage)
  schedule-macro/       (proc-macro crate for #[derive(EntityFields)])
apps/
  cosam-convert/        (format conversion CLI)
  cosam-modify/         (CLI editing tool)
  cosam-editor/         (GUI editor — skeleton only)
```

Each crate should have:

- `Cargo.toml` with `license = "BSD-2-Clause"` and `authors = ["Daphne Pfister"]`
- Copyright header in all source files
- Minimal `lib.rs` or `main.rs` that compiles

---

### [FEATURE-003] EntityFields Derive Macro

**Status:** Open

**Priority:** High

**Summary:** Implement the `#[derive(EntityFields)]` proc-macro in the `schedule-macro` crate.

**Part of:** [META-026]

**Description:** Port and refine the `EntityFields` derive macro from the `feature/schedule-data`
experiment. The macro generates boilerplate for the entity/field system so that
entity structs remain clean and declarative.

---

### [FEATURE-004] Field System: Traits, FieldValue, FieldSet, Validation

**Status:** Open

**Priority:** High

**Summary:** Implement the field trait hierarchy, universal `FieldValue` enum, `FieldSet` registry,
and validation infrastructure.

**Part of:** [META-026]

**Description:** The field system provides type-safe, generic access to entity fields for editing,
querying, import/export, and display.

---

### [FEATURE-005] Core Entity Definitions

**Status:** Open

**Priority:** High

**Summary:** Define the core domain entity structs using the `EntityFields` derive macro.

**Part of:** [META-026]

**Description:** Implement entity definitions for the schedule domain model:

- **Panel** — A scheduled event/session with name, description, timing, flags,
  and computed fields for presenters, room, and panel type
- **Presenter** — A person or group that presents at events
- **EventRoom** — A physical or virtual space where events occur
- **HotelRoom** — A hotel room that may host an event room
- **PanelType** — A category/type classification for panels (e.g., "Gaming",
  "Workshop", "Panel")
- **PresenterRank** — Rank/tier for presenters (Guest, Staff, etc.)

Each entity uses `#[derive(EntityFields)]` with appropriate field annotations
for display names, aliases, required fields, and indexable fields.

---

### [FEATURE-006] UUID-based Identity and Typed ID Wrappers

**Status:** Open

**Priority:** High

**Summary:** Implement UUID-based entity identity with compile-time type-safe ID wrappers.

**Part of:** [META-026]

**Description:** All entities are identified by `uuid::NonNilUuid` (v7 for new entities, v5 for
deterministic edge identities).

---

### [FEATURE-007] Edge/Relationship System

**Status:** Open

**Priority:** High

**Summary:** Implement typed edge storage for entity-to-entity relationships.

**Part of:** [META-026]

**Description:** Relationships between entities are modeled as typed edges with their own storage
and query capabilities. Edge types include:

- **PanelToPresenter** — which presenters are on which panels
- **PresenterToGroup** — presenter group membership (with `always_grouped` and
  `always_shown_in_group` flags)
- **PanelToEventRoom** — which room a panel is assigned to
- **PanelToPanelType** — which category a panel belongs to
- **EventRoomToHotelRoom** — physical room mapping

---

### [FEATURE-008] Schedule Container and EntityStorage

**Status:** Open

**Priority:** High

**Summary:** Implement the `Schedule` struct and `EntityStorage` for managing all entities
and relationships.

**Part of:** [META-026]

**Description:** The `Schedule` struct is the top-level container holding:

- `EntityStorage` — typed collections for each entity type
- Edge storages for all relationship types
- Entity registry (`HashMap<NonNilUuid, EntityKind>`) for UUID → kind lookup
- `ScheduleMetadata` — version, timestamps, generator info, schedule ID
- Edge entity query engine with caching

---

### [FEATURE-010] Edit Command System With Undo/Redo History

**Status:** Open

**Priority:** High

**Summary:** Implement a command-based edit system with full undo/redo support.

**Part of:** [META-026]

**Description:** All mutations to the schedule go through an edit command system that captures
changes as reversible operations, enabling undo/redo in both CLI and GUI contexts.

---

### [FEATURE-009] Query System

**Status:** Open

**Priority:** Medium

**Summary:** Implement field-based search, matching, and bulk update operations.

**Part of:** [META-026]

**Description:** The query system enables finding and updating entities using field-based
criteria rather than direct UUID access.

---

### [FEATURE-011] CRDT Abstraction Layer Design

**Status:** Open

**Priority:** Medium

**Summary:** Design the abstraction layer between the entity/field system and the CRDT backend.

**Part of:** [META-027]

**Description:** Before integrating a specific CRDT library, define the abstraction boundary so
the entity system doesn't depend directly on CRDT internals.

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

- **Jump-start**: Copy entities from a prior year to pre-populate the next
  convention (e.g., recurring panels, returning presenters, same rooms)
- **Historical reference**: View past schedules alongside the current one
- **Widget display**: Optionally serve multi-year data to the calendar widget

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

**Blocked By:** [META-025], [META-026], [META-027], [META-028], [META-029], [META-030], [META-031]

**Description:** Redesign the cosam-sched schedule system from the ground up with:

- **Entity/field system** using a proc-macro (`#[derive(EntityFields)]`) for clean,
  type-safe data structures (ported from `feature/schedule-data` experiment)
- **CRDT-backed storage** enabling a handful of users to edit the schedule concurrently
  without a central database
- **Multi-year archive** support for jump-starting new conventions from prior years
- **Import/export** to and from the existing XLSX spreadsheet format
- **Widget JSON export** for the calendar display widget
- **Three application targets**: `cosam-convert` (format conversion), `cosam-modify`
  (CLI editing), `cosam-editor` (GUI editing)

**Work Items:**

- META-025: Phase 1 — Foundation
- META-026: Phase 2 — Core Data Model
- META-027: Phase 3 — CRDT Integration
- META-028: Phase 4 — File Formats & Import/Export
- META-029: Phase 5 — CLI Tools
- META-030: Phase 6 — GUI Editor
- META-031: Phase 7 — Sync & Multi-User

---

### [META-025] Phase 1 — Foundation

**Status:** Blocked

**Priority:** High

**Summary:** Phase tracker for project foundation and Cargo workspace setup.

**Description:** Establish the Cargo workspace and skeleton crates so all subsequent phases have
a place to land.

**Work Items:**

- FEATURE-002: Cargo workspace setup with crate skeletons

---

### [META-026] Phase 2 — Core Data Model

**Status:** Blocked

**Priority:** High

**Summary:** Phase tracker for the entity/field/macro system and core schedule data model.

**Blocked By:** [META-025]

**Description:** Port and refine the entity/field/macro system from `feature/schedule-data` into
the new workspace. This is the largest and most foundational phase.

**Work Items:**

- FEATURE-003: EntityFields derive macro (schedule-macro)
- FEATURE-004: Field system (traits, FieldValue, FieldSet, validation)
- FEATURE-005: Core entity definitions
- FEATURE-006: UUID-based identity and typed ID wrappers
- FEATURE-007: Edge/relationship system
- FEATURE-008: Schedule container and EntityStorage
- FEATURE-009: Query system
- FEATURE-010: Edit command system with undo/redo history

---

### [META-027] Phase 3 — CRDT Integration

**Status:** Blocked

**Priority:** Medium

**Summary:** Phase tracker for adding CRDT-backed storage underneath the entity/field system.

**Blocked By:** [META-026]

**Description:** Design and implement the CRDT abstraction layer and replace the direct HashMap
entity storage with a CRDT-backed equivalent. This enables concurrent offline
editing and eventual merge without a central server.

**Work Items:**

- FEATURE-011: CRDT abstraction layer design
- FEATURE-012: CRDT-backed entity storage
- FEATURE-013: Change tracking and merge operations

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

- FEATURE-014: Internal schedule file format (save/load)
- FEATURE-015: Multi-year schedule archive support
- FEATURE-016: Widget display JSON export
- FEATURE-017: XLSX spreadsheet import
- FEATURE-018: XLSX spreadsheet export

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

- CLI-019: cosam-convert: format conversion tool
- CLI-020: cosam-modify: CLI editing tool

---

### [META-030] Phase 6 — GUI Editor

**Status:** Blocked

**Priority:** Low

**Summary:** Phase tracker for the cosam-editor desktop GUI application.

**Blocked By:** [META-028]

**Description:** Select the GUI framework and implement the desktop schedule editor. Requires
the data model, edit command system, and file format support from earlier phases.

**Work Items:**

- EDITOR-021: cosam-editor: GUI framework selection and scaffold
- EDITOR-022: cosam-editor: schedule grid view and entity editing

---

### [META-031] Phase 7 — Sync & Multi-User

**Status:** Blocked

**Priority:** Low

**Summary:** Phase tracker for peer-to-peer schedule synchronization and conflict resolution.

**Blocked By:** [META-027]

**Description:** Implement the sync protocol and conflict resolution UI that allow multiple users
to exchange CRDT changes and reconcile concurrent edits to the same fields.

**Work Items:**

- FEATURE-023: Peer-to-peer schedule sync protocol
- FEATURE-024: Merge conflict resolution UI

---

---

[CLI-019]: work-item/low/CLI-019.md
[CLI-020]: work-item/low/CLI-020.md
[EDITOR-021]: work-item/low/EDITOR-021.md
[EDITOR-022]: work-item/low/EDITOR-022.md
[FEATURE-002]: work-item/high/FEATURE-002.md
[FEATURE-003]: work-item/high/FEATURE-003.md
[FEATURE-004]: work-item/high/FEATURE-004.md
[FEATURE-005]: work-item/high/FEATURE-005.md
[FEATURE-006]: work-item/high/FEATURE-006.md
[FEATURE-007]: work-item/high/FEATURE-007.md
[FEATURE-008]: work-item/high/FEATURE-008.md
[FEATURE-009]: work-item/medium/FEATURE-009.md
[FEATURE-010]: work-item/high/FEATURE-010.md
[FEATURE-011]: work-item/medium/FEATURE-011.md
[FEATURE-012]: work-item/medium/FEATURE-012.md
[FEATURE-013]: work-item/medium/FEATURE-013.md
[FEATURE-014]: work-item/medium/FEATURE-014.md
[FEATURE-015]: work-item/medium/FEATURE-015.md
[FEATURE-016]: work-item/medium/FEATURE-016.md
[FEATURE-017]: work-item/medium/FEATURE-017.md
[FEATURE-018]: work-item/medium/FEATURE-018.md
[FEATURE-023]: work-item/low/FEATURE-023.md
[FEATURE-024]: work-item/low/FEATURE-024.md
[META-001]: work-item/meta/META-001.md
[META-025]: work-item/meta/META-025.md
[META-026]: work-item/meta/META-026.md
[META-027]: work-item/meta/META-027.md
[META-028]: work-item/meta/META-028.md
[META-029]: work-item/meta/META-029.md
[META-030]: work-item/meta/META-030.md
[META-031]: work-item/meta/META-031.md
