# Cosplay America Schedule - Work Item

Updated on: Fri Apr 10 14:29:23 2026

## Completed

* [REFACTOR-001] Replace Schedule's string-based EdgeStorage with type-safe edge storage system using typed edge structs and dedicated storage per relationship type. Foundation is complete; remaining work is integration into Schedule and implementing relationship-specific behaviors.
* [REFACTOR-032] Align Panel entity fields with schedule-core canonical column definitions.
* [REFACTOR-033] Align EventRoom entity fields with schedule-core canonical column definitions.
* [REFACTOR-034] Align HotelRoom entity field aliases with schedule-core canonical column definitions.
* [REFACTOR-035] Align PanelType entity field aliases with schedule-core canonical column definitions.

---

## Summary of Open Items

**Total open items:** 31

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
  * [REFACTOR-036] Align Presenter entity field aliases with schedule-core canonical column definitions.
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

**Available:** 037, 038, 039, 040, 041, 042, 043, 044, 045, 046

**Highest used:** 36

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

### [REFACTOR-036] Presenter Entity Field Alignment

**Status:** Not Started

**Priority:** High

**Summary:** Align Presenter entity field aliases with schedule-core canonical column definitions.

**Description:** Ensure Presenter entity field aliases include canonical forms from schedule-core for proper field resolution. Classification and groups/members handling already match schedule-core pattern.

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
[REFACTOR-036]: work-item/high/REFACTOR-036.md
[TEST-028]: work-item/high/TEST-028.md
[UI-018]: work-item/high/UI-018.md
[UI-019]: work-item/high/UI-019.md
[UI-020]: work-item/medium/UI-020.md
[UI-021]: work-item/medium/UI-021.md
[UI-022]: work-item/medium/UI-022.md
