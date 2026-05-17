# BUGFIX-123: Update-mode import does not correct presenter name capitalisation

## Summary

When re-importing an XLSX over an existing schedule, a presenter whose name
differs only in case (e.g. `"camelcase"` → `"CamelCase"`) is matched correctly
but the stored name is never updated to match the xlsx spelling.

## Status

Open

## Priority

Medium

## Description

`find_or_create_presenter_by_name` uses a case-insensitive lookup (`eq_ignore_ascii_case`)
to find an existing presenter, but on a hit it only potentially upgrades the rank —
it does not update the stored `name` field.  As a result, if the existing schedule
stores `"camelcase"` and the xlsx spells it `"CamelCase"`, the name stays wrong
after an update import.

## How Found

Integration test `test_update_presenter_name_capitalization_corrected` added
as part of update-mode import work (FEATURE-122).

## Reproduction

1. Import an xlsx with presenter `"camelcase"`.
2. Re-import (update mode) an xlsx where the same presenter is spelled `"CamelCase"`.
3. Observe the stored name is still `"camelcase"`.

**Expected:** stored name becomes `"CamelCase"` (xlsx is source of truth on update).

**Actual:** stored name remains `"camelcase"`.

## Steps to Fix

In `find_or_create_presenter_by_name` (`crates/schedule-core/src/tables/presenter.rs`),
after finding the existing entity, add:

```rust
d.data.name = name.to_string();
```

before or alongside the rank-upgrade logic.

## Testing

`test_update_presenter_name_capitalization_corrected` in `tests/xlsx_integration.rs`
covers this case and currently fails.
