# FEATURE-116: cosam-viewer — cross-platform schedule viewer app (Dioxus)

## Summary

New Dioxus 0.7 viewer app that reads widget JSON and renders a UI similar to the JS widget.

## Status

In Progress

## Priority

Medium

## Description

Create `apps/cosam-viewer/` as a new Dioxus 0.7 app that:

- Reads the cosam widget JSON format (`docs/widget-json-format.md`) directly — no dependency on `schedule-core`
- Targets macOS (desktop), iPadOS and Android (mobile) via Dioxus feature flags and `dx` build tooling
- Mirrors the JS widget UX: day tabs, list view with time groups, filter panel (rooms + types +
  search), panel detail overlay, 4 themes

## Implementation Details

- Feature flags: `desktop` (default, uses `rfd` for file dialog), `mobile` (for `dx build --platform {ios,android}`)
- Data layer: `src/data/display.rs` — serde types matching widget JSON
- State: `src/state.rs` — `ViewerState` with view mode, day selection, filters, theme
- UI: `src/ui/app.rs` — single `App` component with toolbar, day tabs, filter panel, list view,
  panel detail modal
- CSS: `src/style.css` — 4 themes via CSS custom properties, panel color bars

## Child Work Items

- FEATURE-118: Grid view (rooms × time slots) — deferred
- FEATURE-119: My Schedule / bookmarking — deferred
- FEATURE-120: Mobile-specific build and deploy configuration — deferred

## Meta

- META-117: cosam-viewer tracker

## Acceptance Criteria

- `cargo check` passes for desktop feature
- Opens a widget JSON file and displays panels grouped by day and time
- Day tabs filter by day; room/type filters narrow the list; search narrows by name/description/presenter
- Clicking a panel opens a detail overlay with full info
- 4 themes selectable from toolbar
