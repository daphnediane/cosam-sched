# FEATURE-116: cosam-viewer — cross-platform schedule viewer app (Dioxus)

## Summary

New Dioxus 0.7 viewer app that reads widget JSON and renders a UI similar to the JS widget.

## Status

Completed

## Priority

Medium

## Description

Create `apps/cosam-viewer/` as a new Dioxus 0.7 app that:

- Reads the cosam widget JSON format (`docs/widget-json-format.md`) directly — no dependency on `schedule-core`
- Targets macOS (desktop), iPadOS and Android (mobile) via Dioxus feature flags and `dx` build tooling
- Mirrors the JS widget UX: day tabs, list view with time groups, filter panel (rooms + types +
  search), panel detail overlay, 4 themes

## Acceptance Criteria

- `cargo check` passes for desktop feature
- Opens widget JSON, XLSX, binary `.cosam`, and CSV directory schedules
- Displays panels grouped by day and time
- Day tabs filter by day; room/type filters narrow the list; search narrows by name/description/presenter
- Clicking a panel opens a detail overlay with full info
- 4 themes selectable from toolbar

## Implementation Details

- `apps/cosam-viewer/` Dioxus 0.7 app created with desktop and mobile feature flags
- Data layer: `src/data/display.rs` — serde types matching widget JSON
- State: `src/state.rs` — `ViewerState` with view mode, day selection, filters, theme
- UI: `src/ui/app.rs` — single `App` component with toolbar, day tabs, filter panel, list view, panel detail modal
- CSS: `src/style.css` — 4 themes via CSS custom properties, panel color bars

**Deferred features tracked as separate work items:**

- FEATURE-118: Grid view (rooms × time slots) — high priority
- FEATURE-119: My Schedule / bookmarking — medium priority
- FEATURE-120: Mobile-specific build and deploy configuration — low priority

**Completed enhancement:**

- FEATURE-121: Multi-format schedule loading (XLSX, binary, CSV, URL)

**Meta tracking:**

- META-117: cosam-viewer tracker for ongoing development
