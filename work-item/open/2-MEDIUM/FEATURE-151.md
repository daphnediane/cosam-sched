# FEATURE-151: Brand bridge and widget print formats

## Summary

Carry branding and shipped print-format presets from config into the embedded
widget, and add custom, user-managed print formats to the widget.

## Status

Open

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

## Scope

### Branding bridge (Rust)

- Web-equivalent font fields on `brand.toml [fonts]` (`{role}_web` family,
  weight, style, Google URL) surfaced as font specs in `schedule-layout`.
- Widget-facing brand / print-font / print-format types and optional `brand` /
  `printFormats` fields on the widget export (`schedule-core`).
- A brand bridge in `cosam-convert`: `BrandConfig` → widget brand (logo http(s)
  URL passthrough or base64 data URL, color/font/meta mapping).
- Widget print-format presets parsed from `config/widget-default.toml`
  (overridable by a gitignored `config/widget.toml`).
- Embed `brand` + `printFormats` in the widget-html structural JSON and the JSON
  export path; document both in the widget-json and widget-html format docs.

### Widget print formats

- Print-format data model, localStorage persistence, and CRUD mirroring the
  schedules model; seeded from the embedded `printFormats`.
- Print-format dropdown + edit modals; print consumes the active format
  (content mode, color/bw, columns, logo header band, footer, page fill, Google
  Fonts via `<link>` gated on font readiness).
- The HTML-embed loader forwards `brand` / `printFormats`; the JSON-embed and
  data-URL loaders pass the whole object through unchanged.

### Typst house-style parity

Iterate the widget print layout toward the `schedule-layout` house style:

- `timeSplit` (`none`/`day`/`half_day`/`timeline`) and `sectionSplit`
  (`none`/`room`/`presenter`) carried end-to-end through the TOML bridge and the
  widget data model, emitting a separate grid per split section (page-broken).
  Compact day labels mirror the Typst day-label rule (weekday-only when the
  schedule fits one ISO week; AM/PM for half-day). Unknown TOML keys and unknown
  split values are tolerated and validated/normalized in the widget.
- Descriptions rendered as Typst-style time-slot blocks: per-slot day/time
  headings, title+presenter left / room+time+cost right, card background only
  when `cards`, breaks excluded when a grid is shown, font sizes scaled from the
  base font size.
- Grid header + time column share the brand-primary fill (time column a lighter
  tint than the header band); in `both` mode the grid claims `ceil(columns/2)`
  of the column budget.
- Print grid uses the on-screen CSS-Grid engine in a print-aware mode (no HTML
  `<table>`): equal time rows that fill column height, interactive chrome
  dropped. Each split group is a page-level multi-column flow — grid fills the
  first column(s), descriptions sub-divide and flow into the remaining columns
  and onto later pages full-width without clipping. The brand banner repeats per
  section.
- Description blocks carry the same panel content as the Typst `panel_block`:
  cost, premium/workshop/capacity notices, notes, difficulty, prereq lines, and
  part/rerun cross-references (port the logic or precompute into the widget JSON).
- `orientation` promoted to a print-format field driving `@page size`, so the
  widget can reproduce the portrait-letter house style, not only landscape.

### Per-page footer (browser-print constraints)

The Typst PDF emits a true running footer (`Modified … | Generated … | Page X
of Y | site`). Native browser print cannot match this:

- **GCPM running elements are out.** `position: running(footer)` +
  `content: element(footer)` (<https://www.w3.org/TR/css-gcpm-3/#running-elements>)
  is implemented only by dedicated paged-media formatters (Prince, Antenna
  House, Weasyprint, the Paged.js polyfill), not by Chrome/Firefox/Safari/Edge.
- **`Page X of Y` is out.** `counter(page)` / `counter(pages)` only resolve in
  `@page` margin boxes, which browsers do not support.
- **`position: fixed` is the one viable per-page footer.** It repeats per page
  in Chrome/Firefox; Safari is unreliable. Paged.js would give full GCPM but is
  a heavy runtime dep that re-flows the document and fights the `column-fill:
  auto` flow the description pagination depends on, so it is rejected.

**Decision (per maintainer):** a per-page footer carrying *just the timestamps*
(no page numbers) is acceptable and worth doing; if a clean timestamps-only
footer cannot be achieved, drop the footer entirely rather than ship a broken or
overlapping one. Reserve a bottom strip on full-height grid sections so the
fixed footer never overlaps content; flowing description-only / panel-list modes
may graze the bottom of flowed text — acceptable, or gate the fixed footer to
grid-bearing modes. Ensure the footer is not stranded on a final mostly-blank
page. Genuine running footers with page numbers remain the Typst PDF path's job
(FEATURE-152).

## Acceptance Criteria

- [ ] `cosam-convert` emits `brand` + `printFormats` in the widget export
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
