# Cosplay America Schedule - Work Plan

Generated on: Wed Mar 18 00:34:54 2026

## Completed

* [BUGFIX-001](work-plan/BUGFIX-001.md) Converting the existing spreadsheets loses presenter information during the conversion process.
* [BUGFIX-002](work-plan/BUGFIX-002.md) Break events should only be visible when filtering by room or when no filters are applied.
* [BUGFIX-003](work-plan/BUGFIX-003.md) Remove "free" labeling from events as all events require registration.
* [BUGFIX-004](work-plan/BUGFIX-004.md) Filter out internal staff events from the public schedule JSON using the "Hidden" field in PanelTypes sheet and add `--staff` option to include private events.
* [BUGFIX-006](work-plan/BUGFIX-006.md) The converter does not detect or report scheduling conflicts such as a presenter double-booked across overlapping events, or two non-break events in the same room at the same time.
* [CLEANUP-001](work-plan/CLEANUP-001.md) Complete repository layout cleanup by moving planning outputs under `docs/`, relocating work-plan tools to `scripts/`, and retiring deprecated Perl converter paths.
* [CLEANUP-002](work-plan/CLEANUP-002.md) Migrate to an `apps/` + `crates/` Rust workspace layout, retire the legacy Perl converter now that parity is reached, and track the remaining non-blocking cleanup follow-up items.
* [EDITOR-500](work-plan/EDITOR-500.md) Add the ability to import schedule data from XLSX spreadsheets.
* [EDITOR-504](work-plan/EDITOR-504.md) Implement saving the schedule as JSON, matching the format consumed by the widget.
* [FEATURE-001](work-plan/FEATURE-001.md) Implement a two-part system for Cosplay America schedule management.
* [FEATURE-002](work-plan/FEATURE-002.md) Filter out SPLIT page-break markers and display BREAK time slots stretched across rooms.
* [FEATURE-007](work-plan/FEATURE-007.md) Replace hardcoded panel type colors with CSS-based UID reference system for theming.
* [FEATURE-009](work-plan/FEATURE-009.md) Enable presenter conflict detection to distinguish between individual presenters and groups, allowing groups like "UNC Staff" to be scheduled in multiple panels simultaneously.
* [FEATURE-010](work-plan/FEATURE-010.md) Update the schedule widget to properly display presenter groups and allow filtering by both individual presenters and groups, following the group handling logic from the original implementation.
* [UI-001](work-plan/UI-001.md) Display both the programming room name (e.g., "Programming 1") and the actual hotel room location.
* [UI-003](work-plan/UI-003.md) Implement theme switching with dark, light, and CosAm color modes.
* [UI-004](work-plan/UI-004.md) Replace table-based layout with CSS grid similar to schedule-to-html implementation.

---

## Summary of Open Items

**Total open items:** 21

* **High Priority**
  * [ACCESSIBILITY-001](work-plan/ACCESSIBILITY-001.md) Implement comprehensive accessibility improvements for screen readers and color blindness support.
  * [BUGFIX-005](work-plan/BUGFIX-005.md) The converter ignores the "Hide Panelist" and "Alt Panelist" spreadsheet columns, so presenter suppression and override text are not honored in the JSON output.
  * [EDITOR-501](work-plan/EDITOR-501.md) Add the ability to export schedule data to XLSX spreadsheets.
  * [EDITOR-502](work-plan/EDITOR-502.md) Implement inline editing of individual schedule events.
  * [FEATURE-003](work-plan/FEATURE-003.md) Enable reading schedule data directly from Google Sheets.
  * [FEATURE-005](work-plan/FEATURE-005.md) Add a grid view option to the printable schedule in addition to the existing list view.
  * [FEATURE-008](work-plan/FEATURE-008.md) Enable room-wide events like Market Expo to overlap with subpanels in the same room without triggering false conflict warnings.
  * [UI-002](work-plan/UI-002.md) Prevent event titles from overlapping with the "my schedule" star icon.

* **Medium Priority**
  * [EDITOR-503](work-plan/EDITOR-503.md) Detect and highlight scheduling conflicts between events.
  * [EDITOR-505](work-plan/EDITOR-505.md) Enable drag-and-drop to move events between time slots and rooms.
  * [EDITOR-506](work-plan/EDITOR-506.md) Implement undo/redo for all editing operations.
  * [EDITOR-509](work-plan/EDITOR-509.md) Package the editor as standalone executables for macOS, Windows, and Linux.
  * [EDITOR-510](work-plan/EDITOR-510.md) Define how multiple people and devices can safely edit a single schedule with conflict handling independent of any specific storage backend.
  * [FEATURE-006](work-plan/FEATURE-006.md) Create a compact print format optimized for minimal paper usage.
  * [UI-005](work-plan/UI-005.md) Add sticky headers or repeat day headers between time blocks in grid view for better navigation.
  * [UI-006](work-plan/UI-006.md) Add visual indicators to the schedule widget to highlight conflicting events, making it easy for users to identify and understand scheduling conflicts.
  * [UI-007](work-plan/UI-007.md) Update the room filter dropdown to only include rooms that have scheduled panels, excluding rooms that only contain room-hours events (RH prefix or "Is Room Hours" flag).
  * [UI-008](work-plan/UI-008.md) Add a dedicated room hours section to display operating hours for rooms with RH/Is Room Hours events, formatted by day and room type as shown in the example layout.

* **Low Priority**
  * [EDITOR-507](work-plan/EDITOR-507.md) Support reading from and writing to Google Sheets.
  * [EDITOR-508](work-plan/EDITOR-508.md) Support reading from and writing to Excel files stored in OneDrive.
  * [FEATURE-004](work-plan/FEATURE-004.md) Create a cross-platform desktop application for schedule editing.

---

## Open High Priority Items

### [ACCESSIBILITY-001] Accessibility Improvements

**Status:** In Progress

**Summary:** Implement comprehensive accessibility improvements for screen readers and color blindness support.

**Description:** Implement comprehensive accessibility improvements to ensure the schedule is usable by screen readers and users with various types of color blindness, following W3C WAI standards and achieving WCAG 2.1 AA compliance.

*See full details in: [work-plan/ACCESSIBILITY-001.md](work-plan/ACCESSIBILITY-001.md)*

---

### [BUGFIX-005] Support Hide Panelist and Alt Panelist fields

**Status:** Open

**Summary:** The converter ignores the "Hide Panelist" and "Alt Panelist" spreadsheet columns, so presenter suppression and override text are not honored in the JSON output.

**Description:** In the schedule-to-html spreadsheet format, two columns control presenter display:

* **Hide Panelist**: When non-blank (e.g. "Yes" or "*"), the event's presenter
  list should be suppressed entirely. This is used for events where listing the
  panelists is not appropriate (e.g. staff-run logistics panels).

* **Alt Panelist**: When set, the computed presenter list is replaced with this
  text (e.g. "Mystery Guest"). Useful for one-off presenters who don't have
  their own column or for special display.

Currently `Events.pm` reads presenter columns but never checks these fields,
so all detected presenters are unconditionally included in the JSON output.

See also: `docs/spreadsheet-format.md` and schedule-to-html README §Panelist.

*See full details in: [work-plan/BUGFIX-005.md](work-plan/BUGFIX-005.md)*

---

### [EDITOR-501] XLSX Export Support

**Status:** Open

**Summary:** Add the ability to export schedule data to XLSX spreadsheets.

**Description:** Implement writing schedule data to XLSX files using the `rust_xlsxwriter` crate. This allows round-tripping data back to spreadsheet format for sharing with non-technical staff.

*See full details in: [work-plan/EDITOR-501.md](work-plan/EDITOR-501.md)*

---

### [EDITOR-502] Event Editing UI

**Status:** Open

**Summary:** Implement inline editing of individual schedule events.

**Description:** Allow users to click on an event card to edit its properties: name, description, time, room assignment, panel type, presenters, and flags. Changes should update the in-memory schedule model and mark the file as dirty.

*See full details in: [work-plan/EDITOR-502.md](work-plan/EDITOR-502.md)*

---

### [FEATURE-003] Support Google Sheets for schedule data

**Status:** Open

**Summary:** Enable reading schedule data directly from Google Sheets.

**Description:** The convention is moving to Google Sheets next year. The converter needs to support reading from Google Sheets API in addition to XLSX files.

*See full details in: [work-plan/FEATURE-003.md](work-plan/FEATURE-003.md)*

---

### [FEATURE-005] Printable schedules should include a grid option

**Status:** Open

**Summary:** Add a grid view option to the printable schedule in addition to the existing list view.

**Description:** Currently, printing only shows a list view of events. A grid view similar to the on-screen grid would be useful for attendees who prefer a visual schedule layout.

*See full details in: [work-plan/FEATURE-005.md](work-plan/FEATURE-005.md)*

---

### [FEATURE-008] Allow room-wide events with subpanel overlaps

**Status:** Open

**Summary:** Enable room-wide events like Market Expo to overlap with subpanels in the same room without triggering false conflict warnings.

**Description:** Currently the converter flags conflicts when room-wide events (like Market Expo) overlap with scheduled subpanels (like Learn to solder workshops) in the same room. These overlaps are intentional - the room-wide event marks the overall operating hours while subpanels are specific activities within that timeframe.

The 2025 schedule shows this pattern:

* ME100 "Market Expo" (13:00-18:00) in room 15
* FD001S1 "Learn to solder" (14:00-16:00) in room 15
* ME101 "Market Expo" (10:00-19:00) in room 15  
* FD001S2 "Learn to solder" (10:00-12:00) in room 15
* FD001S3 "Learn to solder" (14:00-16:00) in room 15

*See full details in: [work-plan/FEATURE-008.md](work-plan/FEATURE-008.md)*

---

### [UI-002] Fix event title and star overlap

**Status:** Open

**Summary:** Prevent event titles from overlapping with the "my schedule" star icon.

**Description:** Currently, long event titles can underlap the star icon, making both difficult to read.

*See full details in: [work-plan/UI-002.md](work-plan/UI-002.md)*

## Open Medium Priority Items

### [EDITOR-503] Conflict Detection

**Status:** Open

**Summary:** Detect and highlight scheduling conflicts between events.

**Description:** Automatically identify events that overlap in the same room or involve the same presenter at the same time. Display conflicts visually and provide a summary view.

*See full details in: [work-plan/EDITOR-503.md](work-plan/EDITOR-503.md)*

---

### [EDITOR-505] Drag-and-Drop Event Scheduling

**Status:** Open

**Summary:** Enable drag-and-drop to move events between time slots and rooms.

**Description:** Implement a grid or timeline view where events can be dragged to change their time or room assignment. This provides an intuitive visual scheduling experience.

*See full details in: [work-plan/EDITOR-505.md](work-plan/EDITOR-505.md)*

---

### [EDITOR-506] Undo/Redo Support

**Status:** Open

**Summary:** Implement undo/redo for all editing operations.

**Description:** Track all changes to the schedule model and allow users to undo and redo them. Essential for a comfortable editing experience.

*See full details in: [work-plan/EDITOR-506.md](work-plan/EDITOR-506.md)*

---

### [EDITOR-509] Application Packaging and Distribution

**Status:** Open

**Summary:** Package the editor as standalone executables for macOS, Windows, and Linux.

**Description:** Set up build and packaging pipelines to produce distributable application bundles. Users should be able to download and run the editor without installing Rust or other development tools.

*See full details in: [work-plan/EDITOR-509.md](work-plan/EDITOR-509.md)*

---

### [EDITOR-510] Multi-Device Schedule Sync Strategy

**Status:** Open

**Summary:** Define how multiple people and devices can safely edit a single schedule with conflict handling independent of any specific storage backend.

**Description:** Design the synchronization and conflict-resolution model for concurrent editing across desktop clients. This is intentionally backend-agnostic so it can support Google Sheets, OneDrive, or future storage options without rewriting core merge behavior.

*See full details in: [work-plan/EDITOR-510.md](work-plan/EDITOR-510.md)*

---

### [FEATURE-006] Add a compact printed schedule

**Status:** Open

**Summary:** Create a compact print format optimized for minimal paper usage.

**Description:** Some attendees prefer a pocket-sized schedule. A compact format with smaller fonts and condensed layout would be valuable.

*See full details in: [work-plan/FEATURE-006.md](work-plan/FEATURE-006.md)*

---

### [UI-005] Implement sticky headers or additional header rows

**Status:** Open

**Summary:** Add sticky headers or repeat day headers between time blocks in grid view for better navigation.

**Description:** When viewing the schedule grid, users lose context of which day/time they're viewing as they scroll. Either sticky headers should follow the scroll, or additional header rows should be inserted between days to maintain context.

*See full details in: [work-plan/UI-005.md](work-plan/UI-005.md)*

---

### [UI-006] Visual conflict indicators in schedule widget

**Status:** Open

**Summary:** Add visual indicators to the schedule widget to highlight conflicting events, making it easy for users to identify and understand scheduling conflicts.

**Description:** The converter now includes conflict data in the JSON output, but the widget doesn't display this information to users. Users need visual cues to quickly identify conflicting events and understand what the conflicts are.

Based on the 2025 schedule data, conflicts include:

* Room conflicts (Market Expo vs Learn to solder sessions)
* Presenter conflicts (UNC Staff double-booked)

*See full details in: [work-plan/UI-006.md](work-plan/UI-006.md)*

---

### [UI-007] Improve room filter to exclude room-hours-only rooms

**Status:** Open

**Summary:** Update the room filter dropdown to only include rooms that have scheduled panels, excluding rooms that only contain room-hours events (RH prefix or "Is Room Hours" flag).

**Description:** Currently the room filter shows all rooms from the Rooms sheet, including rooms that only contain room-hours events like "Market Expo" or "Registration". These rooms clutter the filter and don't contain actual panels that users want to filter by.

*See full details in: [work-plan/UI-007.md](work-plan/UI-007.md)*

---

### [UI-008] Display room hours separately from schedule grid

**Status:** Open

**Summary:** Add a dedicated room hours section to display operating hours for rooms with RH/Is Room Hours events, formatted by day and room type as shown in the example layout.

**Description:** Room-hours events (RH prefix or "Is Room Hours" flag) currently appear in the main schedule grid, but they represent operating hours rather than specific panels. These should be displayed separately in a more readable format that shows when each area is open.

*See full details in: [work-plan/UI-008.md](work-plan/UI-008.md)*

## Open Low Priority Items

### [EDITOR-507] Google Sheets Integration

**Status:** Open

**Summary:** Support reading from and writing to Google Sheets.

**Description:** Enable the editor and converter CLI to read and write schedule data via Google Sheets, while keeping this item focused on transport/authentication and schema parity rather than multi-device sync strategy.

Current state: the Perl converter has an unverified Google Sheets path and has not been production-tested for this workflow. Rust support should include explicit validation against real sheets before considering this complete.

Legacy implementation notes from the removed Perl-era docs are archived in branch `feature/final-perl-converter` (`GOOGLE_SHEETS.md`, `google-sheets-config.example.yaml`). Key takeaways to carry forward for Rust:

* OAuth 2.0 credentials flow with explicit token-file handling
* Support direct Google Sheets URLs and robust spreadsheet ID extraction
* Handle both formal table metadata and heuristic range detection
* Validate auth, permissions, and error-path UX before calling the feature production-ready

*See full details in: [work-plan/EDITOR-507.md](work-plan/EDITOR-507.md)*

---

### [EDITOR-508] OneDrive/Office 365 Integration

**Status:** Open

**Summary:** Support reading from and writing to Excel files stored in OneDrive.

**Description:** Enable the editor to work with XLSX files shared via OneDrive/Office 365. This supports workflows where the schedule spreadsheet lives in a shared OneDrive folder.

*See full details in: [work-plan/EDITOR-508.md](work-plan/EDITOR-508.md)*

---

### [FEATURE-004] Develop a standalone editor app

**Status:** In Progress

**Summary:** Create a cross-platform desktop application for schedule editing.

**Description:** Build a standalone cross-platform desktop editor using Rust and GPUI for editing schedules and generating output. Supports macOS, Windows, and Linux.

*See full details in: [work-plan/FEATURE-004.md](work-plan/FEATURE-004.md)*
