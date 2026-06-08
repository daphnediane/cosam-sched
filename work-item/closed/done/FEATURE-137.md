# FEATURE-137: Configurable panel card style and page/grid tinting

## Summary

Add per-job layout options for a bordered "card" panel style, page background
tint, empty grid-cell fill, card fill, and column/panel gaps — all controllable
from `config/layout.toml`.

## Status

Completed

## Priority

Medium

## Description

The description layout drew every panel with a full-height left accent bar.
Adjacent panels of the same panel type produced touching, same-color bars that
visually fused ("bleeding"), and there was no way to tint the page or separate
panels into discrete cards without editing code.

This adds an opt-in **card** style (colored left spine + light border, header and
body in one region) plus page/grid tinting and gap controls, exposed as new
per-job keys. Colors accept hex (`#f2f2f2`), `luma(95%)`, or named Typst colors;
lengths accept `<number><unit>`. All keys are validated, falling back to defaults
on bad input rather than emitting invalid Typst. Defaults preserve the previous
rendering exactly, so existing jobs are unchanged until they opt in.

## Implementation Details

New `LayoutConfig` fields (and matching `JobConfig` TOML keys):

- `page_fill` — page background color (`#set page(fill: …)`); default white.
- `empty_grid_fill` — empty grid-cell fill; default built-in `luma(245)`. Set it
  when `page_fill` is tinted so empty cells do not blend into the background.
- `cards` — render description panels as bordered cards vs. the left-bar.
- `card_fill` — card background when `cards`; default `white`.
- `column_gap` — override the `_col-gutter` column gutter.
- `card_gap` — gap between cards; `"column"` (or unset) means "match the column
  gutter". Applied only for `cards`, so the default bar style keeps Typst's
  default block spacing and non-card jobs emit no extra `#let`.

Plumbing:

- `config.rs` — fields, `sanitize_color` / `sanitize_length`, and resolver
  methods (`page_fill_expr`, `empty_grid_fill_expr`, `card_fill_expr`,
  `column_gap_expr`, `panel_gap_expr`).
- `layout_defaults.rs` + `cosam-convert` `convert_jobs` — TOML → config mapping.
- `document.rs` — emits the `page_fill`, re-binds `_col-gutter`, defines
  `_panel-gap`, builds a `PanelStyle`, and threads the empty-grid fill into the
  grid renderer.
- `panels.rs` — `PanelStyle` (card vs. bar branch); folded the constant
  `_desc-secondary-size` font `#let` into a module `const SECONDARY_SIZE`,
  dropping `panel_block` back under the argument-count lint.
- `grid.rs` — `GridRenderConfig.empty_fill` override for empty cells.

`config/layout.toml` enables the new look (`cards = true`,
`page_fill = "luma(95%)"`) on the `desc` and `workshops` jobs.

## Acceptance Criteria

- New keys parse from `[[jobs]]` and map into `LayoutConfig`.
- Colors accept hex / `luma(...)` / named colors; invalid values fall back to the
  default (an injection string is rejected).
- `cards = true` renders bordered cards; default (`cards` unset) renders the
  original left-bar with unchanged spacing.
- `page_fill` tints the page; `empty_grid_fill` keeps grid empties distinct.
- `card_gap = "column"` and unset both equal the column gutter.
- Untouched (non-card) jobs render byte-identically — no extra `#let` is emitted.
- `cargo clippy --workspace` clean; `cargo test --workspace` green.
