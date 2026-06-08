# REFACTOR-058: Credited vs Uncredited Presenter Handling

## Summary

Update `FIELD_CREDITS` to use the per-edge `credited` flag introduced by
REFACTOR-060, so individual presenters can be excluded from credit display.

## Status

Completed

## Priority

Medium

## Previously Blocked By

- REFACTOR-060: Edge metadata infrastructure — `EdgeDescriptor.fields` + `credited` flag (Done)

## Description

The `credits` computed field (`FIELD_CREDITS`) currently treats all presenters
attached to a panel as credited. The v9 system distinguished between credited
and uncredited presenters (e.g., moderators, tech staff, guests who requested
anonymity) using separate `credited_presenters` vs `all_presenters` lists.

REFACTOR-060 added `credited: bool` per-edge metadata and the
`credited_presenters` / `uncredited_presenters` / `add_credited_presenters` /
`add_uncredited_presenters` field API. `FIELD_CREDITS` now filters by the flag.
This item covers any remaining integration work and documentation.

## Chosen Approach

Edge metadata (Option 2 from original investigation): REFACTOR-060 adds a
`credited: bool` per-edge field on the Panel ↔ Presenter relationship, stored
in a parallel `presenters_meta` automerge map. Default is `true` (all existing
presenters remain credited with no data migration).

This work item covers updating `FIELD_CREDITS` to read the `credited` flag via
`Schedule::edge_get_bool` and exclude uncredited presenters from credit
resolution. The infrastructure is implemented in REFACTOR-060.

## Acceptance Criteria

- [x] `FIELD_CREDITS` reads `credited` flag via `Schedule::edge_get_bool` and
  excludes uncredited presenters from the credit string output
- [x] Group expansion logic also excludes uncredited presenters
- [x] Tests cover credited vs uncredited presenter handling
- [x] Document approach in `architecture.md`

## Notes

Blocked on REFACTOR-060. Once REFACTOR-060 is complete, the `FIELD_CREDITS`
update is already included in that item's scope — this work item may be
marked as Completed at the same time.

**Completion Note:** The implementation evolved beyond the original plan.
FEATURE-065 replaced the per-edge `credited` boolean approach with a cleaner
partitioned edge model (`EDGE_CREDITED_PRESENTERS` / `EDGE_UNCREDITED_PRESENTERS`),
eliminating the `credited` per-edge boolean and its CRDT storage. The current
implementation in `compute_credits()` uses `EDGE_CREDITED_PRESENTERS` and properly
excludes uncredited presenters. Tests verify the functionality, and the approach
is documented in `architecture.md` and `field-system.md`.
