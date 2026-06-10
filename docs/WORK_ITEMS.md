# Cosplay America Schedule - Work Item

Updated on: Tue Jun  9 21:14:06 2026

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
* [BUGFIX-123] When re-importing an XLSX over an existing schedule, a presenter whose name
differs only in case (e.g. `"camelcase"` → `"CamelCase"`) is matched correctly
but the stored name is never updated to match the xlsx spelling.
* [BUGFIX-124] When re-importing an XLSX over an existing schedule, a presenter whose rank in
the new xlsx is lower than the historically stored rank is not downgraded.
The xlsx should be the source of truth on update.
* [BUGFIX-131] `PanelUniqId::parse("SPLIT001")` normalizes the prefix to `"SP"` and returns
`full_id()` = `"SP001"`, discarding the original `"SPLIT001"` string. The raw
form typed in the spreadsheet should be preserved.
* [BUGFIX-145] Two import paths silently drop rows that should be kept. (1) A non-blank Uniq ID
that doesn't match the strict grammar (typos, hyphens, numberless codes) makes
`FIELD_CODE`'s write error, so the whole upsert fails and the row vanishes.
(2) A leading `*` on the Uniq ID is treated as a soft-delete and skipped. Per
design intent there are no required fields, so such rows must import — the
`*` form as an *unscheduled* panel.
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
* [CLI-139] Replace the standalone `cosam-layout` binary with `--layout.<key>=<value>`
flags on `cosam-convert`.
* [EDITOR-032] Select the GUI framework for cosam-editor and create the application scaffold.
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
* [FEATURE-083] Add a dedicated `Hotels` sheet to the XLSX format for richer hotel-room metadata.
* [FEATURE-103] Compare and document the field definitions between the current main branch, v9, v10-try1, and v10-try3 to identify gaps and ensure complete coverage.
* [FEATURE-105] Improve the widget's browser print output so the grid view prints cleanly with proper column layout, hidden chrome, and expanded descriptions.
* [FEATURE-106] New shared Rust crate providing layout engine, brand config, Typst codegen, and in-process PDF compilation for print output formats.
* [FEATURE-107] New CLI binary that consumes `schedule.json` and `config/brand.toml` to produce Typst-compiled PDFs and/or `.typ` source files for all print layout formats.
* [FEATURE-108] Add an `--export-layout <DIR>` flag to `cosam-convert` that runs a default set of `cosam-layout` outputs after the schedule JSON export.
* [FEATURE-110] Add Adobe InDesign Markup Language (IDML) as an optional export format for schedule layouts.
* [FEATURE-114] Add one grid-view reference sheet per day to the exported XLSX, mirroring the HTML schedule grid with merged cells for multi-slot and multi-room events.
* [FEATURE-115] Separate Timeline Sheet in XLSX
* [FEATURE-116] New Dioxus 0.7 viewer app that reads widget JSON and renders a UI similar to the JS widget.
* [FEATURE-118] Add a CSS-grid schedule view to cosam-viewer mirroring the JS widget's grid mode.
* [FEATURE-121] Expand cosam-viewer to open XLSX, binary `.cosam`, and CSV directory schedules, plus
fetch widget JSON from a webpage URL.
* [FEATURE-122] Replace all "import → new schedule" functions with "update existing schedule"
variants; the old functions become thin wrappers that create a blank schedule
and delegate.
* [FEATURE-127] Re-importing the same XLSX or widget JSON into an existing binary schedule
should produce a byte-for-byte identical output when nothing in the source
has changed.
* [FEATURE-129] Replace EditCommand-returns-inverse undo/redo with CRDT heads checkpoints so that bulk operations (XLSX import) become a single undoable step with a user-visible label.
* [FEATURE-132] Add a hybrid "widget-html" format where structural schedule data (meta, rooms, panelTypes, timeline, presenters) is kept as a compact JSON block but panels are rendered as semantic HTML, enabling SEO crawlability and a no-JS fallback while preserving full widget functionality.
* [FEATURE-134] Add a double-sided per-day "flyer" print layout that places the day's schedule grid on the left half of each day's first page with panel descriptions flowing through the remaining columns and onto following full-width pages, one multi-day document with a page-number/timestamp footer.
* [FEATURE-135] Add a `--export-xlsx-grid` option to `cosam-convert` that writes only the
per-day grid reference sheets, and wire it into `sync-schedule.sh`.
* [FEATURE-136] Combine all room signs into one multi-page document and adopt the flyer's
`place`-plus-column-break grid mixing instead of the rigid side-by-side grid.
* [FEATURE-137] Add per-job layout options for a bordered "card" panel style, page background
tint, empty grid-cell fill, card fill, and column/panel gaps — all controllable
from `config/layout.toml`.
* [FEATURE-144] Model convention-wide breaks as a first-class `Break` entity (like `Timeline`),
carrying duration, instead of `Panel` entities flagged `is_break`.
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
* [META-128] Review and redesign the undo/redo system to ensure it integrates with all mutation paths and supports the intended checkpoint-based optimization for bulk operations.
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
* [REFACTOR-125] Move schedule, options, per-pass lookups, and PresenterImportCache into
ImportContext; convert reader free functions to methods on ImportContext.
* [REFACTOR-138] Separate the layout `split` key into independent `section_split` and
`time_split` options, default time split to none, error on unknown keywords,
and move panel-list geometry constants into `geometry.rs`.
* [REFACTOR-140] Introduce a `RankSource` tier model for presenter rank, route all presenter
creation through the single tagged API with deterministic v5 UUIDs, and clean up
`presenter.rs` visibility and People-sheet membership helpers.
* [REFACTOR-141] Remove the presenter `sort_index` field and the XLSX `xlsx_sort_key` sidecar
infrastructure that fed it, so presenters order deterministically by rank then
name with nothing carried over from spreadsheet column/row position.
* [UI-085] Audit and update the calendar widget to handle the format differences between
the v9 JSON output and the format produced by `cosam-convert` (CLI-030).
* [UI-087] The Event Type filter shows all non-hidden panel types even when none of that
type appear in the loaded schedule.
* [UI-088] Guest presenters should appear at the top of the presenter filter dropdown,
above panelists and groups.
* [UI-089] The cost filter has too many options; collapse "Additional Cost" and
"Workshops" into a single "Premium" option.
* [UI-143] Make the day stay visible in list and grid views while scrolling, and show the date in the event detail modal.

---

## Superseded / Rejected

* [CLI-133] (Rejected) Redesign `cosam-layout` to accept `--output <file>` (explicit file path) rather
than `--output-dir <dir>`, mirroring the old `dump_flyers` / `schedule_html`
pattern where each job spec points to a specific output file or directory.

---

## Summary of Open Items

**Total open items:** 18

* **Meta / Project-Level**
  * [META-001] Meta work item tracking the full multi-phase redesign of the schedule system. (Blocked by [META-007], [META-008])
  * [META-007] Phase tracker for the cosam-editor desktop GUI application. (Blocked by [META-005])
  * [META-008] Phase tracker for peer-to-peer schedule synchronization and conflict resolution. (Blocked by [META-004])
  * [META-117] Tracker for all cosam-viewer work: initial viewer app and deferred enhancements.

* **High Priority**
  * [FEATURE-113] Replace the `std::process::Command::new("typst")` subprocess call in
`cosam-convert` with in-process compilation using the `typst` Rust crate,
eliminating the external `typst-cli` dependency.
  * [FEATURE-126] Add update-mode (upsert + soft-delete) semantics to widget JSON import,
analogous to what FEATURE-122 did for XLSX, with extra care to preserve
schedule data that the lossy widget JSON format does not carry.

* **Low Priority**
  * [CLI-100] Add a `--interactive` flag to `cosam-modify` that opens a read-eval-print loop for
entering commands one at a time.
  * [EDITOR-033] ([META-007]) Implement the main schedule grid view and entity editing UI in cosam-editor.
  * [EDITOR-111] Extract the duplicated `schedule_data.rs` UI helper present in both
`cosam-editor-gpui` and `cosam-editor-dioxus` into a new
`crates/cosam-editor-shared` crate once the GUI framework is chosen.
  * [FEATURE-026] Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.
  * [FEATURE-034] ([META-008]) Define and implement the protocol for synchronizing schedule data between peers.
  * [FEATURE-035] ([META-008]) Provide UI for reviewing and resolving merge conflicts after sync.
  * [FEATURE-084] Implement `update_xlsx` to write schedule changes back into an existing XLSX
file, preserving formatting, formulas, extra columns, and non-standard content.
  * [FEATURE-099] Serialize the `EditHistory` undo/redo stacks into the `.schedule` binary file so that
undo/redo works across `cosam-modify` invocations.
  * [FEATURE-119] ([META-117]) Allow attendees to star/bookmark panels and view a personal schedule, mirroring
the JS widget's named-schedule feature.
  * [FEATURE-120] ([META-117]) Configure `dx` build targets for Android and iPadOS, including app metadata,
icons, and CI/CD pipeline integration.
  * [FEATURE-142] Bring the IDML export toward Typst/PDF parity: schedule grid as an InDesign
table, page-header banners, multi-column body text, and page footers.
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

### [EDITOR-033] cosam-editor: Schedule Grid View and Entity Editing

**Status:** Open

**Priority:** Low

**Summary:** Implement the main schedule grid view and entity editing UI in cosam-editor.

**Part of:** [META-007]

**Description:** The core editing experience for cosam-editor (Dioxus 0.7). The initial
scaffold delivered a filter-list-detail layout with inline name editing.
This item tracks the remaining editing features needed for a usable editor.

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

### [FEATURE-113] FEATURE-113: In-process Typst PDF compilation (replace typst CLI subprocess)

**Status:** Open

**Priority:** High

**Summary:** Replace the `std::process::Command::new("typst")` subprocess call in
`cosam-convert` with in-process compilation using the `typst` Rust crate,
eliminating the external `typst-cli` dependency.

**Description:** `apps/cosam-convert/src/main.rs` (`run_layout_export`) currently shells out to
the `typst compile` CLI binary to produce PDFs. This requires `typst-cli` to be
installed separately and on `PATH`, which is inconvenient and fragile.
(`cosam-layout` was removed in CLI-139; layout rendering now lives entirely in
`cosam-convert`.)

The `typst` Rust crate provides a `compile()` API that can do this in-process,
but it requires implementing the `World` trait (file I/O, font loading, date,
package resolution). The `typst-kit` crate (maintained by the Typst team)
provides ready-made font search and embed helpers to simplify `World`
implementation.

---

### [FEATURE-126] FEATURE-126: Widget JSON update-mode import with data preservation

**Status:** Open

**Priority:** High

**Summary:** Add update-mode (upsert + soft-delete) semantics to widget JSON import,
analogous to what FEATURE-122 did for XLSX, with extra care to preserve
schedule data that the lossy widget JSON format does not carry.

**Description:** `import_from_widget_json` currently creates a fresh `Schedule` from
widget JSON. It cannot be used to update an existing schedule because:

* it always allocates new UUIDs (losing CRDT history)
* it does not match or merge entities by natural key
* it silently drops fields that widget JSON does not carry

This feature brings widget JSON import up to the same standard as the
XLSX update-mode added in FEATURE-122:

---

### [FEATURE-026] Multi-Year Schedule Archive Support

**Status:** Open

**Priority:** Low

**Summary:** Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.

**Blocked By:** [FEATURE-025]

**Description:** A schedule archive contains multiple years of convention data in one file,
enabling:

* **Jump-start**: Copy entities from a prior year to pre-populate the next
  convention (recurring panels, returning presenters, same rooms)
* **Historical reference**: View past schedules alongside the current one

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

### [FEATURE-084] FEATURE-084: XLSX Spreadsheet Update (In-Place Save)

**Status:** Open

**Priority:** Low

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

### [FEATURE-099] FEATURE-099: Undo/redo history persistence in binary file

**Status:** Open

**Priority:** Low

**Summary:** Serialize the `EditHistory` undo/redo stacks into the `.schedule` binary file so that
undo/redo works across `cosam-modify` invocations.

**Blocked By:** [CLI-098], [IDEA-101], [IDEA-130]

**Description:** Currently `EditHistory` is in-memory only. A fresh invocation of `cosam-modify` always
starts with empty undo/redo stacks even if the previous invocation made changes.

With the heads-based undo/redo system (FEATURE-129), each `UndoEntry` stores only:

* `label: Cow<'static, str>`
* `pre_heads: Vec<ChangeHash>` (array of 32-byte hashes)
* `changes: Vec<Vec<u8>>` (raw automerge change bytes already in the document)

This is significantly simpler to serialize than the old `EditCommand` approach.
Implementing cross-invocation undo requires:

1. A serialization format for `UndoEntry` — CBOR or JSON for the label and head hashes;
   the change bytes are already raw bytes.
2. A binary file format change — bump `FILE_FORMAT_VERSION` and add a history section to
   the envelope after the automerge document bytes.
3. On load, validate that all `pre_heads` and change hashes still exist in the loaded
   document; discard entries whose heads are no longer reachable (handles diverged replicas).
4. A maximum history depth limit for the on-disk representation (default 100 already in
   `EditHistory::DEFAULT_MAX_DEPTH`).

---

### [FEATURE-119] FEATURE-119: cosam-viewer — My Schedule bookmarking

**Status:** Open

**Priority:** Low

**Summary:** Allow attendees to star/bookmark panels and view a personal schedule, mirroring
the JS widget's named-schedule feature.

**Part of:** [META-117]

**Description:** Add panel bookmarking to cosam-viewer so users can build a personal schedule.
On desktop, persist to a local file or app-data directory. On mobile, use
platform storage. Optionally support URL-hash sharing (as in the JS widget).

---

### [FEATURE-120] FEATURE-120: cosam-viewer — mobile build and deploy configuration

**Status:** Open

**Priority:** Low

**Summary:** Configure `dx` build targets for Android and iPadOS, including app metadata,
icons, and CI/CD pipeline integration.

**Part of:** [META-117]

**Description:** Set up the `Dioxus.toml`, Android manifest, iOS Info.plist, and icon assets
needed to produce release builds of cosam-viewer for Android and iPadOS via
`dx build --platform android` and `dx build --platform ios`.

---

### [FEATURE-142] FEATURE-142: Expand IDML export (grid, banners, columns, footers)

**Status:** Open

**Priority:** Low

**Summary:** Bring the IDML export toward Typst/PDF parity: schedule grid as an InDesign
table, page-header banners, multi-column body text, and page footers.

**Blocked By:** [FEATURE-110]

**Description:** FEATURE-110 shipped a v1 IDML export: a threaded text listing of panels grouped
by day and time slot, with brand-driven paragraph styles. It deliberately
deferred the richer layout features that the Typst pipeline already produces.
This item closes that gap so an `.idml` job approaches the fidelity of its
`.pdf` counterpart for the same `LayoutConfig`.

The work builds on the existing `schedule-layout/src/idml.rs` module and reuses
the layout computations already feeding the Typst path
(`timegrid::GridLayout::compute`, `document::build_sections`,
`blocks::banner`/`blocks::grid`), emitting IDML XML instead of Typst source.

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

### [META-117] META-117: cosam-viewer — cross-platform schedule viewer

**Status:** Blocked

**Priority:** Medium

**Summary:** Tracker for all cosam-viewer work: initial viewer app and deferred enhancements.

**Description:** cosam-viewer is a Dioxus 0.7 app that reads the cosam widget JSON format and
renders a schedule UI similar to the JS widget, targeting macOS, iPadOS, and Android.

**Work Items:**

* FEATURE-116: Initial cosam-viewer app (list view, filters, day tabs, detail modal, 4 themes)
* FEATURE-118: Grid view (rooms × time slots)
* FEATURE-119: My Schedule / bookmarking
* FEATURE-120: Mobile-specific build and deploy configuration

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

[BUGFIX-072]: ../work-item/closed/done/BUGFIX-072.md
[BUGFIX-073]: ../work-item/closed/done/BUGFIX-073.md
[BUGFIX-076]: ../work-item/closed/done/BUGFIX-076.md
[BUGFIX-078]: ../work-item/closed/done/BUGFIX-078.md
[BUGFIX-086]: ../work-item/closed/done/BUGFIX-086.md
[BUGFIX-123]: ../work-item/closed/done/BUGFIX-123.md
[BUGFIX-124]: ../work-item/closed/done/BUGFIX-124.md
[BUGFIX-131]: ../work-item/closed/done/BUGFIX-131.md
[BUGFIX-145]: ../work-item/closed/done/BUGFIX-145.md
[CLI-030]: ../work-item/closed/done/CLI-030.md
[CLI-031]: ../work-item/closed/done/CLI-031.md
[CLI-090]: ../work-item/closed/done/CLI-090.md
[CLI-091]: ../work-item/closed/done/CLI-091.md
[CLI-092]: ../work-item/closed/done/CLI-092.md
[CLI-093]: ../work-item/closed/done/CLI-093.md
[CLI-094]: ../work-item/closed/done/CLI-094.md
[CLI-095]: ../work-item/closed/done/CLI-095.md
[CLI-096]: ../work-item/closed/done/CLI-096.md
[CLI-097]: ../work-item/closed/done/CLI-097.md
[CLI-098]: ../work-item/closed/done/CLI-098.md
[CLI-100]: ../work-item/open/3-LOW/CLI-100.md
[CLI-133]: ../work-item/closed/rejected/CLI-133.md
[CLI-139]: ../work-item/closed/done/CLI-139.md
[EDITOR-032]: ../work-item/closed/done/EDITOR-032.md
[EDITOR-033]: ../work-item/open/3-LOW/EDITOR-033.md
[EDITOR-111]: ../work-item/open/3-LOW/EDITOR-111.md
[FEATURE-009]: ../work-item/closed/done/FEATURE-009.md
[FEATURE-010]: ../work-item/closed/done/FEATURE-010.md
[FEATURE-011]: ../work-item/closed/done/FEATURE-011.md
[FEATURE-012]: ../work-item/closed/done/FEATURE-012.md
[FEATURE-013]: ../work-item/closed/done/FEATURE-013.md
[FEATURE-014]: ../work-item/closed/done/FEATURE-014.md
[FEATURE-015]: ../work-item/closed/done/FEATURE-015.md
[FEATURE-016]: ../work-item/closed/done/FEATURE-016.md
[FEATURE-017]: ../work-item/closed/done/FEATURE-017.md
[FEATURE-018]: ../work-item/closed/done/FEATURE-018.md
[FEATURE-019]: ../work-item/closed/done/FEATURE-019.md
[FEATURE-020]: ../work-item/closed/done/FEATURE-020.md
[FEATURE-021]: ../work-item/closed/done/FEATURE-021.md
[FEATURE-022]: ../work-item/closed/done/FEATURE-022.md
[FEATURE-023]: ../work-item/closed/done/FEATURE-023.md
[FEATURE-024]: ../work-item/closed/done/FEATURE-024.md
[FEATURE-025]: ../work-item/closed/done/FEATURE-025.md
[FEATURE-026]: ../work-item/open/3-LOW/FEATURE-026.md
[FEATURE-027]: ../work-item/closed/done/FEATURE-027.md
[FEATURE-028]: ../work-item/closed/done/FEATURE-028.md
[FEATURE-029]: ../work-item/closed/done/FEATURE-029.md
[FEATURE-034]: ../work-item/open/3-LOW/FEATURE-034.md
[FEATURE-035]: ../work-item/open/3-LOW/FEATURE-035.md
[FEATURE-038]: ../work-item/closed/done/FEATURE-038.md
[FEATURE-043]: ../work-item/closed/done/FEATURE-043.md
[FEATURE-046]: ../work-item/closed/done/FEATURE-046.md
[FEATURE-050]: ../work-item/closed/done/FEATURE-050.md
[FEATURE-051]: ../work-item/closed/done/FEATURE-051.md
[FEATURE-056]: ../work-item/closed/done/FEATURE-056.md
[FEATURE-057]: ../work-item/closed/done/FEATURE-057.md
[FEATURE-065]: ../work-item/closed/done/FEATURE-065.md
[FEATURE-068]: ../work-item/closed/done/FEATURE-068.md
[FEATURE-069]: ../work-item/closed/done/FEATURE-069.md
[FEATURE-070]: ../work-item/closed/done/FEATURE-070.md
[FEATURE-071]: ../work-item/closed/done/FEATURE-071.md
[FEATURE-079]: ../work-item/closed/done/FEATURE-079.md
[FEATURE-081]: ../work-item/closed/done/FEATURE-081.md
[FEATURE-082]: ../work-item/closed/done/FEATURE-082.md
[FEATURE-083]: ../work-item/closed/done/FEATURE-083.md
[FEATURE-084]: ../work-item/open/3-LOW/FEATURE-084.md
[FEATURE-099]: ../work-item/open/3-LOW/FEATURE-099.md
[FEATURE-103]: ../work-item/closed/done/FEATURE-103.md
[FEATURE-105]: ../work-item/closed/done/FEATURE-105.md
[FEATURE-106]: ../work-item/closed/done/FEATURE-106.md
[FEATURE-107]: ../work-item/closed/done/FEATURE-107.md
[FEATURE-108]: ../work-item/closed/done/FEATURE-108.md
[FEATURE-110]: ../work-item/closed/done/FEATURE-110.md
[FEATURE-113]: ../work-item/open/1-HIGH/FEATURE-113.md
[FEATURE-114]: ../work-item/closed/done/FEATURE-114.md
[FEATURE-115]: ../work-item/closed/done/FEATURE-115.md
[FEATURE-116]: ../work-item/closed/done/FEATURE-116.md
[FEATURE-118]: ../work-item/closed/done/FEATURE-118.md
[FEATURE-119]: ../work-item/open/3-LOW/FEATURE-119.md
[FEATURE-120]: ../work-item/open/3-LOW/FEATURE-120.md
[FEATURE-121]: ../work-item/closed/done/FEATURE-121.md
[FEATURE-122]: ../work-item/closed/done/FEATURE-122.md
[FEATURE-126]: ../work-item/open/1-HIGH/FEATURE-126.md
[FEATURE-127]: ../work-item/closed/done/FEATURE-127.md
[FEATURE-129]: ../work-item/closed/done/FEATURE-129.md
[FEATURE-132]: ../work-item/closed/done/FEATURE-132.md
[FEATURE-134]: ../work-item/closed/done/FEATURE-134.md
[FEATURE-135]: ../work-item/closed/done/FEATURE-135.md
[FEATURE-136]: ../work-item/closed/done/FEATURE-136.md
[FEATURE-137]: ../work-item/closed/done/FEATURE-137.md
[FEATURE-142]: ../work-item/open/3-LOW/FEATURE-142.md
[FEATURE-144]: ../work-item/closed/done/FEATURE-144.md
[META-001]: ../work-item/meta/META-001.md
[META-002]: ../work-item/closed/done/META-002.md
[META-003]: ../work-item/closed/done/META-003.md
[META-004]: ../work-item/closed/done/META-004.md
[META-005]: ../work-item/closed/done/META-005.md
[META-006]: ../work-item/closed/done/META-006.md
[META-007]: ../work-item/meta/META-007.md
[META-008]: ../work-item/meta/META-008.md
[META-048]: ../work-item/closed/done/META-048.md
[META-102]: ../work-item/closed/done/META-102.md
[META-117]: ../work-item/meta/META-117.md
[META-128]: ../work-item/closed/done/META-128.md
[REFACTOR-041]: ../work-item/closed/done/REFACTOR-041.md
[REFACTOR-047]: ../work-item/closed/done/REFACTOR-047.md
[REFACTOR-049]: ../work-item/closed/done/REFACTOR-049.md
[REFACTOR-052]: ../work-item/closed/done/REFACTOR-052.md
[REFACTOR-053]: ../work-item/closed/done/REFACTOR-053.md
[REFACTOR-054]: ../work-item/closed/done/REFACTOR-054.md
[REFACTOR-055]: ../work-item/closed/done/REFACTOR-055.md
[REFACTOR-058]: ../work-item/closed/done/REFACTOR-058.md
[REFACTOR-059]: ../work-item/closed/done/REFACTOR-059.md
[REFACTOR-060]: ../work-item/closed/done/REFACTOR-060.md
[REFACTOR-061]: ../work-item/closed/done/REFACTOR-061.md
[REFACTOR-062]: ../work-item/closed/done/REFACTOR-062.md
[REFACTOR-063]: ../work-item/closed/done/REFACTOR-063.md
[REFACTOR-064]: ../work-item/closed/done/REFACTOR-064.md
[REFACTOR-066]: ../work-item/closed/done/REFACTOR-066.md
[REFACTOR-067]: ../work-item/closed/done/REFACTOR-067.md
[REFACTOR-074]: ../work-item/closed/done/REFACTOR-074.md
[REFACTOR-075]: ../work-item/closed/done/REFACTOR-075.md
[REFACTOR-104]: ../work-item/closed/done/REFACTOR-104.md
[REFACTOR-112]: ../work-item/open/3-LOW/REFACTOR-112.md
[REFACTOR-125]: ../work-item/closed/done/REFACTOR-125.md
[REFACTOR-138]: ../work-item/closed/done/REFACTOR-138.md
[REFACTOR-140]: ../work-item/closed/done/REFACTOR-140.md
[REFACTOR-141]: ../work-item/closed/done/REFACTOR-141.md
[UI-085]: ../work-item/closed/done/UI-085.md
[UI-087]: ../work-item/closed/done/UI-087.md
[UI-088]: ../work-item/closed/done/UI-088.md
[UI-089]: ../work-item/closed/done/UI-089.md
[UI-143]: ../work-item/closed/done/UI-143.md
