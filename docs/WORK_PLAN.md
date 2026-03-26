# Cosplay America Schedule - Work Plan

Generated on: Wed Mar 25 21:33:37 2026

## Completed

* [BUGFIX-001] Converting the existing spreadsheets loses presenter information during the conversion process.
* [BUGFIX-002] Break events should only be visible when filtering by room or when no filters are applied.
* [BUGFIX-003] Remove "free" labeling from events as all events require registration.
* [BUGFIX-004] Filter out internal staff events from the public schedule JSON using the "Hidden" field in PanelTypes sheet and add `--staff` option to include private events.
* [BUGFIX-005] The converter ignores the "Hide Panelist" and "Alt Panelist" spreadsheet columns, so presenter suppression and override text are not honored in the JSON output.
* [BUGFIX-006] The converter does not detect or report scheduling conflicts such as a presenter double-booked across overlapping events, or two non-break events in the same room at the same time.
* [BUGFIX-007] The `==Group` syntax in presenter headers incorrectly sets `always_grouped` on the member instead of `always_shown` on the group.
* [BUGFIX-008] Migrate Presenter.members and Presenter.groups from `Vec<String>` to `BTreeSet<String>` to prevent duplicates and maintain sorted order.
* [CLEANUP-001] Complete repository layout cleanup by moving planning outputs under `docs/`, relocating work-plan tools to `scripts/`, and retiring deprecated Perl converter paths.
* [CLEANUP-002] Migrate to an `apps/` + `crates/` Rust workspace layout, retire the legacy Perl converter now that parity is reached, and track the remaining non-blocking cleanup follow-up items.
* [EDITOR-500] Add the ability to import schedule data from XLSX spreadsheets.
* [EDITOR-501] Add the ability to export schedule data to XLSX spreadsheets.
* [EDITOR-503] Detect and highlight scheduling conflicts between events.
* [EDITOR-504] Implement saving the schedule as JSON, matching the format consumed by the widget.
* [FEATURE-001] Implement a two-part system for Cosplay America schedule management.
* [FEATURE-002] Filter out SPLIT page-break markers and display BREAK time slots stretched across rooms.
* [FEATURE-007] Replace hardcoded panel type colors with CSS-based UID reference system for theming.
* [FEATURE-009] Enable presenter conflict detection to distinguish between individual presenters and groups, allowing groups like "UNC Staff" to be scheduled in multiple panels simultaneously.
* [FEATURE-010] Update the schedule widget to properly display presenter groups and allow filtering by both individual presenters and groups, following the group handling logic from the original implementation.
* [FEATURE-011] Define the v5 JSON format for the schedule data, introducing a
base→part→session hierarchy, public/private split, and multi-room sessions.
* [FEATURE-012] Define the Rust data structures for the v5 JSON format in `crates/schedule-core`.
* [FEATURE-014] Update `xlsx_import` to directly build the v5 base→part→session hierarchy
when importing spreadsheet data.
* [FEATURE-015] Implement serialization of the v5 full/private JSON format from the
`Schedule` struct.
* [FEATURE-016] Implement the public export mode that flattens the v5 hierarchy into an
ordered `panels` array suitable for the `cosam-calendar.js` widget.
* [FEATURE-017] Update `widget/cosam-calendar.js` to consume the v5 public JSON format.
* [FEATURE-018] Update `apps/cosam-editor` to work with the v5 `Schedule` struct and expose
the base→part→session hierarchy in the UI.
* [FEATURE-020] Implement the `credits` field generation based on `always_shown`/`always_grouped` semantics from schedule-to-html.
* [FEATURE-021] Support the `<Name` prefix syntax in spreadsheet presenter headers to set `always_grouped` on individual members.
* [FEATURE-023] Implement the v7 JSON schedule format changes in the Rust codebase: panelTypes hashmap, named color sets, merged timeTypes, stable presenter IDs, baked-in breaks, and metadata fields.
* [FEATURE-501] Add a new command-line tool for in-place modifications of XLSX schedule files
* [INVESTIGATE-001] Investigate xlsx_update module for potential corruption issues and determine if it should be disabled
* [UI-001] Display both the programming room name (e.g., "Programming 1") and the actual hotel room location.
* [UI-003] Implement theme switching with dark, light, and CosAm color modes.
* [UI-004] Replace table-based layout with CSS grid similar to schedule-to-html implementation.
* [XLSXWRITE-001] Implement XLSX export enhancements including Grid sheet generation, Lstart/Lend calculated columns, and session conflict resolution with alpha suffix assignment.

---

## Summary of Open Items

**Total open items:** 26

* **High Priority**
  * [FEATURE-027] Enable reading schedule data directly from Google Sheets.
  * [FEATURE-031] Add a grid view option to the printable schedule in addition to the existing list view.
  * [FEATURE-035] Enable room-wide events like Market Expo to overlap with subpanels in the same room without triggering false conflict warnings.
  * [TEST-013] Add comprehensive tests for cosam-modify's new undo/redo/show-history commands
  * [UI-026] Prevent event titles from overlapping with the "my schedule" star icon.
  * [UI-038] Implement comprehensive accessibility improvements for screen readers and color blindness support.
  * [UI-502] Implement inline editing of individual schedule events.

* **Medium Priority**
  * [CLEANUP-028] Refactor xlsx module to be a specialization/implementation detail of ScheduleFile
  * [DEPLOY-509] Package the editor as standalone executables for macOS, Windows, and Linux.
  * [EDITOR-029] Add functional settings window with export preferences and application configuration options.
  * [EDITOR-510] Define how multiple people and devices can safely edit a single schedule with conflict handling independent of any specific storage backend.
  * [FEATURE-019] Populate the `metadata` field on all item types from non-standard spreadsheet columns during xlsx import.
  * [FEATURE-024] Create documentation for the new v8 JSON format with changeLog support
  * [FEATURE-033] Create a compact print format optimized for minimal paper usage.
  * [UI-032] Add sticky headers or repeat day headers between time blocks in grid view for better navigation.
  * [UI-034] Add visual indicators to the schedule widget to highlight conflicting events, making it easy for users to identify and understand scheduling conflicts.
  * [UI-036] Update the room filter dropdown to only include rooms that have scheduled panels, excluding rooms that only contain room-hours events (RH prefix or "Is Room Hours" flag).
  * [UI-037] Add a dedicated room hours section to display operating hours for rooms with RH/Is Room Hours events, formatted by day and room type as shown in the example layout.
  * [UI-505] Enable drag-and-drop to move events between time slots and rooms.
  * [UI-506] Implement undo/redo for all editing operations.

* **Low Priority**
  * [EDITOR-025] Migrate cosam-editor from snapshot-based undo/redo to EditHistory-based system
  * [EDITOR-507] Support reading from and writing to Google Sheets.
  * [EDITOR-508] Support reading from and writing to Excel files stored in OneDrive.
  * [EDITOR-511] Revisit embedding a webview directly in the editor window once gpui_web is available.
  * [FEATURE-030] Create a cross-platform desktop application for schedule editing.
  * [FEATURE-039] Support nested group membership (groups whose members include other groups) in presenter processing and credit resolution.

---

## Next Available IDs

The following ID numbers are available for new items:

**Available:** 022, 040, 041, 042, 043, 044, 045, 046, 047, 048, 049, 050, 051, 052, 053, 054

**Highest used:** 511

---

## Open CLEANUP Items

### [CLEANUP-028] ScheduleFile XLSX Subclassing

**Status:** Open

**Priority:** Medium

**Summary:** Refactor xlsx module to be a specialization/implementation detail of ScheduleFile

**Description:** Currently `xlsx/mod.rs` exposes `load_auto` and `save_auto` as public functions. A cleaner architecture would make these methods on `ScheduleFile` itself, with xlsx as an internal implementation detail.

---

## Open DEPLOY Items

### [DEPLOY-509] Application Packaging and Distribution

**Status:** Open

**Priority:** Medium

**Summary:** Package the editor as standalone executables for macOS, Windows, and Linux.

**Description:** Set up build and packaging pipelines to produce distributable application bundles. Users should be able to download and run the editor without installing Rust or other development tools.

---

## Open EDITOR Items

### [EDITOR-029] Implement Settings Window and Preferences

**Status:** Open

**Priority:** Medium

**Summary:** Add functional settings window with export preferences and application configuration options.

**Description:** Implement a complete settings system for the Cosam Editor application, including:

* Settings window accessible from Edit menu
* Export preferences (minification, file paths, templates)
* Application preferences (theme, shortcuts, etc.)
* Settings persistence using existing settings infrastructure
* Proper integration with GPUI window system

---

### [EDITOR-510] Multi-Device Schedule Sync Strategy

**Status:** Open

**Priority:** Medium

**Summary:** Define how multiple people and devices can safely edit a single schedule with conflict handling independent of any specific storage backend.

**Description:** Design the synchronization and conflict-resolution model for concurrent editing across desktop clients. This is intentionally backend-agnostic so it can support Google Sheets, OneDrive, or future storage options without rewriting core merge behavior.

---

### [EDITOR-025] cosam-editor EditHistory Migration

**Status:** Open

**Priority:** Low

**Summary:** Migrate cosam-editor from snapshot-based undo/redo to EditHistory-based system

**Description:** cosam-editor currently uses a snapshot-based undo/redo system (storing complete panel states) while other apps use the command-based EditHistory. This migration would unify the undo/redo approach across all applications.

---

### [EDITOR-507] Google Sheets Integration

**Status:** Open

**Priority:** Low

**Summary:** Support reading from and writing to Google Sheets.

**Description:** Enable the editor and converter CLI to read and write schedule data via Google Sheets, while keeping this item focused on transport/authentication and schema parity rather than multi-device sync strategy.

Current state: the Perl converter has an unverified Google Sheets path and has not been production-tested for this workflow. Rust support should include explicit validation against real sheets before considering this complete.

Legacy implementation notes from the removed Perl-era docs are archived in branch `feature/final-perl-converter` (`GOOGLE_SHEETS.md`, `google-sheets-config.example.yaml`). Key takeaways to carry forward for Rust:

* OAuth 2.0 credentials flow with explicit token-file handling
* Support direct Google Sheets URLs and robust spreadsheet ID extraction
* Handle both formal table metadata and heuristic range detection
* Validate auth, permissions, and error-path UX before calling the feature production-ready

---

### [EDITOR-508] OneDrive/Office 365 Integration

**Status:** Open

**Priority:** Low

**Summary:** Support reading from and writing to Excel files stored in OneDrive.

**Description:** Enable the editor to work with XLSX files shared via OneDrive/Office 365. This supports workflows where the schedule spreadsheet lives in a shared OneDrive folder.

---

### [EDITOR-511] Embedded Webview Preview

**Status:** Open

**Priority:** Low

**Summary:** Revisit embedding a webview directly in the editor window once gpui_web is available.

**Description:** The editor currently opens schedule previews in the system browser using a temporary HTML file with auto-reload polling. This works but requires context-switching between the editor and browser windows.

Once `gpui_web` (GPUI's planned web/webview integration) becomes available, revisit embedding the preview directly inside the editor window. This would allow side-by-side editing and preview without leaving the application.

---

## Open FEATURE Items

### [FEATURE-027] Support Google Sheets for schedule data

**Status:** Open

**Priority:** High

**Summary:** Enable reading schedule data directly from Google Sheets.

**Description:** The convention is moving to Google Sheets next year. The converter needs to support reading from Google Sheets API in addition to XLSX files.

---

### [FEATURE-031] Printable schedules should include a grid option

**Status:** Open

**Priority:** High

**Summary:** Add a grid view option to the printable schedule in addition to the existing list view.

**Description:** Currently, printing only shows a list view of events. A grid view similar to the on-screen grid would be useful for attendees who prefer a visual schedule layout.

---

### [FEATURE-035] Allow room-wide events with subpanel overlaps

**Status:** Open

**Priority:** High

**Summary:** Enable room-wide events like Market Expo to overlap with subpanels in the same room without triggering false conflict warnings.

**Description:** Currently the converter flags conflicts when room-wide events (like Market Expo) overlap with scheduled subpanels (like Learn to solder workshops) in the same room. These overlaps are intentional - the room-wide event marks the overall operating hours while subpanels are specific activities within that timeframe.

The 2025 schedule shows this pattern:

* ME100 "Market Expo" (13:00-18:00) in room 15
* FD001S1 "Learn to solder" (14:00-16:00) in room 15
* ME101 "Market Expo" (10:00-19:00) in room 15  
* FD001S2 "Learn to solder" (10:00-12:00) in room 15
* FD001S3 "Learn to solder" (14:00-16:00) in room 15

---

### [FEATURE-019] Populate metadata from spreadsheet extra columns

**Status:** Open

**Priority:** Medium

**Summary:** Populate the `metadata` field on all item types from non-standard spreadsheet columns during xlsx import.

**Description:** The `PanelSession` struct has an `extras: ExtraFields` field (renamed to `metadata` in v7) that is defined but never populated during xlsx import — it is always initialized as `IndexMap::new()`. The `row_to_map` function in `xlsx_import.rs` reads all columns into a HashMap, but only known fields are extracted via `get_field()`. The remaining unknown columns are silently discarded.

---

### [FEATURE-024] JSON Format Documentation

**Status:** Open

**Priority:** Medium

**Summary:** Create documentation for the new v8 JSON format with changeLog support

**Description:** The ScheduleFile refactor introduced JSON version 8 with an optional `changeLog` field. This needs proper documentation in the docs/json-format/ directory.

---

### [FEATURE-033] Add a compact printed schedule

**Status:** Open

**Priority:** Medium

**Summary:** Create a compact print format optimized for minimal paper usage.

**Description:** Some attendees prefer a pocket-sized schedule. A compact format with smaller fonts and condensed layout would be valuable.

---

### [FEATURE-030] Develop a standalone editor app

**Status:** In Progress

**Priority:** Low

**Summary:** Create a cross-platform desktop application for schedule editing.

**Description:** Build a standalone cross-platform desktop editor using Rust and GPUI for editing schedules and generating output. Supports macOS, Windows, and Linux.

---

### [FEATURE-039] Support groups-of-groups in presenter processing

**Status:** Open

**Priority:** Low

**Summary:** Support nested group membership (groups whose members include other groups) in presenter processing and credit resolution.

**Description:** The `schedule-to-html` project supported groups-of-groups, where a group's members list could include the name of another group. The current Rust code does not handle this case — group membership is assumed to be individuals only.

---

## Open TEST Items

### [TEST-013] cosam-modify Test Coverage

**Status:** Open

**Priority:** High

**Summary:** Add comprehensive tests for cosam-modify's new undo/redo/show-history commands

**Description:** cosam-modify now supports persistent undo/redo via EditHistory, but lacks automated tests for these features.

---

## Open UI Items

### [UI-026] Fix event title and star overlap

**Status:** Open

**Priority:** High

**Summary:** Prevent event titles from overlapping with the "my schedule" star icon.

**Description:** Currently, long event titles can underlap the star icon, making both difficult to read.

---

### [UI-038] Accessibility Improvements

**Status:** In Progress

**Priority:** High

**Summary:** Implement comprehensive accessibility improvements for screen readers and color blindness support.

**Description:** Implement comprehensive accessibility improvements to ensure the schedule is usable by screen readers and users with various types of color blindness, following W3C WAI standards and achieving WCAG 2.1 AA compliance.

---

### [UI-502] Event Editing UI

**Status:** Open

**Priority:** High

**Summary:** Implement inline editing of individual schedule events.

**Description:** Allow users to click on an event card to edit its properties: name, description, time, room assignment, panel type, presenters, and flags. Changes should update the in-memory schedule model and mark the file as dirty.

---

### [UI-032] Implement sticky headers or additional header rows

**Status:** Open

**Priority:** Medium

**Summary:** Add sticky headers or repeat day headers between time blocks in grid view for better navigation.

**Description:** When viewing the schedule grid, users lose context of which day/time they're viewing as they scroll. Either sticky headers should follow the scroll, or additional header rows should be inserted between days to maintain context.

---

### [UI-034] Visual conflict indicators in schedule widget

**Status:** Open

**Priority:** Medium

**Summary:** Add visual indicators to the schedule widget to highlight conflicting events, making it easy for users to identify and understand scheduling conflicts.

**Description:** The converter now includes conflict data in the JSON output, but the widget doesn't display this information to users. Users need visual cues to quickly identify conflicting events and understand what the conflicts are.

Based on the 2025 schedule data, conflicts include:

* Room conflicts (Market Expo vs Learn to solder sessions)
* Presenter conflicts (UNC Staff double-booked)

---

### [UI-036] Improve room filter to exclude room-hours-only rooms

**Status:** Open

**Priority:** Medium

**Summary:** Update the room filter dropdown to only include rooms that have scheduled panels, excluding rooms that only contain room-hours events (RH prefix or "Is Room Hours" flag).

**Description:** Currently the room filter shows all rooms from the Rooms sheet, including rooms that only contain room-hours events like "Market Expo" or "Registration". These rooms clutter the filter and don't contain actual panels that users want to filter by.

---

### [UI-037] Display room hours separately from schedule grid

**Status:** Open

**Priority:** Medium

**Summary:** Add a dedicated room hours section to display operating hours for rooms with RH/Is Room Hours events, formatted by day and room type as shown in the example layout.

**Description:** Room-hours events (RH prefix or "Is Room Hours" flag) currently appear in the main schedule grid, but they represent operating hours rather than specific panels. These should be displayed separately in a more readable format that shows when each area is open.

---

### [UI-505] Drag-and-Drop Event Scheduling

**Status:** Open

**Priority:** Medium

**Summary:** Enable drag-and-drop to move events between time slots and rooms.

**Description:** Implement a grid or timeline view where events can be dragged to change their time or room assignment. This provides an intuitive visual scheduling experience.

---

### [UI-506] Undo/Redo Support

**Status:** Open

**Priority:** Medium

**Summary:** Implement undo/redo for all editing operations.

**Description:** Track all changes to the schedule model and allow users to undo and redo them. Essential for a comfortable editing experience.

---

---

[BUGFIX-001]: work-plan/done/BUGFIX-001.md
[BUGFIX-002]: work-plan/done/BUGFIX-002.md
[BUGFIX-003]: work-plan/done/BUGFIX-003.md
[BUGFIX-004]: work-plan/done/BUGFIX-004.md
[BUGFIX-005]: work-plan/done/BUGFIX-005.md
[BUGFIX-006]: work-plan/done/BUGFIX-006.md
[BUGFIX-007]: work-plan/done/BUGFIX-007.md
[BUGFIX-008]: work-plan/done/BUGFIX-008.md
[CLEANUP-001]: work-plan/done/CLEANUP-001.md
[CLEANUP-002]: work-plan/done/CLEANUP-002.md
[CLEANUP-028]: work-plan/medium/CLEANUP-028.md
[DEPLOY-509]: work-plan/medium/DEPLOY-509.md
[EDITOR-025]: work-plan/low/EDITOR-025.md
[EDITOR-029]: work-plan/medium/EDITOR-029.md
[EDITOR-500]: work-plan/done/EDITOR-500.md
[EDITOR-501]: work-plan/done/EDITOR-501.md
[EDITOR-503]: work-plan/done/EDITOR-503.md
[EDITOR-504]: work-plan/done/EDITOR-504.md
[EDITOR-507]: work-plan/low/EDITOR-507.md
[EDITOR-508]: work-plan/low/EDITOR-508.md
[EDITOR-510]: work-plan/medium/EDITOR-510.md
[EDITOR-511]: work-plan/low/EDITOR-511.md
[FEATURE-001]: work-plan/done/FEATURE-001.md
[FEATURE-002]: work-plan/done/FEATURE-002.md
[FEATURE-007]: work-plan/done/FEATURE-007.md
[FEATURE-009]: work-plan/done/FEATURE-009.md
[FEATURE-010]: work-plan/done/FEATURE-010.md
[FEATURE-011]: work-plan/done/FEATURE-011.md
[FEATURE-012]: work-plan/done/FEATURE-012.md
[FEATURE-014]: work-plan/done/FEATURE-014.md
[FEATURE-015]: work-plan/done/FEATURE-015.md
[FEATURE-016]: work-plan/done/FEATURE-016.md
[FEATURE-017]: work-plan/done/FEATURE-017.md
[FEATURE-018]: work-plan/done/FEATURE-018.md
[FEATURE-019]: work-plan/medium/FEATURE-019.md
[FEATURE-020]: work-plan/done/FEATURE-020.md
[FEATURE-021]: work-plan/done/FEATURE-021.md
[FEATURE-023]: work-plan/done/FEATURE-023.md
[FEATURE-024]: work-plan/medium/FEATURE-024.md
[FEATURE-027]: work-plan/high/FEATURE-027.md
[FEATURE-030]: work-plan/medium/FEATURE-030.md
[FEATURE-031]: work-plan/high/FEATURE-031.md
[FEATURE-033]: work-plan/medium/FEATURE-033.md
[FEATURE-035]: work-plan/high/FEATURE-035.md
[FEATURE-039]: work-plan/low/FEATURE-039.md
[FEATURE-501]: work-plan/done/FEATURE-501.md
[INVESTIGATE-001]: work-plan/done/INVESTIGATE-001.md
[TEST-013]: work-plan/high/TEST-013.md
[UI-001]: work-plan/done/UI-001.md
[UI-003]: work-plan/done/UI-003.md
[UI-004]: work-plan/done/UI-004.md
[UI-026]: work-plan/high/UI-026.md
[UI-032]: work-plan/medium/UI-032.md
[UI-034]: work-plan/medium/UI-034.md
[UI-036]: work-plan/medium/UI-036.md
[UI-037]: work-plan/medium/UI-037.md
[UI-038]: work-plan/medium/UI-038.md
[UI-502]: work-plan/high/UI-502.md
[UI-505]: work-plan/medium/UI-505.md
[UI-506]: work-plan/medium/UI-506.md
[XLSXWRITE-001]: work-plan/done/XLSXWRITE-001.md
