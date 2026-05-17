# FEATURE-127: Idempotent XLSX (and widget JSON) re-import

## Summary

Re-importing the same XLSX or widget JSON into an existing binary schedule
should produce a byte-for-byte identical output when nothing in the source
has changed.

## Status

Open

## Priority

Medium

## Description

`update_schedule_from_xlsx` (and the future `update_schedule_from_widget_json`)
is intended to be used in a pipeline like:

```sh
cosam-modify input.schedule --import-xlsx source.xlsx -o output.schedule
```

When `source.xlsx` is unchanged relative to what was last imported into
`input.schedule`, the output should be bit-for-bit identical to the input —
including `modified_at` metadata.  Currently it is not, for several reasons:

### Root causes

#### 1. CRDT field writes are unconditional

`crdt::write_field` calls `doc.put()` for every field on every entity on every
import, regardless of whether the value changed.  Because automerge commits
accumulate in the document, each re-import adds new history and `doc.save()`
yields different bytes.  The fix is to read the current automerge value before
writing and skip the `doc.put()` when the new value equals the stored one.

Files: `crates/schedule-core/src/crdt/mod.rs` (`write_field`, `write_text`,
`write_list`).

#### 2. Soft-delete / `put_deleted` is unconditional

`schedule.remove_entity` always calls `crdt::put_deleted(…, true)` even if the
entity is already marked deleted.  The fix is to probe the current `__deleted`
flag and skip when already `true`.

Files: `crates/schedule-core/src/schedule/entity.rs` (`remove_entity`),
`crates/schedule-core/src/crdt/mod.rs` (`put_deleted`).

#### 3. `modified_at` is set unconditionally from the source timestamp

`update_schedule_from_xlsx` sets `schedule.metadata.modified_at =
resolve_source_modified(…)` before the import runs, so the metadata changes
even when no entity data changes.  `modified_at` should only be updated if at
least one entity was actually mutated.

Approach: after the import completes, check whether the automerge document
gained any new commits (e.g. compare a pre-import actor/heads snapshot with
the post-import state).  Update `modified_at` only if the document changed.

#### 4. `normalize_presenter_sort_indices` always re-writes all sort indices

This function assigns sort indices to every presenter on every import.  Since
the indices are deterministic from the same data, the values don't change, but
each write still creates an automerge commit (blocked by root cause 1).  Fix
(1) subsumes this.

### `edge_set` is already idempotent

`edge_set` computes `(added, removed)` and only mirrors actual delta to the
CRDT.  Setting the same edge list twice is already a no-op at the automerge
level.

## Acceptance Criteria

- `cosam-modify input.schedule --import-xlsx same.xlsx -o output.schedule` →
  `input.schedule == output.schedule` (byte-for-byte) when no data changed.
- Same guarantee for widget JSON re-import (FEATURE-126).
- If at least one entity changed, `modified_at` is updated and the binary
  output differs from the input.
- No regression in import correctness tests.
