# Cosplay America Schedule - Work Plan

Generated on: Mon Mar 16 16:07:21 2026

## Completed

* [BUGFIX-001](work-plan/BUGFIX-001.md) Converting the existing spreadsheets loses presenter information during the conversion process.
* [BUGFIX-002](work-plan/BUGFIX-002.md) Break events should only be visible when filtering by room or when no filters are applied.
* [BUGFIX-003](work-plan/BUGFIX-003.md) Remove "free" labeling from events as all events require registration.
* [BUGFIX-006](work-plan/BUGFIX-006.md) The converter does not detect or report scheduling conflicts such as a presenter double-booked across overlapping events, or two non-break events in the same room at the same time.
* [FEATURE-001](work-plan/FEATURE-001.md) Implement a two-part system for Cosplay America schedule management.
* [FEATURE-002](work-plan/FEATURE-002.md) Filter out SPLIT page-break markers and display BREAK time slots stretched across rooms.
* [FEATURE-007](work-plan/FEATURE-007.md) Replace hardcoded panel type colors with CSS-based UID reference system for theming.
* [UI-001](work-plan/UI-001.md) Display both the programming room name (e.g., "Programming 1") and the actual hotel room location.
* [UI-003](work-plan/UI-003.md) Implement theme switching with dark, light, and CosAm color modes.
* [UI-004](work-plan/UI-004.md) Replace table-based layout with CSS grid similar to schedule-to-html implementation.

---

## High Priority

### [ACCESSIBILITY-001] Accessibility Improvements

**Status:** In Progress

**Summary:** Implement comprehensive accessibility improvements for screen readers and color blindness support.

**Description:** Implement comprehensive accessibility improvements to ensure the schedule is usable by screen readers and users with various types of color blindness, following W3C WAI standards and achieving WCAG 2.1 AA compliance.

*See full details in: [work-plan/ACCESSIBILITY-001.md](work-plan/ACCESSIBILITY-001.md)*

---

### [BUGFIX-004] Hide staff only / private events from converted JSON

**Status:** Open

**Summary:** Filter out internal staff events from the public schedule JSON using the "Hidden" field in PanelTypes sheet and add `--staff` option to include private events.

**Description:** Staff-only events are being included in the public JSON output. These should be filtered out during conversion to maintain privacy and reduce clutter. The PanelTypes sheet already has a "Hidden" column for this purpose.

*See full details in: [work-plan/BUGFIX-004.md](work-plan/BUGFIX-004.md)*

---

### [BUGFIX-005] Support Hide Panelist and Alt Panelist fields

**Status:** Open

**Summary:** The converter ignores the "Hide Panelist" and "Alt Panelist" spreadsheet columns, so presenter suppression and override text are not honored in the JSON output.

**Description:** In the schedule-to-html spreadsheet format, two columns control presenter display:

- **Hide Panelist**: When non-blank (e.g. "Yes" or "*"), the event's presenter
  list should be suppressed entirely. This is used for events where listing the
  panelists is not appropriate (e.g. staff-run logistics panels).

- **Alt Panelist**: When set, the computed presenter list is replaced with this
  text (e.g. "Mystery Guest"). Useful for one-off presenters who don't have
  their own column or for special display.

Currently `Events.pm` reads presenter columns but never checks these fields,
so all detected presenters are unconditionally included in the JSON output.

See also: `docs/spreadsheet-format.md` and schedule-to-html README §Panelist.

*See full details in: [work-plan/BUGFIX-005.md](work-plan/BUGFIX-005.md)*

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

- ME100 "Market Expo" (13:00-18:00) in room 15
- FD001S1 "Learn to solder" (14:00-16:00) in room 15
- ME101 "Market Expo" (10:00-19:00) in room 15  
- FD001S2 "Learn to solder" (10:00-12:00) in room 15
- FD001S3 "Learn to solder" (14:00-16:00) in room 15

*See full details in: [work-plan/FEATURE-008.md](work-plan/FEATURE-008.md)*

---

### [FEATURE-009] Handle group presenter conflicts intelligently

**Status:** Open

**Summary:** Enable presenter conflict detection to distinguish between individual presenters and groups, allowing groups like "UNC Staff" to be scheduled in multiple panels simultaneously.

**Description:** Currently the converter flags conflicts when the same presenter name appears in overlapping events, but this creates false positives for presenter groups. Groups like "UNC Staff", "Pros and Cons", or "Guest Panelists" represent multiple people who can be in different panels at the same time.

The 2025 schedule shows this issue:

- UNC Staff scheduled for both "Parasol History and Construction" (10:00-11:00, room 4)
- UNC Staff also scheduled for "Reshaping the Body" (10:00-11:00, room 5)

This is not a real conflict since UNC Staff represents multiple staff members who can be split across different panels.

*See full details in: [work-plan/FEATURE-009.md](work-plan/FEATURE-009.md)*

---

### [UI-002] Fix event title and star overlap

**Status:** Open

**Summary:** Prevent event titles from overlapping with the "my schedule" star icon.

**Description:** Currently, long event titles can underlap the star icon, making both difficult to read.

*See full details in: [work-plan/UI-002.md](work-plan/UI-002.md)*

---

## Medium Priority

### [FEATURE-006] Add a compact printed schedule

**Status:** Open

**Summary:** Create a compact print format optimized for minimal paper usage.

**Description:** Some attendees prefer a pocket-sized schedule. A compact format with smaller fonts and condensed layout would be valuable.

*See full details in: [work-plan/FEATURE-006.md](work-plan/FEATURE-006.md)*

---

### [FEATURE-010] Enhance presenter group display and filtering in widget

**Status:** Open

**Summary:** Update the schedule widget to properly display presenter groups and allow filtering by both individual presenters and groups, following the group handling logic from the original implementation.

**Description:** The current widget displays presenters as a simple list of names, but doesn't handle the sophisticated group logic from the original schedule-to-html system. Users need to see groups properly formatted and be able to filter by groups like "UNC Staff" in addition to individual presenters.

*See full details in: [work-plan/FEATURE-010.md](work-plan/FEATURE-010.md)*

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

- Room conflicts (Market Expo vs Learn to solder sessions)
- Presenter conflicts (UNC Staff double-booked)

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

---

## Low Priority

### [FEATURE-004] Develop a standalone editor app

**Status:** Open

**Summary:** Create a cross-platform desktop application for schedule editing.

**Description:** Build a standalone editor (Electron/Node) that works on Windows and Mac for editing schedules and generating output.

*See full details in: [work-plan/FEATURE-004.md](work-plan/FEATURE-004.md)*

---

