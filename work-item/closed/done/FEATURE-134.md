# FEATURE-134: Flyer schedule layout (grid + descriptions per day)

## Summary

Add a double-sided per-day "flyer" print layout that places the day's schedule grid on the left half of each day's first page with panel descriptions flowing through the remaining columns and onto following full-width pages, one multi-day document with a page-number/timestamp footer.

## Status

Complete

## Priority

Medium

## Description

A new `flyer` layout format, similar in spirit to `room_signs` (grid + descriptions side by side) but producing a single multi-day booklet meant for double-sided printing — ideally one sheet per day.

Page model, per day:

- **First page (odd):** left half of the columns (rounded up — 2 of 4 on letter, 3 of 6 on legal+) holds the day's schedule grid; the remaining right columns hold descriptions.
- **Following pages:** zero or more full-width (4 or 6 column) pages carrying the remaining descriptions for that day.
- A blank page is inserted when needed so the next day starts on an odd page.

Column counts (landscape): **4 on letter**, **6 on legal and larger** (legal, tabloid, super-b, poster).

The day appears as a **running header** (upper-right of the banner, updated per page) and in the grid's **top-left corner cell** (the time column widens to fit the larger of the day word and `"Midnight"`); there is no day heading over the grid. Every page also carries a footer with the page number plus modified/generated timestamps, mirroring the widget's grid footer (`Modified: Jun 15 4:00 PM | Generated: …`).

## Implementation Details

- **Typst flow technique:** the schedule grid is emitted via `#place(top + left, box(width: N%))` (reserves no space) over the left columns; descriptions are one continuous `#columns(total)` flow. Leading `#colbreak()` × grid_cols push the first page's text past the grid into the right columns, while overflow continues full-width on subsequent pages. (`#grid`-cell overflow was rejected — it keeps overflow stuck at half width.)
- `crates/schedule-layout/src/formats/flyer.rs` — new builder; returns a single `(qualifier, source)` pair (empty qualifier → one document for the whole convention). `split_by` is ignored.
- `crates/schedule-layout/src/grid.rs` — `LayoutFormat::Flyer` variant; `PaperSize::flyer_columns(orientation)`.
- `crates/schedule-layout/src/blocks/banner.rs` — reusable `page_footer()` (timestamps + centered `counter(page)` page number + site/org) and `page_header_running()` (right label is raw Typst content).
- Running header: `#metadata("<day>") <flyer-day>` emitted at each day's start; the header reads `query(<flyer-day>).filter(page <= here().page).last()` (read-only query, no `state.update`, so layout converges).
- `crates/schedule-layout/src/blocks/grid.rs` — `GridRenderConfig.corner_label`: day in the corner cell; time-column `measure` widens to `calc.max(Midnight, label)`.
- Day-break uses `#pagebreak(to: "odd")`; footer needs a widened bottom margin (the shared preamble uses a near-zero bottom margin for edge-to-edge grids), set in `flyer.rs`.
- Dispatch wired in `apps/cosam-convert` (`parse_format` + match) and `apps/cosam-layout` (`cli.rs` `FormatArg::Flyer`, `--format flyer`, `default_stem`; `main.rs` `map_format` + generate).
- `config/layout-default.toml` — added `flyer` jobs for letter and legal.

## Acceptance Criteria

- `--format flyer` produces a single multi-day PDF.
- Each day's first page shows the grid in the left half and descriptions on the right; remaining descriptions continue full-width.
- Each day starts on an odd page (blank padding inserted as needed).
- Footer shows page number and modified/generated timestamps on every page.
- Letter → 4 columns, legal and larger → 6 columns (landscape).

## Notes

- Incidentally fixed a stale assertion in `blocks::grid::tests::test_grid_render_config_compact` (`8.5in` → `100%`) that had left the suite red.
- Footer timestamps are formatted from the RFC 3339 values as stored (UTC wall-clock); the widget renders in the viewer's local zone. Revisit if convention-local display is wanted.
- On letter, a day with many rooms makes the half-width grid tight; legal/larger is more legible. This matches the requested column counts.
