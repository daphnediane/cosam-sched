# Cosplay America Schedule - Work Plan

Generated on: Sun Mar 15 21:40:21 2026

## High Priority

### [BUGFIX-001] Fix missing presenters

**Status:** Open

**Summary:** Converting the existing spreadsheets loses presenter information during the conversion process.

**Description:** The converter is not properly extracting presenter data from the spreadsheet columns. This results in events without presenter information in the generated JSON, which is critical for attendees to know who is running each event.

*See full details in: [work-plan/BUGFIX-001.md](work-plan/BUGFIX-001.md)*

---

### [BUGFIX-003] Do not list events as free

**Status:** Open

**Summary:** Remove "free" labeling from events as all events require registration.

**Description:** Currently, some events are marked as "free" which misleads attendees. All events require convention registration, only paid workshops have additional costs.

*See full details in: [work-plan/BUGFIX-003.md](work-plan/BUGFIX-003.md)*

---

### [BUGFIX-004] Hide staff only / private events from converted JSON

**Status:** Open

**Summary:** Filter out internal staff events from the public schedule JSON.

**Description:** Staff-only events are being included in the public JSON output. These should be filtered out during conversion to maintain privacy and reduce clutter.

*See full details in: [work-plan/BUGFIX-004.md](work-plan/BUGFIX-004.md)*

---

### [FEATURE-001] Interactive event calendar with spreadsheet-to-JSON converter

**Status:** Completed

Implement a two-part system for Cosplay America schedule management.

---

### [FEATURE-002] Handle SPLIT and BREAK special events

**Status:** Completed

Filter out SPLIT page-break markers and display BREAK time slots stretched across rooms.

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

### [UI-002] Fix event title and star overlap

**Status:** Open

**Summary:** Prevent event titles from overlapping with the "my schedule" star icon.

**Description:** Currently, long event titles can underlap the star icon, making both difficult to read.

*See full details in: [work-plan/UI-002.md](work-plan/UI-002.md)*

---

## Medium Priority

### [BUGFIX-002] Don't show breaks when any filter besides room is selected

**Status:** Open

**Summary:** Break events should only be visible when filtering by room or when no filters are applied.

**Description:** Currently, break events appear regardless of active filters (except room filter). This creates confusion as breaks should only show in the context of room schedules, not when filtering by type, cost, or presenter.

*See full details in: [work-plan/BUGFIX-002.md](work-plan/BUGFIX-002.md)*

---

### [FEATURE-006] Add a compact printed schedule

**Status:** Open

**Summary:** Create a compact print format optimized for minimal paper usage.

**Description:** Some attendees prefer a pocket-sized schedule. A compact format with smaller fonts and condensed layout would be valuable.

*See full details in: [work-plan/FEATURE-006.md](work-plan/FEATURE-006.md)*

---

### [UI-001] Rooms should list both the room name and the hotel room

**Status:** Open

**Summary:** Display both the programming room name (e.g., "Programming 1") and the actual hotel room location.

**Description:** Currently only the programming room names are shown. Attendees need to see both the programming designation and the actual hotel room number/location for easier navigation.

*See full details in: [work-plan/UI-001.md](work-plan/UI-001.md)*

---

### [UI-003] Add dark mode / light mode switch

**Status:** Open

**Summary:** Implement theme switching with dark, light, and CosAm color modes.

**Description:** Users want the option to switch between dark mode, light mode, and the default Cosplay America color scheme.

*See full details in: [work-plan/UI-003.md](work-plan/UI-003.md)*

---

## Low Priority

### [FEATURE-004] Develop a standalone editor app

**Status:** Open

**Summary:** Create a cross-platform desktop application for schedule editing.

**Description:** Build a standalone editor (Electron/Node) that works on Windows and Mac for editing schedules and generating output.

*See full details in: [work-plan/FEATURE-004.md](work-plan/FEATURE-004.md)*

---

