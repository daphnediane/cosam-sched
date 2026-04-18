# Cosplay America Schedule - Work Item

Updated on: Fri Apr 17 20:35:20 2026

## Completed

* [FEATURE-009] Set up the Cargo workspace root and create skeleton application crates.
* [FEATURE-010] Implement the universal `FieldValue` enum, error types, and CRDT field type annotation.
* [FEATURE-011] Implement the field trait hierarchy and generic `FieldDescriptor` type that replaces the old proc-macro's generated per-field unit structs.
* [FEATURE-012] Implement UUID-based entity identity with compile-time type-safe ID wrappers.
* [FEATURE-013] Implement the static `FieldSet` registry for per-entity-type field metadata lookup.
* [FEATURE-014] Implement the PanelType entity as the first proof of concept for the no-proc-macro field system.
* [FEATURE-015] Port `TimeRange` and implement the Panel entity with stored and computed time fields.
* [FEATURE-016] Implement the remaining core entity data structs and field descriptors.
* [FEATURE-043] Add a `verify` callback to `FieldDescriptor` for cross-field consistency checks after batch writes to computed fields.
* [FEATURE-050] Add `FieldTypeItem` (scalar type tags) and `FieldType` (`Single`/`Optional`/`List`
wrappers) to `value.rs` as `Copy` type-level mirrors of `FieldValueItem`/`FieldValue`.
* [META-002] Phase tracker for project foundation and Cargo workspace setup.
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

---

## Summary of Open Items

**Total open items:** 33

* **Meta / Project-Level**
  * [META-001] Meta work item tracking the full multi-phase redesign of the schedule system. (Blocked by [META-003], [META-004], [META-005], [META-006], [META-007], [META-008])
  * [META-003] Phase tracker for the entity/field system and core schedule data model in schedule-core. (Blocked by [META-002])
  * [META-004] Phase tracker for adding CRDT-backed storage underneath the entity/field system. (Blocked by [META-003])
  * [META-005] Phase tracker for internal file format, multi-year archive, widget JSON, and
XLSX import/export. (Blocked by [META-003], [META-004])
  * [META-006] Phase tracker for the cosam-convert and cosam-modify command-line applications. (Blocked by [META-005])
  * [META-007] Phase tracker for the cosam-editor desktop GUI application. (Blocked by [META-005])
  * [META-008] Phase tracker for peer-to-peer schedule synchronization and conflict resolution. (Blocked by [META-004])
  * [META-048] Restructure `FieldValue` with proper cardinality, add `FieldTypeItem`/`FieldType`
enums, wire `FieldType` into `FieldDescriptor`, and implement the generic
`FieldValueConverter` system from IDEA-038.

* **High Priority**
  * [FEATURE-018] ([META-003]) Implement typed relationship storage for entity-to-entity relationships.
  * [FEATURE-019] ([META-003]) Implement the `Schedule` struct and `EntityStorage` for managing all entities and relationships.
  * [FEATURE-021] ([META-003]) Implement a command-based edit system with full undo/redo support.
  * [FEATURE-038] ([META-048]) Add a type-safe `FieldValueConverter<M>` trait and driver functions for converting
`FieldValue` inputs to typed Rust outputs via a work-queue iteration pattern.
  * [FEATURE-051] ([META-048]) Add a `field_type: FieldType` field to `FieldDescriptor` and populate it in all
existing static field descriptors across every entity file.
  * [REFACTOR-041] Replace the `EntityKind` enum with direct use of `EntityType::TYPE_NAME` strings,
following the v10-try3 design. This eliminates the central enum that required
modification for every new entity type.

* **Medium Priority**
  * [BUGFIX-045] In `scratch/field_update_logic.rs`, duration values are incorrectly stored as `FieldValue::Integer(minutes)` instead of `FieldValue::Duration(Duration)`.
  * [FEATURE-017] ([META-003]) Implement entity builders for constructing entity data with UUID assignment.
  * [FEATURE-020] ([META-003]) Implement field-based search, matching, and bulk update operations.
  * [FEATURE-022] ([META-004]) Design the abstraction layer between the entity/field system and the CRDT backend.
  * [FEATURE-023] ([META-004]) Replace direct `HashMap` entity storage with CRDT-backed storage using automerge.
  * [FEATURE-024] ([META-004]) Implement change tracking, diff computation, and merge for CRDT documents.
  * [FEATURE-025] ([META-005]) Define and implement the native save/load format for schedule documents.
  * [FEATURE-026] ([META-005]) Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.
  * [FEATURE-027] ([META-005]) Implement export of schedule data to the JSON format consumed by the calendar display widget.
  * [FEATURE-028] ([META-005]) Import schedule data from the existing XLSX spreadsheet format.
  * [FEATURE-029] ([META-005]) Export schedule data back to the XLSX spreadsheet format.
  * [FEATURE-046] ([META-003]) Add `FieldSet::write_multiple()` for atomic batch field updates with verification support.
  * [REFACTOR-055] Add `define_field!` macro to bundle hand-written `FieldDescriptor` statics with
`inventory::submit!`, and add `IntoFieldValue` trait hierarchy for type-deduced
`field_value!(expr)` construction.

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

### [FEATURE-018] Relationship Storage (EdgeMap / Reverse Indexes)

**Status:** Open

**Priority:** High

**Summary:** Implement typed relationship storage for entity-to-entity relationships.

**Part of:** [META-003]

**Description:** Relationships between entities are stored as typed `Vec<EntityId>` fields on the
owning entity (virtual edges, not edge entities). Reverse indexes maintain
bidirectional lookup.

---

### [FEATURE-019] Schedule Container + EntityStorage

**Status:** Open

**Priority:** High

**Summary:** Implement the `Schedule` struct and `EntityStorage` for managing all entities and relationships.

**Part of:** [META-003]

**Description:** The `Schedule` struct is the top-level container holding:

* `EntityStorage` — typed collections for each entity type
* EdgeMap instances for all relationship types
* Entity registry (`HashMap<NonNilUuid, EntityKind>`) for UUID → kind lookup
* `ScheduleMetadata` — version, timestamps, generator info, schedule ID

Schedule is a **proxy, not an owner** — entity types own their storage; Schedule
provides UUID-keyed coordination.

---

### [FEATURE-021] Edit Command System With Undo/Redo History

**Status:** Open

**Priority:** High

**Summary:** Implement a command-based edit system with full undo/redo support.

**Part of:** [META-003]

**Description:** All mutations to the schedule go through an edit command system that captures
changes as reversible operations, enabling undo/redo in both CLI and GUI contexts.

---

### [FEATURE-038] FEATURE-038: FieldValueConverter System

**Status:** Open

**Priority:** High

**Summary:** Add a type-safe `FieldValueConverter<M>` trait and driver functions for converting
`FieldValue` inputs to typed Rust outputs via a work-queue iteration pattern.

**Part of:** [META-048]

**Description:** Promoted from IDEA-038. Implements the generic conversion system needed by the import
pipeline (e.g., tagged presenter `"P:Name"` → `EntityId<PresenterEntityType>` with
rank assignment).

---

### [FEATURE-051] FEATURE-051: Add field_type to FieldDescriptor

**Status:** Open

**Priority:** High

**Summary:** Add a `field_type: FieldType` field to `FieldDescriptor` and populate it in all
existing static field descriptors across every entity file.

**Part of:** [META-048]

**Description:** `FieldDescriptor` currently has `crdt_type: CrdtFieldType` to declare CRDT routing,
but no field for the value's logical type. Adding `field_type: FieldType` allows
callers (converters, importers, UI) to know what type a field expects without
calling read/write.

---

### [FEATURE-017] Builder Pattern

**Status:** Open

**Priority:** Medium

**Summary:** Implement entity builders for constructing entity data with UUID assignment.

**Part of:** [META-003]

**Description:** The old proc-macro generated per-entity builders with `with_*` setters and
`build()` methods. Without the macro, builders need explicit implementation.

---

### [FEATURE-020] Query System

**Status:** Open

**Priority:** Medium

**Summary:** Implement field-based search, matching, and bulk update operations.

**Part of:** [META-003]

**Description:** The query system enables finding and updating entities using field-based
criteria rather than direct UUID access.

---

### [FEATURE-022] CRDT Abstraction Layer Design

**Status:** Open

**Priority:** Medium

**Summary:** Design the abstraction layer between the entity/field system and the CRDT backend.

**Part of:** [META-004]

**Description:** Before integrating a specific CRDT library, define the abstraction boundary so
the entity system doesn't depend directly on CRDT internals.

Uses the `CrdtFieldType` annotations (Scalar, Text, List, Derived) on field
descriptors to drive write-through and materialization without per-entity tables.

See `docs/crdt-design.md` for the settled design decisions.

---

### [FEATURE-023] CRDT-backed Entity Storage

**Status:** Open

**Priority:** Medium

**Summary:** Replace direct `HashMap` entity storage with CRDT-backed storage using automerge.

**Part of:** [META-004]

**Description:** Implement the CRDT abstraction layer (FEATURE-022) with automerge as the
concrete backend, replacing in-memory `HashMap<NonNilUuid, Data>` collections
with CRDT-backed equivalents.

Write-through: field writes propagate to automerge document based on
`CrdtFieldType`. Materialization: on load, entities are reconstructed from
CRDT state using `crdt_fields` metadata on each `FieldSet`.

---

### [FEATURE-024] Change Tracking and Merge Operations

**Status:** Open

**Priority:** Medium

**Summary:** Implement change tracking, diff computation, and merge for CRDT documents.

**Part of:** [META-004]

**Description:** Build on the CRDT storage (FEATURE-023) to provide:

* Change tracking between document states
* Diff computation showing what changed between two versions
* Merge operations for combining concurrent changes from multiple actors
* Conflict surfacing for concurrent scalar edits (LWW with visibility)

---

### [FEATURE-025] Internal Schedule File Format

**Status:** Open

**Priority:** Medium

**Summary:** Define and implement the native save/load format for schedule documents.

**Part of:** [META-005]

**Description:** The internal format is used for saving and loading schedule state, including
CRDT history for sync support.

---

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
defines and implements the export format (clean break from v9/v10 format).

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

### [FEATURE-046] FEATURE-046: Bulk Field Updates (FieldSet::write_multiple)

**Status:** Open

**Priority:** Medium

**Summary:** Add `FieldSet::write_multiple()` for atomic batch field updates with verification support.

**Part of:** [META-003]

**Description:** Atomic batch update method for setting multiple fields on a single entity. Essential for interdependent computed fields (e.g., `start_time`, `end_time`, `duration`) where multiple fields must be written and then verified together.

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

**Blocked By:** [META-003], [META-004], [META-005], [META-006], [META-007], [META-008]

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

### [META-003] Phase 2 — Core Data Model (schedule-core)

**Status:** Open

**Priority:** High

**Summary:** Phase tracker for the entity/field system and core schedule data model in schedule-core.

**Blocked By:** [META-002]

**Description:** Build the `schedule-core` crate containing the complete entity/field system.
Entity `Data` struct declarations are hand-written and visible — macros must not
obscure them. Proc-macros and `macro_rules!` may be used for boilerplate (trait
impls, field accessor singletons, builders). `CrdtFieldType` annotations are
baked in from the start.

**Work Items:**

* FEATURE-010: FieldValue, error types, CrdtFieldType
* FEATURE-011: Field traits + FieldDescriptor
* FEATURE-012: EntityType, EntityId, EntityKind
* FEATURE-013: FieldSet registry
* FEATURE-014: PanelType entity (proof of concept)
* FEATURE-015: TimeRange + Panel entity
* FEATURE-016: Presenter + EventRoom + HotelRoom entities
* FEATURE-017: Builder pattern
* FEATURE-018: Relationship storage (EdgeMap / reverse indexes)
* FEATURE-019: Schedule container + EntityStorage
* FEATURE-020: Query system
* FEATURE-043: Field verification callbacks (verify_fn)
* FEATURE-046: Bulk field updates (write_multiple)
* FEATURE-021: Edit command system with undo/redo

---

### [META-048] META-048: FieldValue / FieldType / Converter Overhaul

**Status:** Open

**Priority:** High

**Summary:** Restructure `FieldValue` with proper cardinality, add `FieldTypeItem`/`FieldType`
enums, wire `FieldType` into `FieldDescriptor`, and implement the generic
`FieldValueConverter` system from IDEA-038.

**Description:** The current `FieldValue` enum conflates scalar values, lists, and absence into a
single flat enum. This overhaul splits it into `FieldValueItem` (scalars) and
`FieldValue` (`Single`/`List` wrappers), adds a matching `FieldTypeItem` /
`FieldType` pair for type-level declarations, wires `FieldType` into field descriptors,
and finally adds the type-safe `FieldValueConverter` system for import pipelines.

The `EntityIdentifier` ad-hoc enum is also removed; entity references are unified
under `FieldValueItem::EntityIdentifier(RuntimeEntityId)`.

**Note**: REFACTOR-049 completed the FieldValue restructuring. The actual implementation
uses `Single`/`List` wrappers (without an `Optional` variant) and `EntityIdentifier`
as the variant name (not `EntityId`). Absent optional fields return `None` from
read functions; empty lists return `FieldValue::List(vec![])`.

**Work Items:**

* REFACTOR-049: Restructure FieldValue → FieldValueItem + cardinality
* FEATURE-050: Add FieldTypeItem and FieldType enums
* FEATURE-051: Add field\_type to FieldDescriptor
* FEATURE-038: FieldValueConverter system

---

### [META-004] Phase 3 — CRDT Integration

**Status:** Blocked

**Priority:** Medium

**Summary:** Phase tracker for adding CRDT-backed storage underneath the entity/field system.

**Blocked By:** [META-003]

**Description:** Design and implement the CRDT abstraction layer and replace the direct HashMap
entity storage with a CRDT-backed equivalent. This enables concurrent offline
editing and eventual merge without a central server.

The integration leverages field-level CRDT semantics (`CrdtFieldType` on each
field descriptor) to avoid per-entity boilerplate. Write-through and materialize
patterns iterate the field metadata — no per-entity-kind tables needed.

**Work Items:**

* FEATURE-022: CRDT abstraction layer design
* FEATURE-023: CRDT-backed entity storage
* FEATURE-024: Change tracking and merge operations

---

### [META-005] Phase 4 — File Formats & Import/Export

**Status:** Blocked

**Priority:** Medium

**Summary:** Phase tracker for internal file format, multi-year archive, widget JSON, and
XLSX import/export.

**Blocked By:** [META-003], [META-004]

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

## Open REFACTOR Items

### [REFACTOR-041] REFACTOR-041: Remove EntityKind enum, use type strings directly

**Status:** Done

**Priority:** High

**Summary:** Replace the `EntityKind` enum with direct use of `EntityType::TYPE_NAME` strings,
following the v10-try3 design. This eliminates the central enum that required
modification for every new entity type.

**Description:** The `EntityKind` enum in `entity.rs` served two purposes:

1. Tagging `RuntimeEntityId` with the entity type for dynamic dispatch
2. Providing v5 UUID namespaces for deterministic ID generation

Both are now handled without a central enum:

* `RuntimeEntityId` uses `type_name: String` (from `EntityType::TYPE_NAME`)
* `EntityType::uuid_namespace()` provides per-type v5 namespaces directly
  on the trait (returns `&'static Uuid` via internal `LazyLock`)

---

### [REFACTOR-055] REFACTOR-055: Unify field registration via `define_field!` and add `IntoFieldValue` trait

**Status:** In Progress

**Priority:** Medium

**Summary:** Add `define_field!` macro to bundle hand-written `FieldDescriptor` statics with
`inventory::submit!`, and add `IntoFieldValue` trait hierarchy for type-deduced
`field_value!(expr)` construction.

**Description:** Two related improvements to reduce boilerplate and prevent silent omission of fields from
the registry:

1. **`define_field!` macro** — hand-written `FieldDescriptor` statics currently require a
   separate `inventory::submit!` call after each one. Forgetting it silently omits the
   field from the registry with no compiler error. The new `define_field!` macro wraps
   both into a single declaration. Affects 8 hand-written statics across `panel.rs`,
   `presenter.rs`, `event_room.rs`, and `panel_type.rs`.

2. **`IntoFieldValue` trait hierarchy** — constructing `FieldValue` values currently
   requires naming the type variant explicitly (`field_string!`, `field_datetime!`, etc.)
   because `macro_rules!` cannot dispatch on types. Adding `IntoFieldValueItem` +
   `IntoFieldValue` traits with blanket `impl`s for all scalar types, `Option<T>`, and
   `Vec<T>` allows a single `field_value!(expr)` macro arm to select the right variant
   via Rust's trait dispatch.

No proc macros — both improvements use `macro_rules!` + traits, preserving full
visibility of `FieldDescriptor` literal bodies.

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
[FEATURE-017]: work-item/medium/FEATURE-017.md
[FEATURE-018]: work-item/high/FEATURE-018.md
[FEATURE-019]: work-item/high/FEATURE-019.md
[FEATURE-020]: work-item/medium/FEATURE-020.md
[FEATURE-021]: work-item/high/FEATURE-021.md
[FEATURE-022]: work-item/medium/FEATURE-022.md
[FEATURE-023]: work-item/medium/FEATURE-023.md
[FEATURE-024]: work-item/medium/FEATURE-024.md
[FEATURE-025]: work-item/medium/FEATURE-025.md
[FEATURE-026]: work-item/medium/FEATURE-026.md
[FEATURE-027]: work-item/medium/FEATURE-027.md
[FEATURE-028]: work-item/medium/FEATURE-028.md
[FEATURE-029]: work-item/medium/FEATURE-029.md
[FEATURE-034]: work-item/low/FEATURE-034.md
[FEATURE-035]: work-item/low/FEATURE-035.md
[FEATURE-038]: work-item/high/FEATURE-038.md
[FEATURE-043]: work-item/done/FEATURE-043.md
[FEATURE-046]: work-item/medium/FEATURE-046.md
[FEATURE-050]: work-item/done/FEATURE-050.md
[FEATURE-051]: work-item/high/FEATURE-051.md
[META-001]: work-item/meta/META-001.md
[META-002]: work-item/done/META-002.md
[META-003]: work-item/meta/META-003.md
[META-004]: work-item/meta/META-004.md
[META-005]: work-item/meta/META-005.md
[META-006]: work-item/meta/META-006.md
[META-007]: work-item/meta/META-007.md
[META-008]: work-item/meta/META-008.md
[META-048]: work-item/meta/META-048.md
[REFACTOR-041]: work-item/high/REFACTOR-041.md
[REFACTOR-047]: work-item/done/REFACTOR-047.md
[REFACTOR-049]: work-item/done/REFACTOR-049.md
[REFACTOR-052]: work-item/done/REFACTOR-052.md
[REFACTOR-053]: work-item/done/REFACTOR-053.md
[REFACTOR-054]: work-item/done/REFACTOR-054.md
[REFACTOR-055]: work-item/medium/REFACTOR-055.md
