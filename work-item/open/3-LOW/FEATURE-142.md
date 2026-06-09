# FEATURE-142: Expand IDML export (grid, banners, columns, footers)

## Summary

Bring the IDML export toward Typst/PDF parity: schedule grid as an InDesign
table, page-header banners, multi-column body text, and page footers.

## Status

Open

## Priority

Low

## Blocked By

- FEATURE-110: IDML export v1 (text listing) must exist first — done

## Description

FEATURE-110 shipped a v1 IDML export: a threaded text listing of panels grouped
by day and time slot, with brand-driven paragraph styles. It deliberately
deferred the richer layout features that the Typst pipeline already produces.
This item closes that gap so an `.idml` job approaches the fidelity of its
`.pdf` counterpart for the same `LayoutConfig`.

The work builds on the existing `schedule-layout/src/idml.rs` module and reuses
the layout computations already feeding the Typst path
(`timegrid::GridLayout::compute`, `document::build_sections`,
`blocks::banner`/`blocks::grid`), emitting IDML XML instead of Typst source.

## Implementation Details

### Schedule grid as an InDesign `<Table>`

- For `ContentMode::GridOnly` and the grid half of `Both`, emit an InDesign
  `<Table>` inside the story instead of (or alongside) the text listing.
- Drive it from `GridLayout::compute`: `room_order` → columns, `time_slots` →
  rows, each `GridCell` → a `<Cell>` with `RowSpan` from `row_start`/`row_end`
  (covered cell positions are omitted, per IDML table rules).
- Add cell/table paragraph + cell styles (header row, time column, event cells);
  fill event cells with the panel-type swatch (see swatches below).
- Honor `empty_grid_fill` for no-event cells.

### Section splits

- Reuse `document::build_sections` (promote the needed items to `pub(crate)`) so
  room/presenter/day splits produce one IDML story/spread group per section,
  matching the Typst output's sectioning.

### Banners (page header)

- Emit a master-spread header banner (brand color bar + logo + running
  section/day labels), mirroring `blocks::banner`. Running labels map to IDML
  text variables or per-section master items.

### Columns

- Apply `LayoutConfig` column counts (`effective_columns`, the per-paper
  `description_columns`/`flyer_columns`) to body text frames via
  `TextFramePreference TextColumnCount` (and the flyer's grid-left / text-right
  split for `Both`).

### Footers

- Emit a footer per `FooterMode` (Full / TimestampOnly / None): modified/
  generated timestamps, auto page number, and the site/org label, mirroring
  `blocks::banner::page_footer*`.

### Color / swatches

- Define per-panel-type CMYK/RGB swatches in `Resources/Graphic.xml` and apply
  them to grid cells and (optionally) card accents, honoring `ColorMode`
  (color vs. grayscale).

### Pagination

- Replace the heuristic page-count estimate with a more robust approach (e.g.
  generous frame count plus a note when content may overset), or document the
  heuristic's limits.

## Acceptance Criteria

- `grid_only` / `both` IDML jobs render the schedule grid as an editable
  InDesign table that opens without errors
- Room / presenter / day splits produce the same sectioning as the Typst output
- Page-header banner and footer appear and match the brand configuration
- Multi-column body text honors the configured column counts
- Panel-type colors appear as named swatches (color and grayscale modes)
- `cargo test -p schedule-layout --features idml` passes; default build still
  builds without IDML dependencies

## Notes

Fonts remain referenced, not embedded (IDML never embeds fonts); a future item
could add a Document Fonts folder alongside the package. Embedded raster/vector
logos and advanced transforms can be staged after the table grid lands. Evaluate
after the grid whether full parity is worth the ongoing maintenance versus the
Typst/PDF path.
