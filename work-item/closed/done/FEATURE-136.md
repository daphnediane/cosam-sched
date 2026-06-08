# FEATURE-136: Single-document room signs with flyer-style grid mixing

## Summary

Combine all room signs into one multi-page document and adopt the flyer's
`place`-plus-column-break grid mixing instead of the rigid side-by-side grid.

## Status

Completed

## Priority

Medium

## Description

Room signs previously emitted one Typst document per room per day, each a single
page using a fixed `#grid(columns: (X%, 1fr))` to set the schedule grid beside
this room's descriptions. That rigid split could not let descriptions reflow
past the grid, and produced many separate files.

Room signs now produce a *single* multi-page document (one `(qualifier,
source)` pair with an empty qualifier, like the flyer). Each room/day starts on
a fresh page laid out like the flyer's first page: the full schedule grid is
`place`d over the left half of the columns (this room's column highlighted, day
label in the corner) while the room's descriptions flow through a full-width
column block whose leading column breaks reserve the grid's space and let
overflow continue full-width on following pages. Every page carries a branded
running header (room left, day right) and the flyer's timestamp/page-number
footer.

## Implementation Details

- `blocks/banner.rs`: added `page_header_running_split` (two-slot running header)
  and moved the flyer's `footer_timestamps`/`fmt_stamp` helpers here so both
  formats share them.
- `formats/room_signs.rs`: rewritten to emit one document; iterates rooms ×
  days, emits a `<room-sign>` metadata marker (room + day) per page for the
  running header, places the highlighted grid via `#place` + `box`, and flows
  descriptions through `render_time_grouped_panels` with leading `#colbreak()`s.
  Respects `config.filter.room_uid`.
- `formats/flyer.rs`: now calls the shared `banner::footer_timestamps`.
- `blocks/grid.rs`: removed the now-unused `GridRenderConfig::compact`; room
  signs use `full_page` with a room highlight, matching the flyer.
- `blocks/panels.rs`: removed the now-unused `render_description_blocks`.
- `docs/layout-formats.md`: new document covering all print layout formats, the
  shared building blocks, the grid/column mixing technique, and output
  conventions; linked from `docs/doc-index.md`.

## Acceptance Criteria

- `cosam-convert --export-layout` produces a single `room-signs-<paper>.pdf`
  with one page per room/day (plus overflow pages), not per-room/day files.
- Each page highlights its room's grid column and shows the correct room/day in
  the running header, including on description-overflow pages.
- `cargo clippy` and `cargo test` are clean across the workspace.
