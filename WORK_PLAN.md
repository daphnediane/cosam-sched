# Cosplay America Schedule - Work Plan

Generated on: Mon Mar 16 13:48:51 2026

## Completed

* [BUGFIX-001](work-plan/BUGFIX-001.md) Converting the existing spreadsheets loses presenter information during the conversion process.
* [BUGFIX-002](work-plan/BUGFIX-002.md) Break events should only be visible when filtering by room or when no filters are applied.
* [BUGFIX-003](work-plan/BUGFIX-003.md) Remove "free" labeling from events as all events require registration.
* [FEATURE-001](work-plan/FEATURE-001.md) Implement a two-part system for Cosplay America schedule management.
* [FEATURE-002](work-plan/FEATURE-002.md) Filter out SPLIT page-break markers and display BREAK time slots stretched across rooms.
* [UI-001](work-plan/UI-001.md) Display both the programming room name (e.g., "Programming 1") and the actual hotel room location.
* [UI-004](work-plan/UI-004.md) Replace table-based layout with CSS grid similar to schedule-to-html implementation.

---

## High Priority

### [ACCESSIBILITY-001] Accessibility Improvements

**Status:** Open

**Summary:** Implement comprehensive accessibility improvements for screen readers and color blindness support.

**Description:** Implement comprehensive accessibility improvements to ensure the schedule is usable by screen readers and users with various types of color blindness, following W3C WAI standards and achieving WCAG 2.1 AA compliance.

*See full details in: [work-plan/ACCESSIBILITY-001.md](work-plan/ACCESSIBILITY-001.md)*

---

### [BUGFIX-004] Hide staff only / private events from converted JSON

**Status:** Open

**Summary:** Filter out internal staff events from the public schedule JSON.

**Description:** Staff-only events are being included in the public JSON output. These should be filtered out during conversion to maintain privacy and reduce clutter.

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

### [FEATURE-007] Reference panel types by UID instead of hardcoding colors

**Status:** Open

**Summary:** Replace hardcoded panel type colors with CSS-based UID reference system for theming.

**Description:** Currently panel type colors are hardcoded in the event data, making it difficult to implement themes and maintain consistent styling. This change will make panel types reference UIDs and use CSS classes for colors, enabling proper theming support.

*See full details in: [work-plan/FEATURE-007.md](work-plan/FEATURE-007.md)*

---

### [UI-002] Fix event title and star overlap

**Status:** Open

**Summary:** Prevent event titles from overlapping with the "my schedule" star icon.

**Description:** Currently, long event titles can underlap the star icon, making both difficult to read.

*See full details in: [work-plan/UI-002.md](work-plan/UI-002.md)*

---

## Medium Priority

### [BUGFIX-006] Detect and warn about scheduling conflicts

**Status:** Open

**Summary:** The converter does not detect or report scheduling conflicts such as a presenter double-booked across overlapping events, or two non-break events in the same room at the same time.

**Description:** When building the schedule spreadsheet, mistakes happen — a presenter may be
marked as attending two events that overlap in time, or two events may be
accidentally assigned to the same room at the same time.

Currently the converter silently produces JSON with these conflicts, and the
widget displays overlapping events without any indication that something is
wrong. Neither tool provides any warning to the schedule author.

*See full details in: [work-plan/BUGFIX-006.md](work-plan/BUGFIX-006.md)*

---

### [FEATURE-006] Add a compact printed schedule

**Status:** Open

**Summary:** Create a compact print format optimized for minimal paper usage.

**Description:** Some attendees prefer a pocket-sized schedule. A compact format with smaller fonts and condensed layout would be valuable.

*See full details in: [work-plan/FEATURE-006.md](work-plan/FEATURE-006.md)*

---

### [UI-003] Add dark mode / light mode switch

**Status:** Open

**Summary:** Implement theme switching with dark, light, and CosAm color modes.

**Description:** Users want the option to switch between dark mode, light mode, and the default Cosplay America color scheme.

*See full details in: [work-plan/UI-003.md](work-plan/UI-003.md)*

---

### [UI-005] Implement sticky headers or additional header rows

**Status:** Open

**Summary:** Add sticky headers or repeat day headers between time blocks in grid view for better navigation.

**Description:** When viewing the schedule grid, users lose context of which day/time they're viewing as they scroll. Either sticky headers should follow the scroll, or additional header rows should be inserted between days to maintain context.

*See full details in: [work-plan/UI-005.md](work-plan/UI-005.md)*

---

## Low Priority

### [FEATURE-004] Develop a standalone editor app

**Status:** Open

**Summary:** Create a cross-platform desktop application for schedule editing.

**Description:** Build a standalone editor (Electron/Node) that works on Windows and Mac for editing schedules and generating output.

*See full details in: [work-plan/FEATURE-004.md](work-plan/FEATURE-004.md)*

---

