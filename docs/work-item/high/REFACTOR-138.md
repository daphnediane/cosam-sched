# REFACTOR-138: Split section/time layout options; reject unknown keywords

## Summary

Separate the layout `split` key into independent `section_split` and
`time_split` options, default time split to none, error on unknown keywords,
and move panel-list geometry constants into `geometry.rs`.

## Status

Open

## Priority

High

## Description

The layout config currently encodes both the entity (section) split and the
time split in one `split` string with seven combined values (`none`, `day`,
`half_day`, `room`, `room_day`, `presenter`, `presenter_day`). `parse_content`
in `apps/cosam-convert/src/main.rs` decodes that single key into a
`SectionSplit` + `TimeSplit`, and silently falls back to defaults for
unrecognized values. Two latent bugs came from this coupling (fixed in the
parent commit): bare `presenter`/`room` wrongly defaulted the time split to
`Day`, splitting guest postcards per day and forcing a two-dimensional banner.

Make the split dimensions explicit and fail loudly on bad input.

## Implementation Details

- **Separate options.** Replace the single `split` field on `JobConfig`
  (`apps/cosam-convert/src/layout_defaults.rs`) with two optional keys:
  - `section_split`: `none` (default), `room`, `presenter`
  - `time_split`: `none` (default), `day`, `half_day`
  - Decide on backward-compat for the old `split = "presenter_day"` form
    (pre-alpha: a clean break is acceptable; otherwise keep `split` as a
    deprecated combined alias that expands to the two new keys).
- **Default time split = none**, not `day`. A grid-bearing content mode
  (`both`, `grid_only`) structurally needs a per-day split — when `time_split`
  is none for those modes, error rather than silently substituting `day`.
- **Reject unknown keywords.** `parse_content`/`parse_*` helpers currently use
  catch-all `_ =>` arms that map unknown strings to a default. Return a
  `Result`/collected error (or at minimum a `warn!`) listing the offending key
  and the valid values, instead of silently degrading.
- **Geometry constants.** *(Done in the parent commit.)* The panel-list layout
  dimensions moved from `crates/schedule-layout/src/blocks/panels.rs` into
  `crates/schedule-layout/src/geometry.rs` (`PL_ACCENT_COL_PT`,
  `PL_COL_GUTTER_PT`, `PL_HOUR_COL_EM`, `PL_ROW_GUTTER_EM`,
  `PL_HEADING_ABOVE_EM`, `PL_HEADING_BELOW_EM`, `COLBREAK_THRESHOLD_PT`),
  emitted as preamble `#let`s; the secondary-size var name moved to
  `fonts.rs::DESC_SECONDARY_SIZE_VAR`.
- Update `config/layout-default.toml` and `config/layout.toml` to the new keys.
- Update the `cosam-convert` `--layout.<key>=<value>` flags and their
  `parse_*` helpers in `apps/cosam-convert/src/main.rs` to expose the two new
  keys instead of the combined `split` (`cosam-layout` was removed in CLI-139).

## Acceptance Criteria

- [ ] `section_split` and `time_split` are independent config keys
- [ ] Time split defaults to none; grid modes error when no time split is given
- [ ] Unknown split/content/paper/footer keywords produce a warning or error,
      not a silent default
- [x] Panel-list geometry constants live in `geometry.rs` (done in parent commit)
- [ ] Sample/default config files updated; `cargo test` green

## Notes

Parent commit (panel-list postcard fixes) already corrected the immediate
symptom by mapping bare `presenter`/`room` to no time split. This item makes
the underlying config model explicit so the coupling cannot reintroduce the
bug.
