# BUGFIX-124: Update-mode import does not reset presenter rank to xlsx's highest

## Summary

When re-importing an XLSX over an existing schedule, a presenter whose rank in
the new xlsx is lower than the historically stored rank is not downgraded.
The xlsx should be the source of truth on update.

## Status

Completed

## Priority

Medium

## Description

`find_or_create_presenter_by_name` never downgrades a presenter's rank — it only
upgrades when the new rank has a lower priority number (higher rank).  This is
correct for a single-pass import, but for an update import the xlsx is the
authoritative source.  If the previous import had Alice as Guest and the new xlsx
only lists her as Panelist, her rank should be reset to Panelist.

The desired behaviour: after an update import, each presenter's rank equals the
highest rank they appear with **in the new xlsx** (People sheet + all presenter
columns), even if that is lower than what was stored before.

## How Found

Integration test `test_update_presenter_rank_does_not_exceed_xlsx_highest` added
as part of update-mode import work (FEATURE-122).

## Reproduction

1. Import an xlsx where Alice appears as `"Guest"`.
2. Re-import (update mode) an xlsx where Alice appears only as `"Panelist"`.
3. Observe Alice's rank is still `Guest`.

**Expected:** Alice's rank becomes `Panelist` (xlsx is source of truth on update).

**Actual:** Alice's rank stays `Guest` (rank upgrade-only logic prevents the reset).

## Steps to Fix

In the update-mode import path (`update_schedule_from_xlsx`), before processing
presenter columns and the People sheet, snapshot each presenter's rank and reset
it to the lowest useful rank.  Let the normal upgrade-only logic during import
bring each presenter up to the highest rank seen in the new xlsx.

Alternatively, do a pre-pass that collects (name → max_rank) from the xlsx and
applies them directly, bypassing the upgrade-only gate.

Either approach must avoid resetting ranks for presenters that will be soft-deleted
(i.e. not present in the new xlsx at all).

## Testing

`test_update_presenter_rank_does_not_exceed_xlsx_highest` in
`tests/xlsx_integration.rs` covers this case and currently fails.
