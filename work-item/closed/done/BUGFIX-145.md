# BUGFIX-145: Import drops panels with "invalid" or `*`-marked Uniq IDs

## Summary

Two import paths silently drop rows that should be kept. (1) A non-blank Uniq ID
that doesn't match the strict grammar (typos, hyphens, numberless codes) makes
`FIELD_CODE`'s write error, so the whole upsert fails and the row vanishes.
(2) A leading `*` on the Uniq ID is treated as a soft-delete and skipped. Per
design intent there are no required fields, so such rows must import — the
`*` form as an *unscheduled* panel.

## Status

Completed

## Priority

High

## Blocked By (optional)

## Description

`PanelUniqId::parse` returns `None` for anything that isn't
`PREFIX(letters) + NUM(digits) + alnum-suffix`. The `code` field's write
callback turns that `None` into an `Err`
(`crates/schedule-core/src/tables/panel/mod.rs` `FIELD_CODE`), and
`find_or_create_entity` propagates it, so the import loop hits its error arm
and `continue`s past the row with only an `eprintln`
(`crates/schedule-core/src/xlsx/read/schedule.rs`, timeline.rs likewise).

Net effect: a coordinator typo (`GW19#`), a uniquifying suffix (`GP001-01`), or
a numberless marker (`BREAK`) **drops the panel entirely**. This violates the
design principle recorded in FEATURE-043 (line 50): *"No required fields — a
panel without code or name is just soft-deleted/unscheduled."* The code field
is not required; an unparseable value should be preserved best-effort, never
fatal.

Spun out of the BUGFIX-131 discussion (raw-prefix preservation), which fixed
truncation but left the strict-grammar drop in place.

## How Found

Tracing the import path while discussing BUGFIX-131: `FIELD_CODE`'s write errors
on `parse() == None`, and `find_or_create_entity`'s error arm skips the row.

## Reproduction

1. Add a Schedule row with a non-empty but unparseable Uniq ID (e.g. `GP001-01`
   or `GW19#`) and a Name.
2. Import the workbook.

**Expected:** the panel imports with its code preserved (soft/unscheduled at
worst), per "no required fields".

**Actual:** the row is dropped; only an `eprintln` on stderr.

## Steps to Fix

Make `PanelUniqId::parse` total for any non-blank (whitespace-trimmed) input —
it returns `None` only for empty/whitespace. Best-effort decomposition:

- `prefix` = leading run of ASCII letters (may be empty), uppercased.
  `type_prefix()` is still its first two chars.
- `prefix_num` = the following run of digits, or `0` if absent.
- remainder = suffix, after pulling `P<n>`/`S<n>` part/session tags as today;
  the leftover is preserved verbatim (now allowing non-alpha, e.g. `-01`).

Examples (from the spec discussion):

- `"123A"` → prefix `""`, num `123`, suffix `"A"` → `full_id() == "123A"`
- `"BREAK"` → prefix `"BREAK"` (type_prefix `"BR"`), num `0`, suffix `""`
  → `full_id() == "BREAK000"`
- `"GP001-01"` → prefix `"GP"`, num `1`, suffix `"-01"` → round-trips

Because `parse` returns `Some` for non-blank input, `FIELD_CODE` no longer
errors and the panel always imports. (The `Err` arm in the write callback
becomes effectively unreachable for non-blank input; keep it as a guard.)

### Asterisk = unscheduled (not soft-delete)

A `*` anywhere on the Uniq ID now marks the row *unscheduled* instead of
deleting it: the `*`s are stripped, the row imports, and its start time/time is
forced empty (so it sorts last, like a blank-Start-Time unschedule). Deletion is
done by removing the row from the sheet (already soft-deleted as "not seen on
re-import"). Applied symmetrically in `xlsx/read/schedule.rs` and
`xlsx/read/timeline.rs`. This reverses the previous leading-`*` soft-delete
convention (only ever in code + one test, never documented).

## Testing

- `parse_invalid_returns_none`: only `""` / whitespace stays `None`; `"123"`,
  `"INVALID"`, `"GP001-1"` now parse. Add round-trip asserts for the examples
  above plus `type_prefix()` checks.
- Integration: a Schedule row with a typo'd Uniq ID imports as a Panel with the
  code preserved (not dropped); re-import stays idempotent.

## Notes

The same principle implies a nameless row should also soft-delete/unschedule
rather than be skipped (`schedule.rs` currently `continue`s on missing Name).
Out of scope here; capture separately if desired. Asterisk-as-unscheduled is a
related but distinct import-behavior change (see the `*` handling in
`schedule.rs`/`timeline.rs`).
