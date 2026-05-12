# Cosplay America Schedule - Work Item

Updated on: Mon May 11 22:05:19 2026

## Completed

* [BUGFIX-072] Several homogeneous-edge queries on the presenter member/group relationship
use the near/far field pair swapped from what their docs and field names
advertise. Introduce `FIELD_*_NEAR` / `FIELD_*_FAR` aliases to make the
intent explicit at each call site and fix the inverted queries.
* [BUGFIX-073] `PanelInternalData::time_slot` has no CRDT backing field, so panel start /
end / duration are not mirrored to the Automerge document and are lost
through any save → load (or merge) round trip.
* [BUGFIX-076] The edge_field_properties macro currently sets add_fn to AddEdge for all target edges without checking if the edge has multiple source fields. This should return None for target edges with multiple sources since add_edge doesn't support multi-source edges yet.
* [BUGFIX-078] The `callback_field_properties!` macro generates `CrdtFieldType::Scalar` for all fields, but it should generate `Derived` for fields with custom read/write callbacks that project from internal state (like Panel's time_slot projections).
* [BUGFIX-086] Room filter chips are blank and hotel room context is absent because the new
export format uses camelCase field names that the widget doesn't handle.
* [CLI-030] CLI tool for converting between schedule file formats (XLSX, native binary, widget JSON, HTML).
* [CLI-031] CLI tool for making batch edits to schedule data from the command line.
* [CLI-090] Add `Schedule::touch_modified()` and `EditContext::schedule_mut()` to schedule-core;
wire `touch_modified` into `apply()`, `undo()`, and `redo()`.
* [CLI-091] Establish the module layout, Cargo dependencies, arg-parsing skeleton, and file
load/save infrastructure for `cosam-modify`.
* [CLI-092] Implement the `list` and `get` subcommands to display entities and their field values.
* [CLI-093] Implement the `set` subcommand to update a named field on one or more entities.
* [CLI-094] Implement the `create` subcommand to add a new entity of any type with specified fields.
* [CLI-095] Implement the `delete` subcommand to soft-delete an entity by name or UUID.
* [CLI-096] Implement `add-edge` and `remove-edge` subcommands to manage entity relationships.
* [CLI-097] Implement in-memory `undo`, `redo`, and `show-history` subcommands.
* [CLI-098] Add `--help` output, proper exit codes, integration tests for all commands, and close out
CLI-031 and CLI-090–098.
* [EDITOR-033] Implement the main schedule grid view and entity editing UI in cosam-editor.
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
* [FEATURE-027] Implement export of schedule data to the JSON format consumed by the calendar display widget.
* [FEATURE-028] Import schedule data from the existing XLSX spreadsheet format.
* [FEATURE-029] Export schedule data back to the XLSX spreadsheet format.
* [FEATURE-038] Add a type-safe `FieldValueConverter<M>` trait and driver functions for converting
`FieldValue` inputs to typed Rust outputs via a work-queue iteration pattern.
* [FEATURE-043] Add a `verify` callback to `FieldDescriptor` for cross-field consistency checks after batch writes to computed fields.
* [FEATURE-046] Add `FieldSet::write_multiple()` for atomic batch field updates with verification support.
* [FEATURE-050] Add `FieldTypeItem` (scalar type tags) and `FieldType` (`Single`/`Optional`/`List`
wrappers) to `value.rs` as `Copy` type-level mirrors of `FieldValueItem`/`FieldValue`.
* [FEATURE-051] Add a `field_type: FieldType` field to `FieldDescriptor` and populate it in all
existing static field descriptors across every entity file.
* [FEATURE-056] Add computed/synthesized fields to public data structures to support widget JSON export.
* [FEATURE-057] Implement a transitive edge relationship cache to efficiently compute inclusive members, groups, panels, and other hierarchical relationships.
* [FEATURE-065] Convert `credited_presenters` and `uncredited_presenters` on Panel from computed/derived fields
into actual edge storage fields, eliminating the `credited` per-edge boolean and its CRDT
`presenters_meta` map.
* [FEATURE-068] Add `Copy` as a supertrait of `DynamicEntityId` so that by-value usage of id
parameters is ergonomic without ownership gymnastics.
* [FEATURE-069] Encode CRDT edge ownership direction directly in `CrdtFieldType` instead of
relying solely on `EdgeDescriptor` and `canonical_owner()`.
* [FEATURE-070] Remove the separate `EdgeDescriptor` struct and inventory; encode CRDT-edge ownership and target field directly inside `CrdtFieldType::EdgeOwner` on the owner field.
* [FEATURE-071] Replace the declarative `macro_rules!` field-declaration helpers (`stored_field!`,
`edge_field!`, `define_field!`) with attribute-style proc-macros in a new
`schedule-macro` crate; add an `exclusive_with:` clause to express
cross-partition edge exclusivity declaratively.
* [FEATURE-079] Add UUID conflict detection to entity creation and expand UuidPreference with "prefer" variants that allow fallback to alternate UUIDs.
* [FEATURE-081] Implement a UUID-indexed sidecar structure to track where each entity came from (file, sheet, row) separate from the CRDT schedule document.
* [FEATURE-082] Preserve unknown XLSX columns across import/export without encoding them as
first-class entity fields, and decide how this interacts with CRDT merge.
* [FEATURE-103] Compare and document the field definitions between the current main branch, v9, v10-try1, and v10-try3 to identify gaps and ensure complete coverage.
* [FEATURE-105] Improve the widget's browser print output so the grid view prints cleanly with proper column layout, hidden chrome, and expanded descriptions.
* [FEATURE-106] New shared Rust crate providing layout engine, brand config, Typst codegen, and in-process PDF compilation for print output formats.
* [FEATURE-107] New CLI binary that consumes `schedule.json` and `config/brand.toml` to produce Typst-compiled PDFs and/or `.typ` source files for all print layout formats.
* [FEATURE-108] Add an `--export-layout <DIR>` flag to `cosam-convert` that runs a default set of `cosam-layout` outputs after the schedule JSON export.
* [META-002] Phase tracker for project foundation and Cargo workspace setup.
* [META-003] Phase tracker for the entity/field system and core schedule data model in schedule-core.
* [META-004] Phase tracker for making an automerge CRDT document the authoritative storage
underneath `Schedule`.
* [META-005] Phase tracker for internal file format, multi-year archive, widget JSON, and
XLSX import/export.
* [META-006] Phase tracker for the cosam-convert and cosam-modify command-line applications.
* [META-048] Restructure `FieldValue` with proper cardinality, add `FieldTypeItem`/`FieldType`
enums, wire `FieldType` into `FieldDescriptor`, and implement the generic
`FieldValueConverter` system from IDEA-038.
* [META-102] Implement sidecar storage for provenance and extra metadata, and enable in-place XLSX updates.
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
* [REFACTOR-058] Update `FIELD_CREDITS` to use the per-edge `credited` flag introduced by
REFACTOR-060, so individual presenters can be excluded from credit display.
* [REFACTOR-059] Introduce `EdgeDescriptor` as a first-class type that co-locates edge definition,
CRDT ownership, and relationship semantics on the canonical owner entity type,
replacing the split `canonical_owner()` match table and `OWNER_EDGE_FIELDS` constant.
* [REFACTOR-060] Add per-edge data infrastructure to `EdgeDescriptor` and implement `credited: bool`
on the Panel ↔ Presenter relationship so individual presenters can be excluded
from credits without hiding all credits for the panel.
* [REFACTOR-061] Add type-erased field identity (`FieldId`) and field-based edge endpoint (`FieldNodeId`) types as
the foundation for the FieldNodeId-based edge system.
* [REFACTOR-062] Replace string-based `EdgeDescriptor` fields with `&'static dyn FieldDescriptorAny` references
and move EdgeDescriptor registration to `inventory`.
* [REFACTOR-063] Replace the two-map `RawEdgeMap` with a nested `HashMap<NonNilUuid, HashMap<FieldId, Vec<FieldNodeId>>>`,
eliminating the `homogeneous_reverse` special case.
* [REFACTOR-064] Adapt `schedule.rs`, `edge_crdt.rs`, and `edge_cache.rs` to use the new FieldNodeId-based
`RawEdgeMap`, replacing type-parameter-based edge lookups with field-based lookups.
* [REFACTOR-066] Eliminate per-entity-type `CollectedField<E>` registries, merge `FieldDescriptorAny` into `NamedField`,
and improve `FieldId` conversions with a global registry and type-safe downcasting.
* [REFACTOR-067] Add compile-time typed `FieldNodeId<E>` type similar to `EntityId<E>`, and rename existing `FieldNodeId` to `RuntimeFieldNodeId` for consistency with the entity ID pattern.
* [REFACTOR-074] Split edge fields out of `FieldDescriptor<E>` into a new `HalfEdgeDescriptor` struct; add
`EdgeKind` enum with ownership direction and exclusivity information.
* [REFACTOR-075] Update edit_integration.rs tests to work with new WriteFn::Schedule edge write mechanism used by HALF_EDGE_* fields
* [REFACTOR-104] Replace `PanelCommonData.cost: Option<String>` with a typed `AdditionalCost` enum
and a separate `for_kids: bool` flag, making invalid cost states unrepresentable.
* [UI-085] Audit and update the calendar widget to handle the format differences between
the v9 JSON output and the format produced by `cosam-convert` (CLI-030).
* [UI-087] The Event Type filter shows all non-hidden panel types even when none of that
type appear in the loaded schedule.
* [UI-088] Guest presenters should appear at the top of the presenter filter dropdown,
above panelists and groups.
* [UI-089] The cost filter has too many options; collapse "Additional Cost" and
"Workshops" into a single "Premium" option.

---

## Summary of Open Items

**Total open items:** 16

* **Meta / Project-Level**
  * [META-001] Meta work item tracking the full multi-phase redesign of the schedule system. (Blocked by [META-007], [META-008])
  * [META-007] Phase tracker for the cosam-editor desktop GUI application. (Blocked by [META-005])
  * [META-008] Phase tracker for peer-to-peer schedule synchronization and conflict resolution. (Blocked by [META-004])

* **High Priority**
  * [FEATURE-084] Implement `update_xlsx` to write schedule changes back into an existing XLSX
file, preserving formatting, formulas, extra columns, and non-standard content.

* **Medium Priority**
  * [FEATURE-026] Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.
  * [FEATURE-113] Replace the `std::process::Command::new("typst")` subprocess calls in
`schedule-layout` and `cosam-convert` with in-process compilation using the
`typst` Rust crate, eliminating the external `typst-cli` dependency.

* **Low Priority**
  * [CLI-100] Add a `--interactive` flag to `cosam-modify` that opens a read-eval-print loop for
entering commands one at a time.
  * [EDITOR-032] ([META-007]) Select the GUI framework for cosam-editor and create the application scaffold.
  * [EDITOR-111] Extract the duplicated `schedule_data.rs` UI helper present in both
`cosam-editor-gpui` and `cosam-editor-dioxus` into a new
`crates/cosam-editor-shared` crate once the GUI framework is chosen.
  * [FEATURE-034] ([META-008]) Define and implement the protocol for synchronizing schedule data between peers.
  * [FEATURE-035] ([META-008]) Provide UI for reviewing and resolving merge conflicts after sync.
  * [FEATURE-077] Implement add/remove operations for list cardinality fields in accessor_field_properties.
  * [FEATURE-083] Add a dedicated `Hotels` sheet to the XLSX format for richer hotel-room metadata.
  * [FEATURE-099] Serialize the `EditHistory` undo/redo stacks into the `.schedule` binary file so that
undo/redo works across `cosam-modify` invocations.
  * [FEATURE-110] Add Adobe InDesign Markup Language (IDML) as an optional export format for schedule layouts.
  * [REFACTOR-112] Update the `#[ignore]`d `set_neighbors` tests in `schedule-core/src/edge/map.rs`
to compile and pass against the current `RawEdgeMap` API.

---

## Placeholders

*No placeholders — all stubs have been promoted.*

Use `perl scripts/work-item-update.pl --create <PREFIX>` to add new stubs.

---

## Open CLI Items

### [CLI-100] CLI-100: cosam-modify interactive mode (--interactive REPL)

**Status:** Open

**Priority:** Low

**Summary:** Add a `--interactive` flag to `cosam-modify` that opens a read-eval-print loop for
entering commands one at a time.

**Blocked By:** [CLI-098]

**Description:** Interactive mode presents a prompt (`>`) and accepts the same commands as batch mode, one
per line:

```text
cosam-modify --file myfile.schedule --interactive
> list panels --select presenter matches Jane
> set panel GW0103 note "Will be outside if no rain"
> save
> open other.schedule
> quit
Save your changes? (Y/N)
```

---

## Open EDITOR Items

### [EDITOR-032] cosam-editor: GUI Framework Selection and Scaffold

**Status:** In progress

**Priority:** Low

**Summary:** Select the GUI framework for cosam-editor and create the application scaffold.

**Part of:** [META-007]

**Description:** Evaluate and select between GUI framework candidates, then create the initial
application structure.

---

### [EDITOR-111] EDITOR-111: Extract shared schedule_data module to crates/cosam-editor-shared

**Status:** Open

**Priority:** Low

**Summary:** Extract the duplicated `schedule_data.rs` UI helper present in both
`cosam-editor-gpui` and `cosam-editor-dioxus` into a new
`crates/cosam-editor-shared` crate once the GUI framework is chosen.

**Blocked By:** [EDITOR-032]

**Description:** Both `apps/cosam-editor-gpui/src/ui/schedule_data.rs` and
`apps/cosam-editor-dioxus/src/ui/schedule_data.rs` contain identical
or near-identical logic for adapting `schedule-core` data for display.
Once the framework decision is made the surviving copy should move to
`crates/cosam-editor-shared` so it can be reused by any future editor
target without duplication.

---

## Open FEATURE Items

### [FEATURE-084] FEATURE-084: XLSX Spreadsheet Update (In-Place Save)

**Status:** Open

**Priority:** High

**Summary:** Implement `update_xlsx` to write schedule changes back into an existing XLSX
file, preserving formatting, formulas, extra columns, and non-standard content.

**Blocked By:** [FEATURE-029]

**Description:** `export_xlsx` (FEATURE-029) always writes a fresh workbook from scratch.
`update_xlsx` would instead open the original file and patch only the rows that
changed, preserving:

* Cell formatting (colors, fonts, borders)
* Formula cells the user has added (e.g., conditional-format helpers)
* Extra non-standard columns (custom per-convention data)
* Timestamp and Grid sheets
* Non-imported sheets that we never touch

This is the workflow convention staff actually uses: import once to seed the
schedule database, then save back repeatedly as edits accumulate.

---

### [FEATURE-026] Multi-Year Schedule Archive Support

**Status:** Open

**Priority:** Medium

**Summary:** Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.

**Blocked By:** [FEATURE-025]

**Description:** A schedule archive contains multiple years of convention data in one file,
enabling:

* **Jump-start**: Copy entities from a prior year to pre-populate the next
  convention (recurring panels, returning presenters, same rooms)
* **Historical reference**: View past schedules alongside the current one

---

### [FEATURE-113] FEATURE-113: In-process Typst PDF compilation (replace typst CLI subprocess)

**Status:** Open

**Priority:** Medium

**Summary:** Replace the `std::process::Command::new("typst")` subprocess calls in
`schedule-layout` and `cosam-convert` with in-process compilation using the
`typst` Rust crate, eliminating the external `typst-cli` dependency.

**Description:** Both `apps/cosam-convert/src/main.rs` (`run_layout_export`) and
`apps/cosam-layout/src/main.rs` (`compile_typst`) currently shell out to the
`typst compile` CLI binary to produce PDFs. This requires `typst-cli` to be
installed separately and on `PATH`, which is inconvenient and fragile.

The `typst` Rust crate provides a `compile()` API that can do this in-process,
but it requires implementing the `World` trait (file I/O, font loading, date,
package resolution). The `typst-kit` crate (maintained by the Typst team)
provides ready-made font search and embed helpers to simplify `World`
implementation.

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

### [FEATURE-077] FEATURE-077: List cardinality support for accessor_field_properties

**Status:** Open

**Priority:** Low

**Summary:** Implement add/remove operations for list cardinality fields in accessor_field_properties.

**Description:** The accessor_field_properties macro currently sets add_fn and remove_fn to None for all fields, with a TODO comment to revisit if list cardinality support is implemented. This feature implements add/remove operations for accessor fields (computed fields that read/write to underlying storage) with list cardinality.

Currently, add/remove operations are only supported for edge fields through the AddEdge/RemoveEdge variants. Supporting add/remove for accessor list fields would require:

1. **Determine use cases**: Identify which accessor fields with list cardinality should support add/remove operations (e.g., adding to a list field vs. replacing the entire list)

2. **Add new AddFn/RemoveFn variants**: Create new callback variants for accessor field add/remove operations, possibly:
   * AddFn::BareList - for bare function add operations on lists
   * AddFn::ScheduleList - for schedule-aware add operations on lists
   * RemoveFn::BareList - for bare function remove operations on lists
   * RemoveFn::ScheduleList - for schedule-aware remove operations on lists

3. **Implement AddableField/RemovableField for FieldDescriptor**: The FieldDescriptor already implements these traits, but they would need to handle the new list-specific variants

4. **Update conversion support**: The conversion layer (field_value_to_runtime_entity_ids and similar functions) may need updates to handle list add/remove operations for non-edge types. Currently these conversions are primarily designed for entity IDs in edge contexts.

5. **Update accessor_field_properties macro**: Add logic to generate appropriate add_fn/remove_fn based on:
   * Field cardinality (Single vs. List)
   * Whether add/remove operations are desired for the field
   * The type of callback needed (bare vs. schedule)

6. **Update stored_output.rs**: Modify the macro to conditionally generate add_fn/remove_fn instead of always setting them to None

7. **Testing**: Add comprehensive tests for add/remove operations on accessor list fields

---

### [FEATURE-083] FEATURE-083: Separate Hotel Room sheet in XLSX import/export

**Status:** Open

**Priority:** Low

**Summary:** Add a dedicated `Hotels` sheet to the XLSX format for richer hotel-room metadata.

**Description:** Currently hotel rooms are expressed as a single column (`Hotel Room`) in the Rooms sheet,
limited to one hotel room name per event room. A dedicated `Hotels` sheet would allow richer
metadata (sort key, long name, notes) and cleaner round-trips, mirroring how rooms and panel
types already get their own sheets.

Proposed sheet name: `Hotels` (with `Hotel Rooms` as a fallback alias).

Proposed columns:

* Hotel Room — canonical name (key for linking from the Rooms sheet)
* Sort Key — optional integer for ordering
* Long Name — optional display name

Implementation notes:

* Import: teach `read/rooms.rs` to look for a Hotels sheet and create `HotelRoomEntityType`
  entities from it; the `Hotel Room` column in the Rooms sheet would still be accepted as a
  fallback for files without the separate sheet.
* Export: add `write_hotel_rooms_sheet()` in `xlsx/write/export.rs` alongside the existing
  `write_rooms_sheet()`; suppress the `Hotel Room` column from the Rooms sheet when the
  separate sheet is written.
* The `EDGE_HOTEL_ROOMS` relationship in `event_room.rs` and `columns::room_map::HOTEL_ROOM`
  are the key integration points.

---

### [FEATURE-099] FEATURE-099: Undo/redo history persistence in binary file

**Status:** Open

**Priority:** Low

**Summary:** Serialize the `EditHistory` undo/redo stacks into the `.schedule` binary file so that
undo/redo works across `cosam-modify` invocations.

**Blocked By:** [CLI-098], [IDEA-101]

**Description:** Currently `EditHistory` is in-memory only. A fresh invocation of `cosam-modify` always
starts with empty undo/redo stacks even if the previous invocation made changes.

Implementing cross-invocation undo requires:

1. A serialization format for `EditCommand` (and thus `FieldValue`, `RuntimeEntityId`, etc.)
2. A binary file format change — either bumping `FILE_FORMAT_VERSION` and adding an undo
   section to the envelope, or storing the history inside the automerge document.
3. Care that CRDT `apply_changes` / `merge` paths do not restore stale undo state from a
   diverged replica.
4. A maximum history depth limit for the on-disk representation.

---

### [FEATURE-110] FEATURE-110: Add IDML export format option

**Status:** Open

**Priority:** Low

**Summary:** Add Adobe InDesign Markup Language (IDML) as an optional export format for schedule layouts.

**Blocked By:** [FEATURE-106]

**Description:** Add IDML export as an optional output format in the schedule-layout crate. IDML is Adobe's XML-based format for InDesign documents, packaged as a ZIP archive containing XML files and assets. This would provide an alternative to the Typst/PDF workflow for users who need editable InDesign files or require InDesign-specific features.

IDML is significantly more complex than the current Typst approach, requiring XML generation for multiple components (Stories, Spreads, MasterSpreads, Styles, Resources) and ZIP packaging. This feature should be implemented as an optional format behind a feature flag.

---

## Open META Items

### [META-001] Architecture Redesign: CRDT-backed Schedule System

**Status:** Open

**Priority:** High

**Summary:** Meta work item tracking the full multi-phase redesign of the schedule system.

**Blocked By:** [META-007], [META-008]

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

### [REFACTOR-112] REFACTOR-112: Update ignored set_neighbors tests to current RawEdgeMap API

**Status:** Open

**Priority:** Low

**Summary:** Update the `#[ignore]`d `set_neighbors` tests in `schedule-core/src/edge/map.rs`
to compile and pass against the current `RawEdgeMap` API.

**Description:** The test `test_set_neighbors_replaces_and_patches_reverse` (and any related
`set_neighbors` tests) in `crates/schedule-core/src/edge/map.rs` are marked
`#[ignore]` with a TODO comment because they were written against an older API
and no longer compile or reflect the current `RawEdgeMap` structure (which uses
a `HashMap<NonNilUuid, HashMap<FieldId, Vec<FieldNodeId>>>` layout).

---

---

[BUGFIX-072]: work-item/done/BUGFIX-072.md
[BUGFIX-073]: work-item/done/BUGFIX-073.md
[BUGFIX-076]: work-item/done/BUGFIX-076.md
[BUGFIX-078]: work-item/done/BUGFIX-078.md
[BUGFIX-086]: work-item/done/BUGFIX-086.md
[CLI-030]: work-item/done/CLI-030.md
[CLI-031]: work-item/done/CLI-031.md
[CLI-090]: work-item/done/CLI-090.md
[CLI-091]: work-item/done/CLI-091.md
[CLI-092]: work-item/done/CLI-092.md
[CLI-093]: work-item/done/CLI-093.md
[CLI-094]: work-item/done/CLI-094.md
[CLI-095]: work-item/done/CLI-095.md
[CLI-096]: work-item/done/CLI-096.md
[CLI-097]: work-item/done/CLI-097.md
[CLI-098]: work-item/done/CLI-098.md
[CLI-100]: work-item/low/CLI-100.md
[EDITOR-032]: work-item/low/EDITOR-032.md
[EDITOR-033]: work-item/done/EDITOR-033.md
[EDITOR-111]: work-item/low/EDITOR-111.md
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
[FEATURE-027]: work-item/done/FEATURE-027.md
[FEATURE-028]: work-item/done/FEATURE-028.md
[FEATURE-029]: work-item/done/FEATURE-029.md
[FEATURE-034]: work-item/low/FEATURE-034.md
[FEATURE-035]: work-item/low/FEATURE-035.md
[FEATURE-038]: work-item/done/FEATURE-038.md
[FEATURE-043]: work-item/done/FEATURE-043.md
[FEATURE-046]: work-item/done/FEATURE-046.md
[FEATURE-050]: work-item/done/FEATURE-050.md
[FEATURE-051]: work-item/done/FEATURE-051.md
[FEATURE-056]: work-item/done/FEATURE-056.md
[FEATURE-057]: work-item/done/FEATURE-057.md
[FEATURE-065]: work-item/done/FEATURE-065.md
[FEATURE-068]: work-item/done/FEATURE-068.md
[FEATURE-069]: work-item/done/FEATURE-069.md
[FEATURE-070]: work-item/done/FEATURE-070.md
[FEATURE-071]: work-item/done/FEATURE-071.md
[FEATURE-077]: work-item/low/FEATURE-077.md
[FEATURE-079]: work-item/done/FEATURE-079.md
[FEATURE-081]: work-item/done/FEATURE-081.md
[FEATURE-082]: work-item/done/FEATURE-082.md
[FEATURE-083]: work-item/low/FEATURE-083.md
[FEATURE-084]: work-item/high/FEATURE-084.md
[FEATURE-099]: work-item/low/FEATURE-099.md
[FEATURE-103]: work-item/done/FEATURE-103.md
[FEATURE-105]: work-item/done/FEATURE-105.md
[FEATURE-106]: work-item/done/FEATURE-106.md
[FEATURE-107]: work-item/done/FEATURE-107.md
[FEATURE-108]: work-item/done/FEATURE-108.md
[FEATURE-110]: work-item/low/FEATURE-110.md
[FEATURE-113]: work-item/medium/FEATURE-113.md
[META-001]: work-item/meta/META-001.md
[META-002]: work-item/done/META-002.md
[META-003]: work-item/done/META-003.md
[META-004]: work-item/done/META-004.md
[META-005]: work-item/done/META-005.md
[META-006]: work-item/done/META-006.md
[META-007]: work-item/meta/META-007.md
[META-008]: work-item/meta/META-008.md
[META-048]: work-item/done/META-048.md
[META-102]: work-item/done/META-102.md
[REFACTOR-041]: work-item/done/REFACTOR-041.md
[REFACTOR-047]: work-item/done/REFACTOR-047.md
[REFACTOR-049]: work-item/done/REFACTOR-049.md
[REFACTOR-052]: work-item/done/REFACTOR-052.md
[REFACTOR-053]: work-item/done/REFACTOR-053.md
[REFACTOR-054]: work-item/done/REFACTOR-054.md
[REFACTOR-055]: work-item/done/REFACTOR-055.md
[REFACTOR-058]: work-item/done/REFACTOR-058.md
[REFACTOR-059]: work-item/done/REFACTOR-059.md
[REFACTOR-060]: work-item/done/REFACTOR-060.md
[REFACTOR-061]: work-item/done/REFACTOR-061.md
[REFACTOR-062]: work-item/done/REFACTOR-062.md
[REFACTOR-063]: work-item/done/REFACTOR-063.md
[REFACTOR-064]: work-item/done/REFACTOR-064.md
[REFACTOR-066]: work-item/done/REFACTOR-066.md
[REFACTOR-067]: work-item/done/REFACTOR-067.md
[REFACTOR-074]: work-item/done/REFACTOR-074.md
[REFACTOR-075]: work-item/done/REFACTOR-075.md
[REFACTOR-104]: work-item/done/REFACTOR-104.md
[REFACTOR-112]: work-item/low/REFACTOR-112.md
[UI-085]: work-item/done/UI-085.md
[UI-087]: work-item/done/UI-087.md
[UI-088]: work-item/done/UI-088.md
[UI-089]: work-item/done/UI-089.md
