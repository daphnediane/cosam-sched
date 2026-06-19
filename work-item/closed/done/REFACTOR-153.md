# REFACTOR-153: Unify widget/interchange DTO into a shared leaf crate

## Summary

Collapse the duplicated widget JSON types into one `schedule-widget-format` leaf
crate and fix four format usability warts.

## Status

Completed

## Priority

Medium

## Description

`schedule-layout` carried a hand-mirrored copy of the widget JSON types
(`model::ScheduleData` and friends) kept in sync with `schedule_core::widget_json`
via a field-by-field mapper in `ScheduleData::from_schedule`. The two type sets
had silently drifted (`i32`↔`i64` ids, `Option<u32>`↔`i32` duration, `colors`
map vs typed struct, timeline `description` vs `name`, a synthetic required
`Presenter.uid`). The mapper hid this for the in-process path, but real widget
JSON crossing the WASM boundary on `feature/widget-print-typst` failed to parse
(`missing field uid`, `invalid type: sequence, expected a map`).

This unifies all consumers onto one serde DTO and improves the format while the
shape was being touched.

## Implementation Details

- New leaf crate `crates/schedule-widget-format` (serde/serde_json/thiserror
  only) holds the single DTO (`WidgetExport`, `WidgetPanel`, `WidgetPanelType`,
  `WidgetPanelColors`, `WidgetRoom`, `WidgetTimeline`, `WidgetPresenter`,
  `WidgetMeta`) plus shared accessors (`scheduled_panels`, `break_panels`,
  `sorted_rooms`, `is_series_continuation`, `from_json`, `load`).
- `schedule-core` re-exports the DTO from `widget_json::types`; export/import
  updated for the new fields.
- `schedule-layout` deletes its mirror structs + mapper; `model` aliases the DTO
  (`ScheduleData = WidgetExport`, …) and `from_schedule` is a thin wrapper over
  `export_to_widget_json`. Internal room uids moved `i64` → `i32` to match.
- `cosam-convert` / `cosam-viewer` updated for the typed colors and
  `from_schedule` move.

### Format changes (format version 1)

1. `WidgetPanelType.prefix` carried inline (identity no longer reconstructed from
   the map key by list consumers).
2. `colors` is a typed object `{ color, bw }` instead of a stringly-typed map.
3. Timeline label renamed `description` → `name` (readers accept the old key via
   serde alias / JS fallback).
4. `meta.version` bumped `0` → `1` as the single widget JSON format version.

## Notes / Deferred

- Making `schedule-core` an optional layout dependency for a lean WASM build is
  deferred: `schedule-layout/src/blocks/panels.rs` still uses
  `schedule_core::value::uniq_id::PanelUniqId` for prereq resolution. Relocating
  that is part of the `feature/widget-print-typst` rebase, not this pass.
- Presenter stable IDs and time/link field canonicalization were intentionally
  left out of this format pass.

## Acceptance Criteria

- [x] `cargo test --workspace` green; `cargo clippy --workspace` warning-free.
- [x] Layout no-regression: all 11 default-config documents (106 pages) render
      byte-identically (PNG page-hash diff vs pre-change baseline).
- [x] Widget JS builds and consumes the new format unchanged (changes are
      additive / unconsumed by the widget).
- [x] `schedule-widget-format` dep tree is serde-only (no schedule-core/chrono).
