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

This feature brings much of the Typst layout power into the widget's print path:
a dropdown to create/manage custom print formats (mirroring the schedules UX)
exposing the subset of layout options that map to browser print, plus a branding
bridge so `cosam-convert` makes brand logos, colors, and web-equivalent print
fonts from `config/brand.toml` available to the widget. Web fonts load via a
Google Fonts `<link>` in the print window (e.g. Trend Sans One → Montserrat
SemiBold 600). On-screen widget fonts are out of scope.

## Implementation Details

### Phase 1 — Branding bridge (Rust; first commit)

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

### Phases 2–4 — Widget print formats (separate commit; in progress)

- Print-format data model, localStorage persistence, and CRUD mirroring the
  schedules model; seeded from `data.printFormats`.
- Print-format dropdown + edit modals; `_doPrint` consuming the active format
  (content mode, color/bw, columns, logo header band, footer, page fill, Google
  Fonts via `<link>` gated on `document.fonts.ready`).
- `load-html-embed.js` forwards `brand`/`printFormats` (the JSON-embed and
  data-URL loaders pass the whole object through unchanged).

## Acceptance Criteria

- [x] `cosam-convert` emits `brand` + `printFormats` in the widget export
      (colors, base64/URL logos, web fonts, the shipped default formats).
- [ ] Widget seeds the print dropdown from shipped defaults; create/edit/rename/
      delete/reset persist to localStorage.
- [ ] Print output applies brand header band, web fonts, columns, and B&W mode.
- [ ] Time/section splits, grid styling, dynamic column allocation, Typst-style
      description layout, font sizing, and break exclusion approach the house
      style.
- [ ] `both` mode paginates: the grid occupies the first column(s) of a day's
      first page and descriptions flow full-width onto later pages without
      clipping.
- [ ] Description blocks carry cost, premium/workshop/capacity notices, notes,
      prereqs, and part/rerun cross-references.
- [ ] `orientation` is a print-format field; the widget can emit portrait letter
      matching the shipped letter PDF.
- [ ] Per-page timestamps-only footer per the maintainer decision above (or no
      footer if a clean one is unachievable).

## Notes

Web fonts/logos load from external URLs at print time (Squarespace-friendly);
they fail offline or under strict CSP. Logos may be base64-embedded or referenced
by URL (e.g. a Squarespace-hosted asset).
