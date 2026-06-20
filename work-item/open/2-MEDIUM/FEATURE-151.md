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

### Phase 5 — Typst house-style parity (this iteration)

- `timeSplit` (`none`/`day`/`half_day`/`timeline`) and `sectionSplit`
  (`none`/`room`/`presenter`) added end-to-end: `WidgetPrintFormat`
  (`schedule-core`), `widget_config` bridge + normalizers (`cosam-convert`),
  the widget data model/coerce/edit-modal, and both default TOMLs. Unknown
  TOML keys stay silently ignored (so `orientation` etc. carry over from
  `layout.toml` shape); unknown split *values* pass through (the widget's
  `_coercePrintFormat` validates and falls back to `none`).
- `_doPrint` groups events with `_getTimeSplitGroups` and emits a separate grid
  per split section (page-broken), mirroring `document.rs` `time_sections` /
  `split_halves`. Compact day labels match `make_day_label` (weekday-only when
  the schedule fits one ISO week; `AM`/`PM` for half-day).
- Descriptions rebuilt as Typst-style blocks (`_buildPrintTimeGroupedDescriptions`):
  per-time-slot `== Day H PM` headings, title+presenter left / room+time right,
  no card background unless `cards`, no stray left accent, breaks excluded when
  a grid is shown. Font sizes scale from `base_font_pt`.
- Grid header + time column share the brand-primary fill; room columns fill the
  table; in `both` mode the grid claims `ceil(columns/2)` of the column budget.

### Phase 6 — Drop the print `<table>`, reuse the CSS-Grid engine

The print grid had regressed to an HTML `<table>` (against the original design,
which mandated CSS Grid). Replaced it by making the on-screen `_buildGridView`
print-aware (`printMode`): equal `1fr` time rows that fill the column height,
a `cosam-print-grid` class, and the interactive footer/star chrome dropped.
`_doPrint` now renders each time-split group as a `.cosam-print-section` page-
level multi-column flow (`column-fill: auto`, mirroring the legacy
`schedule-to-html` landscape print): the CSS-Grid schedule fills the left half,
descriptions sub-divide and flow into the right half and onto later pages. The
brand banner repeats per section (`column-span: all`) carrying the split label.
`_buildPrintGridTable` and all `cosam-print-grid-*` table CSS were deleted.
Fixes: missing per-page headers, uneven rows, grid not filling height,
descriptions not filling columns, grid not overflowing into the right half.

### Phase 7 — Remaining split grid/description parity (next iteration)

Comparing the widget print (landscape, `both` + `timeSplit: day`) against the
shipped Typst `sched-letter.pdf` (portrait letter, `both` + day split) surfaces
the following gaps. They are ordered by impact; the first is structural and
blocks the rest from being judged fairly.

#### 7a. Descriptions are clipped to one page per day (blocker)

The Typst `both` mode (`document.rs` `ContentMode::Both`) `#place`s the grid at
top-left covering `ceil(cols/2)` of the page width on the **first page only**,
then runs **one page-spanning `#columns()` flow** over the full width with
`grid_cols` leading `#colbreak()`s. Descriptions flow past the grid columns on
page 1 and reclaim the full page width on every later page — so a long day
paginates across as many pages as it needs. The legacy `schedule-to-html` does
the same with a page-level flow (`poster17x11.css`: `html { column-count: 2;
column-fill: auto }`, grid `break-before: column`).

The widget does **not** do this. `.cosam-print-section.cosam-print-has-grid.cosam-print-has-desc`
([cosam-calendar.css:2787-2819](widget/cosam-calendar.css#L2787-L2819)) is a
fixed `height: 100%` two-track CSS grid with `overflow: hidden`, and the
description track is itself `overflow: hidden`. Result: each day is clamped to a
single landscape page and any descriptions beyond that page are **silently
clipped** (widget = 5 pages with cut-off text; Typst = 10 pages, complete). The
Phase 6 note claims descriptions "flow onto later pages" but the shipped CSS
prevents it.

Fix direction: drop the fixed-height/overflow-hidden grid container for `both`
and reuse the legacy/Typst model — a page-level multi-column flow where the grid
region is `break-after: column` (or floated/placed) so it owns the first
column(s) on page 1 and descriptions flow through the remaining columns and onto
subsequent pages full-width. The per-section brand banner already uses
`column-span: all`, which fits a real column flow.

#### 7b. Description blocks omit most Typst panel content

`_buildPrintEventCard` ([cosam-calendar.js:3492-3536](widget/cosam-calendar.js#L3492-L3536))
renders only title, credits, a room/time line, and the description. The Typst
`panel_block` (`blocks/panels.rs`) additionally renders — and `sched-letter.pdf`
visibly shows — all of:

- **Cost** in the right-hand stack (`$50`, `$80 for the full series`).
- **Premium/workshop/capacity notices** (`*Premium workshop:* (Capacity: 20)
  Requires a separate purchase.`, `*Workshop:*`, `*Limited space:*`).
- **Notes, difficulty, and "This workshop is full."**
- **Prereq** lines (`Prereq: <panel>: <weekday time>`).
- **Cross-references**: `Part 1: Saturday 1 PM`, `or Part 2: …`, `Rerun at: …`.

Port `workshop_cap_notice` / `build_cross_refs` / prereq logic (or precompute
these into the widget JSON) so the description column carries the same
information.

#### 7c. Right-column metadata uses a literal backslash, no stacking, no cost

[cosam-calendar.js:3522](widget/cosam-calendar.js#L3522) builds
``metaText = `${room} \\ ${timeRange}` `` — the `\` is a leaked Typst line-break
token and prints literally (`Salon F \ 2 PM – 3 PM`). Typst's `build_right_column`
stacks **room / time / cost** vertically (right-aligned). Replace the literal
backslash with a stacked right column and add the cost line.

#### 7d. Orientation is not a format field

`_buildPrintCssVars` hardcodes `landscape` for every non-`panelList` mode
([cosam-calendar.js:4010-4011](widget/cosam-calendar.js#L4010-L4011)). The Typst
`LayoutConfig` has a first-class `orientation`, and the shipped `sched-letter`
reference is **portrait letter**. Promote `orientation` to a `WidgetPrintFormat`
field (it already flows untouched through the TOML bridge per Phase 5) and drive
`@page size` from it, so the widget can reproduce the portrait-letter house
style instead of only landscape.

#### 7e. Description slot headings ignore the compact day-label rule

`_buildPrintTimeGroupedDescriptions` ([cosam-calendar.js:3475](widget/cosam-calendar.js#L3475))
formats slot headers with `toLocaleDateString('en-US', { weekday: 'long' })`
unconditionally, while the section labels already use `makeCompactDayLabel`
(mirroring `make_day_label`). Use `makeCompactDayLabel` here too so `== Thursday
5 PM`-style headings match the Typst `== <day_label> <time>` exactly.

#### 7f. Print grid time column is solid primary, not a tint

The print rule paints both the header row **and** the time column with
`--cosam-print-header-bg` (solid brand primary)
([cosam-calendar.css:2850-2860](widget/cosam-calendar.css#L2850-L2860)), even
though `--cosam-print-time-col-bg` (a 15% tint) is computed for exactly this.
Typst tints the time column lighter than the header band. Point the print time
column at `--cosam-print-time-col-bg`.

#### 7g. Footer does not repeat per page / lacks page numbers

`_buildPrintFooter` appends one footer to the end of the print root
([cosam-calendar.js:4086-4087](widget/cosam-calendar.js#L4086-L4087)); Typst
emits a per-page footer with `Modified … | Generated … | Page X of Y | site`.

What is and isn't achievable in browser print:

- **GCPM running elements are out.** The CSS GCPM mechanism for true running
  footers — `position: running(footer)` + `content: element(footer)`
  (<https://www.w3.org/TR/css-gcpm-3/#running-elements>) — is implemented only
  by dedicated paged-media formatters (Prince, Antenna House, Weasyprint, the
  Paged.js polyfill). None of the target browsers (Chrome, Firefox, Safari,
  Edge) support it, and the widget prints through native `window.print()`, so a
  GCPM running footer would silently render nothing.
- **`Page X of Y` is out.** `counter(page)` / `counter(pages)` only resolve
  inside `@page` margin boxes (`@bottom-center { content: counter(page) }`),
  which browsers also do not support. So page numbers are not achievable in
  native browser print at all.
- **`position: fixed` is the one viable per-page footer.** A fixed-positioned
  element repeats on every printed page in Chrome and Firefox; Safari is
  unreliable (often first-page-only or overlapping content). Paged.js would give
  full GCPM but is a heavy runtime dep that re-flows the document into its own
  page boxes — it would fight the `column-fill: auto` flow 7a depends on, and the
  widget rules want deps minimal, so it is rejected.

**Decision (per maintainer):** a per-page footer carrying *just the timestamps*
(no page numbers) is acceptable and worth doing; if a clean timestamps-only
footer cannot be achieved, drop the footer entirely rather than ship a broken or
overlapping one. Implementation note: make `.cosam-print-footer`
`position: fixed` at the page bottom and reserve a bottom strip on the
full-height grid sections (`height: calc(100% - <footer-h>)`) so it never
overlaps the grid. The flowing description-only / panel-list modes cannot
reserve a per-page strip until 7a constrains per-page height, so the footer may
graze the bottom of flowed text there — acceptable, or gate the fixed footer to
grid-bearing modes. Genuine running footers with page numbers remain the
Typst PDF path's job. Also ensure the footer is not stranded on a final
mostly-blank page once 7a makes content paginate.

## Acceptance Criteria

- [x] `cosam-convert` emits `brand` + `printFormats` in the widget export
      (colors, base64/URL logos, web fonts, the shipped default formats).
- [ ] Widget seeds the print dropdown from shipped defaults; create/edit/rename/
      delete/reset persist to localStorage.
- [x] Print output applies brand header band, web fonts, columns, and B&W mode.
- [x] Iterate the widget print layout to approach the `schedule-layout` house
      style (implemented: time/section splits, grid styling, dynamic column allocation,
      Typst-style description layout, proper font sizing, break exclusion).
- [ ] `both` mode paginates: the grid occupies the first column(s) of a day's
      first page and descriptions flow full-width onto later pages without
      clipping (Phase 7a).
- [ ] Description blocks carry cost, premium/workshop/capacity notices, notes,
      prereqs, and part/rerun cross-references (Phase 7b–7c).
- [ ] `orientation` is a print-format field; the widget can emit portrait
      letter matching `sched-letter.pdf` (Phase 7d).
- [ ] Per-page timestamps-only footer per the maintainer decision above (or no
      footer if a clean one is unachievable).

## Notes

Web fonts/logos load from external URLs at print time (Squarespace-friendly);
they fail offline or under strict CSP. Logos may be base64-embedded or referenced
by URL (e.g. a Squarespace-hosted asset).
