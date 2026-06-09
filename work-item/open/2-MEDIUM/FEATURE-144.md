# FEATURE-144: Pull breaks into their own table (parallel to Timeline)

## Summary

Model convention-wide breaks as a first-class `Break` entity (like `Timeline`),
carrying duration, instead of `Panel` entities flagged `is_break`.

## Status

Open

## Priority

Medium

## Blocked By (optional)

## Description

Breaks are currently regular `Panel` entities whose panel type has
`is_break: true`, assigned to a pseudo `BREAK` room and excluded from the public
room list. They are not panels in any real sense, but they carry more data than
a `Timeline` (which is a single time point) — notably a **duration** (a
`TimeRange`). This item promotes breaks to their own entity/table, paralleling
the existing `Timeline` table (`crates/schedule-core/src/tables/timeline.rs`),
while keeping the import path that extracts them from the main Schedule sheet.

Spun out of BUGFIX-131 (raw-prefix preservation), which removed the immediate
data-loss problem but left breaks structurally tangled with panels.

## Implementation Details

- New `Break` entity type mirroring `Timeline` but with a `TimeRange` (start +
  duration/end) instead of a single `time` point, plus `name`/`description`/
  `note` and an owner edge to its `PanelType`.
- XLSX read: keep extracting break rows from the main **Schedule** sheet
  (detected via the `is_break` panel type), but route them into the `Break`
  table instead of `Panel`. Optionally also accept a dedicated `Breaks` sheet.
- XLSX write: export breaks to their own sheet (mirroring the Timeline sheet).
- Widget JSON (format is changeable — not public): add a dedicated top-level
  `breaks` array paralleling `timeline`. Decide whether the synthesized
  `%IB`/`%NB` implicit/overnight breaks also move into this array or stay
  computed in the widget. Update `widget/cosam-calendar.js` to render from the
  `breaks` array; today it keys break rendering off `panelType.isBreak`.
- Layout (Typst/IDML): update any break handling that assumes breaks are panels.

## Acceptance Criteria

- Breaks round-trip through xlsx import → export with duration preserved.
- Widget renders breaks (banners, overnight moon) from the new `breaks` array;
  my-schedule still excludes them.
- Splits/timeline behavior unchanged.
- `cargo test --workspace` and `cargo clippy --workspace` green.

## Notes

Domain context from the BUGFIX-131 discussion, worth resolving as part of (or
adjacent to) this work — both concern `PanelUniqId::parse`, which currently
requires `PREFIX(letters) + NUM(digits)`:

- **Numberless codes** — historically a bare `"BREAK"`/`"SPLIT"` was a valid
  Uniq ID with `prefix_num = 0`. The current regex requires at least one digit,
  so bare codes fail to parse. If the new `Break` table accepts such codes,
  `parse` may need to treat a missing number as `0`.
- **Uniquifying suffix** — codes like `"GP001-01"` use a trailing `-NN` to
  disambiguate; the current parser rejects the hyphen outright (so the code
  silently fails to parse — a latent BUGFIX-131-style issue). Consider whether
  the suffix grammar should accept a `-NN` uniquifier.

These parser-grammar changes are arguably their own BUGFIX; capture separately
if they grow beyond this feature's scope.
