# FEATURE-151: Brand bridge and widget print formats

## Summary

Carry branding and shipped print-format presets from config into the embedded
widget, and add custom, user-managed print formats to the widget.

## Status

In Progress

## Priority

Medium

## Description

The project has two output formatters: the powerful Typst-based PDF formatter
(`schedule-layout`, driven by `LayoutConfig` + `BrandConfig`) and the widget's
browser print, which previously offered only a hard-coded grid-vs-list choice
with system fonts and no branding.

This feature introduces a pluggable print architecture for the widget:

- **Core ships a simple print** as the always-available fallback: the on-screen
  CSS-Grid engine in a print mode with readable fonts, theme-aware colors, and
  proper pagination.
- **A branding bridge** so `cosam-convert` makes brand logos, colors, and
  web-equivalent print fonts from `config/brand.toml` available to the widget.
  Web fonts load via a Google Fonts `<link>` in the print window (e.g. Trend Sans
  One → Montserrat SemiBold 600). On-screen widget fonts are out of scope.
- **A `printPlugin` hook** on `CosAmCalendar.init` is the seam for opt-in plugins.
- **An advanced print-format plugin** (future work) reintroduces the rich format
  system: a dropdown to create/manage custom print formats (mirroring the
  schedules UX) exposing the subset of layout options that map to browser print,
  including time/section splits, Typst-style descriptions, brand header/footer,
  and dynamic column allocation. The advanced code is preserved on
  `feature/widget-print-formats` (commit `3f0effd`).

## Implementation Details

### Completed — Branding bridge (Phase 1)

- Web-equivalent font fields on `brand.toml [fonts]` (`{role}_web` family,
  weight, style, Google URL) + `web_font_specs()` (`schedule-layout/brand.rs`).
- `ScheduleBrand` / `SchedulePrintFont` / `SchedulePrintFormat` types and optional
  `brand` / `printFormats` fields on `ScheduleConfig` (`schedule-widget-format`).
- `brand_bridge`: `BrandConfig` → `ScheduleBrand` (logo http(s) URL passthrough or
  base64 data URL, color/font/meta mapping) (`cosam-convert`).
- `widget_config`: parse `config/widget-default.toml` (overridable by a
  gitignored `config/widget.toml`) into print-format presets (`cosam-convert`).
- Embed `brand` + `printFormats` in the widget-html structural JSON and the JSON
  export path (`main.rs`, `static_html.rs`); document both in the widget-json and
  widget-html format docs.
- Ship config/widget-default.toml with five default formats; document the
  new keys in brand.sample.toml and gitignore user config/widget.toml.
- Split schedule-widget-format into separate modules: config.rs (ScheduleConfig)
  and schedule.rs (WidgetExport).
- Rename Widget* types to Schedule* to unify naming with schedule-layout.
- Add version field to ScheduleConfig for format versioning.
- Remove config field from WidgetExport to completely separate concerns.
- Create widget-config-format.md documentation for the config format.

### Current direction — Pluggable print architecture

The advanced format system (originally built on `feature/widget-print-formats`,
commit `3f0effd`) had layout bugs and lived inside core `cosam-calendar.js`. To
keep core lean, print is being made **pluggable**:

- **Core ships a simple print** as the always-available fallback: the on-screen
  CSS-Grid engine in a print mode (`_buildGridView(events, printMode, fillPage)`),
  one day per page. It uses an even time axis (`evenTimeKeys` fills regular
  time-unit grid lines between event boundaries, mirroring `schedule-to-html`),
  `minmax(0, 1fr)` rows that fill the page (an over-full day clips panel content,
  never the hours), a readable font, theme-aware colors, stripped bracketing
  breaks, starred-pick marks, and the in-grid generated/modified footer.
- **A `printPlugin` hook** on `CosAmCalendar.init` is the seam: `_handlePrint`
  delegates to `printPlugin.print(ctx)` when registered (else the simple print),
  and the toolbar exposes an `extendToolbar` extension point. This generalizes
  the `feature/widget-print-typst` branch's `opts.pdfExportHook`.
- **The advanced print-format system becomes an opt-in plugin**
  (`PrintFormatPlugin`): the format CRUD/dropdown/edit-modal, time/section
  splits, Typst-style descriptions, brand header/footer, CSS-var injection. It
  reuses core's grid engine via `ctx.renderer._buildGridView(...)` and owns its
  own localStorage + toolbar UI. `cosam-convert` embeds the **simple** print by
  default until the plugin's layout bugs are fixed; a flag opts into the plugin.
- **Typst WASM PDF export (FEATURE-152)** rebases onto the same `printPlugin`
  seam as a second plugin (its `pdfExportHook` → `printPlugin`).
- The advanced code is preserved on `feature/widget-print-formats` (commit
  `3f0effd`) as the source to transplant into the plugin.

The positive-layered print CSS leaves a `format-print` layer slot for the plugin
to populate alongside the `common-print` base and the `simple-print` layer.

## Acceptance Criteria

- [x] `cosam-convert` emits `brand` + `printFormats` in the widget export
      (colors, base64/URL logos, web fonts, the shipped default formats).
- [x] Core ships a CSS-Grid simple print: even time-unit rows that fill
      the page, readable text, theme-aware colors, bracketing breaks stripped,
      starred-pick marks, generated/modified footer; plus a `printPlugin` seam on
      `CosAmCalendar.init` for opt-in plugins.
- [ ] Advanced print-format plugin reintroduces the format dropdown + CRUD,
      seeded from shipped defaults, persisted to localStorage (transplanted from
      `feature/widget-print-formats` commit `3f0effd`).
- [ ] Plugin print output applies brand header band, web fonts, columns, and B&W
      mode.
- [ ] Plugin approaches the `schedule-layout` house style — time/section splits,
      grid styling, dynamic column allocation, Typst-style descriptions, font
      sizing, break exclusion.
- [ ] `both` mode paginates: the grid occupies the first column(s) of a day's
      first page and descriptions flow full-width onto later pages without
      clipping.
- [ ] Description blocks carry cost, premium/workshop/capacity notices, notes,
      prereqs, and part/rerun cross-references.
- [ ] `orientation` is a print-format field; the widget can emit portrait
      letter matching `sched-letter.pdf`.
- [ ] Per-page timestamps-only footer per the maintainer decision (or no
      footer if a clean one is unachievable).

## Notes

Web fonts/logos load from external URLs at print time (Squarespace-friendly);
they fail offline or under strict CSP. Logos may be base64-embedded or referenced
by URL (e.g. a Squarespace-hosted asset).
