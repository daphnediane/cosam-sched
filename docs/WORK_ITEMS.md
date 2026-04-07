# Cosplay America Schedule - Work Item

Updated on: Fri Apr 10 14:29:28 2026

## Completed

* [REFACTOR-001] Replace Schedule's string-based EdgeStorage with type-safe edge storage system using typed edge structs and dedicated storage per relationship type. Foundation is complete; remaining work is integration into Schedule and implementing relationship-specific behaviors.
* [REFACTOR-032] Align Panel entity fields with schedule-core canonical column definitions.
* [REFACTOR-033] Align EventRoom entity fields with schedule-core canonical column definitions.
* [REFACTOR-034] Align HotelRoom entity field aliases with schedule-core canonical column definitions.
* [REFACTOR-035] Align PanelType entity field aliases with schedule-core canonical column definitions.
* [REFACTOR-036] Align Presenter entity field aliases with schedule-core canonical column definitions.
* [REFACTOR-039] Remove `EntityId = u64` type alias and `InternalId` struct from `entity/mod.rs`; add `EntityKind` and `PublicEntityRef` enums; re-export `uuid::Uuid`.
* [REFACTOR-040] Replace `FieldValue::EntityId(EntityId)` with `FieldValue::Uuid(Uuid)` and remove `FieldValue::InternalId(InternalId)` from `field/mod.rs`.
* [REFACTOR-041] Introduce per-entity typed ID newtypes (`PanelId`, `PresenterId`, `EventRoomId`, `HotelRoomId`, `PanelTypeId`) each wrapping `uuid::Uuid`, replacing bare `u64` typed IDs.
* [REFACTOR-042] Rename `Edge::from_id()` and `Edge::to_id()` to `from_uuid()` and `to_uuid()` returning `Option<uuid::Uuid>`; update `RelationshipStorage` and `RelationshipEdge` trait signatures to use `Uuid`.
* [REFACTOR-043] Update `schedule-macro/src/lib.rs` to emit `entity_uuid: uuid::Uuid` in generated `*Data` structs, generate a `new()` constructor with `Uuid::now_v7()`, generate a `to_public()` method, and replace `FieldTypeCategory::EntityId`/`InternalId` with `Uuid`.
* [REFACTOR-044] Replace `HashMap<EntityId, Vec<EdgeId>>` outgoing/incoming indexes in `GenericEdgeStorage` with `HashMap<uuid::Uuid, Vec<EdgeId>>`.

---

## Summary of Open Items

**Total open items:** 37

* **High Priority**
  * [CLI-013] Port cosam-convert from schedule-core to schedule-data for XLSX-to-JSON conversion.
  * [CLI-014] Port cosam-modify from schedule-core to schedule-data for batch schedule editing.
  * [EDITOR-015] Port cosam-editor from schedule-core to schedule-data for the GPUI desktop editor.
  * [FEATURE-007] Define and implement a new JSON format version aligned with schedule-data's internal model.
  * [FEATURE-008] Implement display/public JSON export for the schedule widget, equivalent to schedule-core's display_export.
  * [FEATURE-009] Implement XLSX reading and writing against schedule-data entities, replacing schedule-core's xlsx module.
  * [FEATURE-010] Implement room and presenter conflict detection with support for room-wide event exemptions.
  * [REFACTOR-002] Align all entity fields with spreadsheet canonical columns and schedule-core equivalents.
  * [REFACTOR-003] Implement public query families for all entity types with ranked index matching.
  * [REFACTOR-004] Implement per-entity mutation families with deterministic side effects for add, update, restore, and find-or-add operations.
  * [REFACTOR-005] Implement command-based edit history with undo/redo stacks and atomic batch operations.
  * [REFACTOR-006] Implement derived scheduling-state propagation and complete the field validation system in schedule-macro.
  * [REFACTOR-031] Extract timeline entries (SPLIT, BREAK, room hours) into a dedicated TimelineEntry entity following the schedule-core pattern.
  * [REFACTOR-037] Migrate from internal u64-based entity IDs to standard UUID v4 for entities, schedules, and edges to enable cross-schedule ID sharing and simplify the public API.
  * [REFACTOR-038] Replace the `EdgeId(u64)` type with `EdgeId(uuid::Uuid)` and add an edge UUID registry to `Schedule` for cross-edge lookups.
  * [REFACTOR-045] Update all five concrete edge implementation files to use typed `*Id(Uuid)` constructors and implement `from_uuid()`/`to_uuid()` from the `Edge` trait.
  * [REFACTOR-046] Replace `HashMap<u64, StoredEntity>` and `u64`-keyed internals in `schedule/storage.rs` with `HashMap<uuid::Uuid, StoredEntity>`.
  * [REFACTOR-047] Remove `IdAllocators` from `Schedule`, add `schedule_id: Uuid` to `ScheduleMetadata`, add a private entity UUID registry, implement `Schedule::fetch_uuid`, and update all typed entity/edge method signatures to use typed ID wrappers.
  * [REFACTOR-048] Expose `Schedule::type_of_uuid` and `Schedule::lookup_uuid` using the private entity registry added in REFACTOR-047, and add `EntityRef<'a>` as the borrowed-data return type for `lookup_uuid`.
  * [REFACTOR-049] Update the four existing integration test files to use `Uuid` instead of `EntityId`/`InternalId`, and add new tests for `fetch_uuid` and `lookup_uuid`.
  * [TEST-028] Comprehensive integration tests validating schedule-data against schedule-core behavior with real schedule data.
  * [UI-018] Implement comprehensive accessibility for the schedule widget: screen readers, color blindness support, and keyboard navigation.
  * [UI-019] Prevent panel titles from overlapping with the "my schedule" star icon in the schedule widget.

* **Medium Priority**
  * [EDITOR-016] Implement inline editing of individual panel properties in the editor.
  * [EDITOR-017] Implement functional settings window with export preferences and application configuration.
  * [FEATURE-011] Support nested group membership where a group's members can include other groups.
  * [REFACTOR-029] Replace GenericEdgeStorage usage for PanelToEventRoom with a specialized PanelToEventRoomStorage implementation similar to other edge types, adding any relationship-specific behaviors if needed.
  * [UI-020] Add visual indicators to the schedule widget to highlight conflicting panels.
  * [UI-021] Add sticky day/time headers and a separate room hours section to the schedule widget.
  * [UI-022] Add grid view and compact print format options for the schedule widget.

* **Low Priority**
  * [DEPLOY-025] Package the editor as standalone executables for macOS, Windows, and Linux.
  * [EDITOR-023] Enable drag-and-drop to move panels between time slots and rooms in the editor.
  * [EDITOR-024] Define how multiple people and devices can safely edit a single schedule with conflict handling.
  * [EDITOR-026] Support reading from and writing to Excel files stored in OneDrive.
  * [EDITOR-027] Revisit embedding a webview directly in the editor window once gpui_web is available.
  * [FEATURE-012] Enable reading schedule data directly from Google Sheets API.
  * [FEATURE-030] Document custom fields in schedule-data that are not present in schedule-core.

---

## Next Available IDs

The following ID numbers are available for new items:

**Available:** 050, 051, 052, 053, 054, 055, 056, 057, 058, 059

**Highest used:** 49

---

## Open CLI Items

### [CLI-013] Migrate cosam-convert to schedule-data

**Status:** Not Started

**Priority:** High

**Summary:** Port cosam-convert from schedule-core to schedule-data for XLSX-to-JSON conversion.

**Description:** cosam-convert currently uses schedule-core for XLSX import and JSON/HTML export. Migrate it to use schedule-data's `ScheduleFile` API, entity model, and export pipeline. This is the primary CLI tool for producing the schedule widget's JSON.

---

### [CLI-014] Migrate cosam-modify to schedule-data

**Status:** Not Started

**Priority:** High

**Summary:** Port cosam-modify from schedule-core to schedule-data for batch schedule editing.

**Description:** cosam-modify is the CLI batch editing tool (103KB main.rs). Migrate it from schedule-core's `EditCommand` system to schedule-data's mutation API and edit history.

---

## Open DEPLOY Items

### [DEPLOY-025] Application Packaging and Distribution

**Status:** Not Started

**Priority:** Low

**Summary:** Package the editor as standalone executables for macOS, Windows, and Linux.

**Description:** Set up build and packaging pipelines to produce distributable application bundles. Users should be able to download and run the editor without installing Rust or other development tools.

---

## Open EDITOR Items

### [EDITOR-015] Migrate cosam-editor to schedule-data

**Status:** Not Started

**Priority:** High

**Summary:** Port cosam-editor from schedule-core to schedule-data for the GPUI desktop editor.

**Description:** cosam-editor currently uses schedule-core data types and its snapshot-based undo system. Migrate to schedule-data's entity model, mutation API, and (optionally) edit history. Evaluate whether to keep snapshot undo or migrate to command-based undo (from old EDITOR-025).

---

### [EDITOR-016] Panel Editing UI

**Status:** Not Started

**Priority:** Medium

**Summary:** Implement inline editing of individual panel properties in the editor.

**Description:** Allow users to click on a panel card to edit its properties: name, description, time, room assignment, panel type, presenters, and flags. Changes should update the in-memory schedule model via schedule-data mutations and mark the file as dirty.

---

### [EDITOR-017] Editor Settings and Preferences

**Status:** Not Started

**Priority:** Medium

**Summary:** Implement functional settings window with export preferences and application configuration.

**Description:** Add a settings system for the editor, including export preferences (minification, file paths, templates), application preferences (theme, auto-save), and settings persistence. Infrastructure already exists in `settings.rs` and `ui/settings_window.rs`.

---

### [EDITOR-023] Drag-and-Drop Panel Scheduling

**Status:** Not Started

**Priority:** Low

**Summary:** Enable drag-and-drop to move panels between time slots and rooms in the editor.

**Description:** Implement a grid or timeline view in the editor where panels can be dragged to change their time or room assignment, providing an intuitive visual scheduling experience.

---

### [EDITOR-024] Multi-Device Schedule Sync Strategy

**Status:** Not Started

**Priority:** Low

**Summary:** Define how multiple people and devices can safely edit a single schedule with conflict handling.

**Description:** Design the synchronization and conflict-resolution model for concurrent editing across desktop clients. Backend-agnostic so it can support Google Sheets, OneDrive, or future storage options without rewriting core merge behavior.

---

### [EDITOR-026] OneDrive/Office 365 Integration

**Status:** Not Started

**Priority:** Low

**Summary:** Support reading from and writing to Excel files stored in OneDrive.

**Description:** Enable the editor to work with XLSX files shared via OneDrive/Office 365. This supports workflows where the schedule spreadsheet lives in a shared OneDrive folder.

---

### [EDITOR-027] Embedded Webview Preview

**Status:** Not Started

**Priority:** Low

**Summary:** Revisit embedding a webview directly in the editor window once gpui_web is available.

**Description:** The editor currently opens schedule previews in the system browser using a temporary HTML file with auto-reload polling. Once `gpui_web` becomes available, embed the preview directly inside the editor for side-by-side editing.

---

## Open FEATURE Items

### [FEATURE-007] JSON Format V11 — Schedule-Data Native Format

**Status:** Not Started

**Priority:** High

**Summary:** Define and implement a new JSON format version aligned with schedule-data's internal model.

**Description:** The current JSON format (V10) was designed around schedule-core's data model. Schedule-data uses a fundamentally different internal model (monotonic IDs, entity/edge split, field system). A new format version is needed to serialize/deserialize schedule-data's `Schedule` struct directly.

---

### [FEATURE-008] Display JSON Export

**Status:** Not Started

**Priority:** High

**Summary:** Implement display/public JSON export for the schedule widget, equivalent to schedule-core's display_export.

**Description:** The schedule widget consumes a display-oriented JSON format (currently V10-display) that differs from the full internal format. Implement export from schedule-data's `Schedule` to this display format, porting logic from `schedule-core/src/file/display_export.rs`.

---

### [FEATURE-009] XLSX Import/Export for Schedule-Data

**Status:** Not Started

**Priority:** High

**Summary:** Implement XLSX reading and writing against schedule-data entities, replacing schedule-core's xlsx module.

**Description:** Port XLSX import/export from `schedule-core/src/xlsx/` into schedule-data, reading spreadsheet rows into entity Data structs via the field system and writing them back. The ScheduleFile abstraction should be the single entry point for all file I/O (from old CLEANUP-028).

---

### [FEATURE-010] Conflict Detection System

**Status:** Not Started

**Priority:** High

**Summary:** Implement room and presenter conflict detection with support for room-wide event exemptions.

**Description:** Port and improve conflict detection from schedule-core and the Perl converter into schedule-data. Detect room conflicts and presenter double-bookings, with proper handling of room-wide events (from old FEATURE-035).

---

### [FEATURE-011] Groups-of-Groups Presenter Processing

**Status:** Not Started

**Priority:** Medium

**Summary:** Support nested group membership where a group's members can include other groups.

**Description:** The `schedule-to-html` Perl project supported groups-of-groups, where a group's members list could include the name of another group. The current Rust code does not handle this. Implement recursive group expansion with cycle detection for credit resolution and conflict detection.

---

### [FEATURE-012] Google Sheets Integration

**Status:** Not Started

**Priority:** Low

**Summary:** Enable reading schedule data directly from Google Sheets API.

**Description:** The convention is moving to Google Sheets. The converter and editor need to support reading from Google Sheets API in addition to XLSX files. This is transport/authentication only — multi-device sync is a separate concern.

---

### [FEATURE-030] Document Schedule-Data Custom Field Extensions

**Status:** Not Started

**Priority:** Low

**Summary:** Document custom fields in schedule-data that are not present in schedule-core.

**Description:** The schedule-data crate includes custom fields that are useful for the editor but are not present in schedule-core's XLSX/JSON processing. These fields should be documented to distinguish them from canonical fields.

---

## Open REFACTOR Items

### [REFACTOR-002] Field Alignment with Schedule-Core Canonical Columns

**Status:** Not Started

**Priority:** High

**Summary:** Align all entity fields with spreadsheet canonical columns and schedule-core equivalents.

**Description:** Review and align entity fields with `crates/schedule-core/src/xlsx/columns.rs` canonical column definitions. Ensure field names and aliases match `docs/spreadsheet-format.md` and JSON format specifications. Add any missing canonical fields.

---

### [REFACTOR-003] Query API Surface

**Status:** Not Started

**Priority:** High

**Summary:** Implement public query families for all entity types with ranked index matching.

**Description:** Port and extend query capabilities from `schedule-core/src/edit/find.rs` into `schedule-data`, providing typed query families per entity type with ranked `FieldSet` index matching.

---

### [REFACTOR-004] Mutation API and Side-Effect Semantics

**Status:** Not Started

**Priority:** High

**Summary:** Implement per-entity mutation families with deterministic side effects for add, update, restore, and find-or-add operations.

**Description:** Port mutation behavior from `schedule-core/src/edit/command.rs` into `schedule-data`, providing typed mutation families that handle side effects (e.g., creating dependent presenters/rooms/edges from panel edits) deterministically.

---

### [REFACTOR-005] Edit Commands, History, and Batch Undo/Redo

**Status:** Not Started

**Priority:** High

**Summary:** Implement command-based edit history with undo/redo stacks and atomic batch operations.

**Description:** Port `EditCommand` equivalents from `schedule-core/src/edit/command.rs` and `schedule-core/src/edit/history.rs` into `schedule-data`. Support atomic multi-step mutations as single undo steps.

---

### [REFACTOR-006] Scheduling Derivation and Field Validation

**Status:** Not Started

**Priority:** High

**Summary:** Implement derived scheduling-state propagation and complete the field validation system in schedule-macro.

**Description:** Implement derived scheduling state (scheduled/unscheduled) based on time_range presence and indirect references (presenter groups). Complete the `#[validate]` attribute in schedule-macro to generate `CheckedField` implementations.

---

### [REFACTOR-031] Separate TimelineEntry Entity from Panel

**Status:** Not Started

**Priority:** High

**Summary:** Extract timeline entries (SPLIT, BREAK, room hours) into a dedicated TimelineEntry entity following the schedule-core pattern.

**Description:** Currently timeline entries (SPLIT, BREAK, room hours) are stored as Panel entities with special flags. Per the schedule-core architecture, these should be in a separate TimelineEntry entity with its own storage. This aligns with how schedule-core handles timeline entries as a distinct `timeline: Vec<TimelineEntry>` field separate from panels.

---

### [REFACTOR-037] Replace EntityId with uuid::Uuid for all IDs

**Status:** In Progress

**Priority:** High

**Summary:** Migrate from internal u64-based entity IDs to standard UUID v4 for entities, schedules, and edges to enable cross-schedule ID sharing and simplify the public API.

**Description:** Currently the codebase uses `EntityId` (a crate-private u64 type alias) for internal entity identifiers and `InternalId` as a public wrapper. This design has limitations:

* Entity IDs cannot be shared across different schedules (e.g., a guest reinvited in a future year)
* Requires opaque wrapper to hide internal implementation
* Public API exposure of internal types

Replace with standard `uuid::Uuid` v4 for:

* All entity IDs (panels, presenters, rooms, etc.)
* Schedule IDs
* Edge IDs (relationships between entities)

Since UUIDs are standard and self-describing, they can be made public without an opaque wrapper. UUIDs are 128-bit (16 bytes) vs 64-bit (8 bytes) for u64, but this tradeoff is acceptable for the benefits:

* Standard RFC 4122 format
* Built-in collision resistance
* Can be serialized/deserialized reliably
* Enables cross-schedule entity tracking
* No need for opaque wrapper

---

### [REFACTOR-038] Migrate EdgeId from u64 to uuid::Uuid

**Status:** Blocked

**Priority:** High

**Summary:** Replace the `EdgeId(u64)` type with `EdgeId(uuid::Uuid)` and add an edge UUID registry to `Schedule` for cross-edge lookups.

**Description:** Currently `EdgeId` is a `(u64)` newtype generated sequentially in each edge storage. This is an internal counter with no cross-storage identity guarantees. Migrating to UUID v7 enables:

* Stable edge references across sessions and serialization round-trips
* Unified `Schedule::lookup_edge_uuid` registry alongside the entity UUID registry
* Consistent identity model for all objects in the schedule

This work is **blocked** on REFACTOR-039 through REFACTOR-049 (entity UUID migration) being complete. Once entity UUIDs are in place, edge UUIDs become the natural next step.

---

### [REFACTOR-045] Update edge implementation files to typed UUID IDs

**Status:** Open

**Priority:** High

**Summary:** Update all five concrete edge implementation files to use typed `*Id(Uuid)` constructors and implement `from_uuid()`/`to_uuid()` from the `Edge` trait.

**Description:** Part of REFACTOR-037. After the `Edge` trait is updated (REFACTOR-042) and typed ID wrappers exist (REFACTOR-041), the five concrete edge files need their constructors and `Edge` impl updated.

Files to update:

* `edge/panel_to_presenter.rs`
  * `new(panel_id: EntityId, presenter_id: EntityId)` → `new(panel_id: PanelId, presenter_id: PresenterId)`
  * `from_id: InternalId` → `from_id: PanelId`, `to_id: InternalId` → `to_id: PresenterId`
  * `impl Edge`: `from_uuid() -> Option<Uuid> { Some(self.from_id.0) }`, `to_uuid()` same pattern

* `edge/panel_to_panel_type.rs`
  * `new(panel_id: EntityId, panel_type_id: EntityId)` → `new(panel_id: PanelId, panel_type_id: PanelTypeId)`
  * Same `from_uuid`/`to_uuid` pattern

* `edge/panel_to_event_room.rs`
  * `new(panel_id: PanelId, room_id: EventRoomId)`
  * Same pattern

* `edge/event_room_to_hotel_room.rs`
  * `new(event_room_id: EventRoomId, hotel_room_id: HotelRoomId)`
  * Same pattern

* `edge/presenter_to_group.rs`
  * `PresenterToGroupEdge` enum variants use `PresenterId` instead of `InternalId`/`EntityId`
  * `PresenterToGroupStorage` inner `HashMap<EntityId, Vec<EntityId>>` maps → `HashMap<Uuid, Vec<Uuid>>`
  * `member_to_groups`, `group_to_members`, `groups`, `always_grouped` caches all use `Uuid` keys/values
  * `RelationshipStorage` impl: `get_inclusive_members(group_id: Uuid)`, `get_inclusive_groups(member_id: Uuid)`, `is_group(uuid: Uuid)`
  * Construction and lookup methods updated throughout

---

### [REFACTOR-046] Update entity storage to use Uuid keys

**Status:** Open

**Priority:** High

**Summary:** Replace `HashMap<u64, StoredEntity>` and `u64`-keyed internals in `schedule/storage.rs` with `HashMap<uuid::Uuid, StoredEntity>`.

**Description:** Part of REFACTOR-037. `EntityStorage` in `schedule/storage.rs` stores entities serialized as JSON strings, keyed by a `u64` internal ID. After the entity ID migration these keys become `uuid::Uuid`.

Changes to `crates/schedule-data/src/schedule/storage.rs`:

* `EntityTypeStorage::by_internal_id: HashMap<u64, ...>` → `HashMap<uuid::Uuid, ...>`
* `StoredEntity` struct: `internal_id: u64` field → `internal_uuid: uuid::Uuid`
* `EntityStorage::add_with_id(id: EntityId, ...)` → `add_with_uuid(uuid: Uuid, ...)`
* `EntityStorage::get(id: EntityId)` → `get(uuid: Uuid)`
* `EntityStorage::contains_id(id: EntityId)` → `contains_uuid(uuid: Uuid)`
* Update all internal `HashMap::get`, `HashMap::insert`, `HashMap::contains_key` calls to use `Uuid`
* Remove import of `EntityId`; add `use uuid::Uuid`

The `deserialize` function stub is kept as-is (it returns `None`); only the key type changes.

---

### [REFACTOR-047] Update Schedule core: remove allocators, add schedule_id and fetch_uuid

**Status:** Open

**Priority:** High

**Summary:** Remove `IdAllocators` from `Schedule`, add `schedule_id: Uuid` to `ScheduleMetadata`, add a private entity UUID registry, implement `Schedule::fetch_uuid`, and update all typed entity/edge method signatures to use typed ID wrappers.

**Description:** Part of REFACTOR-037. This is the main wiring phase that makes the UUID migration visible in the public `Schedule` API.

Changes to `crates/schedule-data/src/schedule/mod.rs`:

* Remove `IdAllocators` struct and all its uses (UUID generation no longer requires counters)
* Remove `pub type EdgeId = u64` (use `edge::EdgeId` which is unchanged)
* `ScheduleMetadata`: add `pub schedule_id: uuid::Uuid`; generate it in `ScheduleMetadata::new()` via `uuid::Uuid::now_v7()`
* `Schedule`: add private `entity_registry: HashMap<uuid::Uuid, crate::entity::EntityKind>`
* Update `add_entity` to insert `(data.uuid(), EntityKind::Panel)` (etc.) into `entity_registry`
* Implement `pub fn fetch_uuid(&self, uuid: uuid::Uuid) -> Option<crate::entity::PublicEntityRef>`:
  * Match `entity_registry.get(&uuid)` → `EntityKind::Panel` → look up in typed storage → call `data.to_public()` → wrap in `PublicEntityRef::Panel(...)`
  * Repeat for all five entity kinds
* Update all typed accessor/mutator methods to use typed ID parameters:
  * `get_panel_presenters(panel_id: PanelId) -> Vec<PresenterId>`
  * `connect_panel_to_presenter(panel_id: PanelId, presenter_id: PresenterId)`
  * `get_presenter_panels(presenter_id: PresenterId) -> Vec<PanelId>`
  * `connect_panel_to_event_room(panel_id: PanelId, room_id: EventRoomId)`
  * `connect_panel_to_panel_type(panel_id: PanelId, type_id: PanelTypeId)`
  * `connect_event_room_to_hotel_room(event_room_id: EventRoomId, hotel_room_id: HotelRoomId)`
  * (all similar methods throughout)
* `find_related` generic method: return `Vec<uuid::Uuid>` for the untyped path

---

### [REFACTOR-048] Add type_of_uuid, lookup_uuid, and EntityRef to Schedule

**Status:** Open

**Priority:** High

**Summary:** Expose `Schedule::type_of_uuid` and `Schedule::lookup_uuid` using the private entity registry added in REFACTOR-047, and add `EntityRef<'a>` as the borrowed-data return type for `lookup_uuid`.

**Description:** Part of REFACTOR-037. REFACTOR-047 added a private `entity_registry: HashMap<Uuid, EntityKind>` and `fetch_uuid` (which returns owned public data). This phase adds the remaining two registry methods for internal use:

* `pub fn type_of_uuid(&self, uuid: uuid::Uuid) -> Option<crate::entity::EntityKind>`
  * Simple registry dispatch — returns only the type tag
  * Useful for callers that already know how to use typed methods once they know the type

* `pub fn lookup_uuid(&self, uuid: uuid::Uuid) -> Option<crate::entity::EntityRef<'_>>`
  * Returns borrowed internal `*Data` via `EntityRef<'a>`
  * Useful for internal code that needs to inspect raw entity data without copying

New type `EntityRef<'a>` added to `entity/mod.rs`:

```rust
pub enum EntityRef<'a> {
    Panel(&'a PanelData),
    Presenter(&'a PresenterData),
    EventRoom(&'a EventRoomData),
    HotelRoom(&'a HotelRoomData),
    PanelType(&'a PanelTypeData),
}
```

`lookup_uuid` dispatches through the registry (same as `fetch_uuid`) but borrows the `*Data` struct rather than cloning it.

---

### [REFACTOR-049] Update and extend tests for UUID migration

**Status:** Open

**Priority:** High

**Summary:** Update the four existing integration test files to use `Uuid` instead of `EntityId`/`InternalId`, and add new tests for `fetch_uuid` and `lookup_uuid`.

**Description:** Part of REFACTOR-037. After all implementation phases are complete, the test suite needs to be updated to reflect the new UUID-based API and extended to cover the new registry methods.

Files to update in `crates/schedule-data/tests/`:

* `entity_fields_integration.rs` — replace `EntityId` references with `Uuid`; update entity construction to use generated `*Data::new(...)` constructors
* `direct_indexable_test.rs` — same EntityId → Uuid updates
* `indexable_fields_test.rs` — same updates
* `simple_indexable_test.rs` — same updates

New tests to add (can be in `entity_fields_integration.rs` or a new `uuid_registry_test.rs`):

* `test_schedule_metadata_has_uuid` — verify `ScheduleMetadata::new()` generates a non-nil `schedule_id`
* `test_fetch_uuid_panel` — add a panel to a schedule, call `fetch_uuid(panel.uuid())`, verify returned `PublicEntityRef::Panel` matches
* `test_fetch_uuid_unknown_returns_none` — call `fetch_uuid` with a random UUID, verify `None`
* `test_lookup_uuid_returns_borrowed_data` — verify `lookup_uuid` returns `EntityRef::Panel(&PanelData)` for a known panel
* `test_type_of_uuid` — verify `type_of_uuid` returns `Some(EntityKind::Panel)` for a known panel UUID and `None` for an unknown UUID
* `test_entity_data_new_generates_unique_uuids` — create two `PanelData::new(...)` instances, verify UUIDs differ
* `test_to_public_roundtrip` — create `PanelData`, call `to_public()`, verify all stored fields match

---

### [REFACTOR-029] Migrate PanelToEventRoom to Specialized Storage

**Status:** Not Started

**Priority:** Medium

**Summary:** Replace GenericEdgeStorage usage for PanelToEventRoom with a specialized PanelToEventRoomStorage implementation similar to other edge types, adding any relationship-specific behaviors if needed.

**Description:** PanelToEventRoom currently uses GenericEdgeStorage directly in Schedule. This should be migrated to a dedicated PanelToEventRoomStorage to maintain consistency with the edge system refactoring (REFACTOR-001).

---

## Open TEST Items

### [TEST-028] Integration Testing and Schedule-Core Parity Validation

**Status:** Not Started

**Priority:** High

**Summary:** Comprehensive integration tests validating schedule-data against schedule-core behavior with real schedule data.

**Description:** Before decommissioning schedule-core, validate that schedule-data produces equivalent results for all supported operations using representative real-world schedule data (e.g., 2025 convention schedule).

---

## Open UI Items

### [UI-018] Widget Accessibility Improvements

**Status:** In Progress

**Priority:** High

**Summary:** Implement comprehensive accessibility for the schedule widget: screen readers, color blindness support, and keyboard navigation.

**Description:** Ensure the schedule widget is usable by screen readers and users with color blindness, following WCAG 2.1 AA compliance. Initial work started: skip link, live results region, keyboard activation, focus styles, theme switcher with high-contrast option, CSS custom properties.

---

### [UI-019] Fix Panel Title and Star Overlap in Widget

**Status:** Not Started

**Priority:** High

**Summary:** Prevent panel titles from overlapping with the "my schedule" star icon in the schedule widget.

**Description:** Long panel titles can underlap the star icon, making both difficult to read. Adjust CSS to ensure proper spacing.

---

### [UI-020] Visual Conflict Indicators in Widget

**Status:** Not Started

**Priority:** Medium

**Summary:** Add visual indicators to the schedule widget to highlight conflicting panels.

**Description:** Display conflict data from JSON output in the widget. Conflicting panels should have visual cues (icons, colored borders), hover tooltips with details, and click-to-highlight related conflicts.

---

### [UI-021] Sticky Headers and Room Hours Display in Widget

**Status:** Not Started

**Priority:** Medium

**Summary:** Add sticky day/time headers and a separate room hours section to the schedule widget.

**Description:** Two related widget improvements: (1) sticky or repeated headers so users maintain day/time context while scrolling the grid, and (2) a dedicated room hours section that extracts room-wide events (registration, market expo, etc.) from the main grid into a separate readable display.

---

### [UI-022] Printable Schedule Options

**Status:** Not Started

**Priority:** Medium

**Summary:** Add grid view and compact print format options for the schedule widget.

**Description:** Currently printing only shows a list view. Add a grid view option similar to the on-screen grid, and a compact format optimized for minimal paper usage (pocket-sized schedule).

---

---

[CLI-013]: work-item/high/CLI-013.md
[CLI-014]: work-item/high/CLI-014.md
[DEPLOY-025]: work-item/low/DEPLOY-025.md
[EDITOR-015]: work-item/high/EDITOR-015.md
[EDITOR-016]: work-item/medium/EDITOR-016.md
[EDITOR-017]: work-item/medium/EDITOR-017.md
[EDITOR-023]: work-item/low/EDITOR-023.md
[EDITOR-024]: work-item/low/EDITOR-024.md
[EDITOR-026]: work-item/low/EDITOR-026.md
[EDITOR-027]: work-item/low/EDITOR-027.md
[FEATURE-007]: work-item/high/FEATURE-007.md
[FEATURE-008]: work-item/high/FEATURE-008.md
[FEATURE-009]: work-item/high/FEATURE-009.md
[FEATURE-010]: work-item/high/FEATURE-010.md
[FEATURE-011]: work-item/medium/FEATURE-011.md
[FEATURE-012]: work-item/low/FEATURE-012.md
[FEATURE-030]: work-item/low/FEATURE-030.md
[REFACTOR-001]: work-item/done/REFACTOR-001.md
[REFACTOR-002]: work-item/high/REFACTOR-002.md
[REFACTOR-003]: work-item/high/REFACTOR-003.md
[REFACTOR-004]: work-item/high/REFACTOR-004.md
[REFACTOR-005]: work-item/high/REFACTOR-005.md
[REFACTOR-006]: work-item/high/REFACTOR-006.md
[REFACTOR-029]: work-item/medium/REFACTOR-029.md
[REFACTOR-031]: work-item/high/REFACTOR-031.md
[REFACTOR-032]: work-item/done/REFACTOR-032.md
[REFACTOR-033]: work-item/done/REFACTOR-033.md
[REFACTOR-034]: work-item/done/REFACTOR-034.md
[REFACTOR-035]: work-item/done/REFACTOR-035.md
[REFACTOR-036]: work-item/done/REFACTOR-036.md
[REFACTOR-037]: work-item/high/REFACTOR-037.md
[REFACTOR-038]: work-item/high/REFACTOR-038.md
[REFACTOR-039]: work-item/done/REFACTOR-039.md
[REFACTOR-040]: work-item/done/REFACTOR-040.md
[REFACTOR-041]: work-item/done/REFACTOR-041.md
[REFACTOR-042]: work-item/done/REFACTOR-042.md
[REFACTOR-043]: work-item/done/REFACTOR-043.md
[REFACTOR-044]: work-item/done/REFACTOR-044.md
[REFACTOR-045]: work-item/high/REFACTOR-045.md
[REFACTOR-046]: work-item/high/REFACTOR-046.md
[REFACTOR-047]: work-item/high/REFACTOR-047.md
[REFACTOR-048]: work-item/high/REFACTOR-048.md
[REFACTOR-049]: work-item/high/REFACTOR-049.md
[TEST-028]: work-item/high/TEST-028.md
[UI-018]: work-item/high/UI-018.md
[UI-019]: work-item/high/UI-019.md
[UI-020]: work-item/medium/UI-020.md
[UI-021]: work-item/medium/UI-021.md
[UI-022]: work-item/medium/UI-022.md
