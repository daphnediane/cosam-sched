# FEATURE-146: Make Uniq ID prefix authoritative for type; reassign on type change

## Summary

Drop the `Kind`/`Panel Types` type columns from export and make a row's type
come solely from its Uniq ID prefix, reassigning the code (with an old-id
history) when an entity's type no longer matches its prefix.

## Status

Open

## Priority

Medium

## Blocked By (optional)

## Description

After FEATURE-144, breaks/timelines/panels all derive their type from the Uniq
ID prefix on import (prefix → panel type). The `Kind` column (Schedule sheet)
and `Panel Types` column (Timeline/Break) are now redundant *fallbacks*. We want
the prefix to be the single source of truth for type, and to drop those columns
from export.

To do that safely, whenever an entity's associated panel type stops matching its
Uniq ID prefix (e.g. the type edge is changed, or a legacy row was typed via
`Kind`), the entity's code must be **reassigned** so its prefix matches the new
type — preserving the previous code(s) in an `Old Uniq Id` history.

Applies to **Panel, Timeline, and Break**.

## Implementation Details

- **Trigger:** reassign when the panel-type edge changes (preferred), with an
  export-time safety net that reassigns any remaining prefix/type mismatch.
- **Old Uniq Id is a list (history), not a single value.** Per entity, it
  records every previously-assigned code. Two purposes:
  1. **Reserve** — every id in any entity's `Old Uniq Id` list is treated as
     *used*, so reassignment never hands it to a different entity.
  2. **Reclaim** — when an entity switches to a type whose prefix matches one of
     *its own* `Old Uniq Id` entries, and that id isn't currently the live code
     of another entity, restore that old id instead of allocating a new number
     (clean round-trip when toggling a type back and forth).
- **Number allocation** (when no reclaimable old id exists): pick a free number
  under the target prefix, avoiding all live codes *and* all `Old Uniq Id`
  entries across every entity.
- **Group consistency:** parts/sessions sharing a base id (`<prefix><num>` with
  differing `P`/`S`) must all receive the **same** new number, keeping the group
  intact; suffixes/part/session components are preserved.
- **Schema:** add a persisted `old_codes` list field to Panel, Timeline, and
  Break (CRDT field + serialization). Export writes the current code → `Uniq ID`
  and the history → `Old Uniq Id`; import reads `Old Uniq Id` back into the list
  (and reserves those ids).
- **Drop type columns on export:** remove `Kind` from the Schedule sheet write
  and `Panel Types` from the Timeline/Break writes (keep both accepted on import
  as `READ_ONLY` fallbacks for legacy/hand-authored sheets).

## Acceptance Criteria

- A panel/timeline/break whose type doesn't match its prefix is reassigned a
  prefix-matching code; the old code is recorded in `Old Uniq Id` and never
  reused by another entity.
- Switching a type back reclaims the prior code when free.
- Parts/sessions of one base stay grouped under a shared new number.
- `Kind`/`Panel Types` are no longer written on export but still import.
- `cargo test --workspace` and `cargo clippy --workspace` green.

## Notes

Spun out of FEATURE-144 (breaks as a first-class entity). The `Old Uniq Id`
column already exists on the Schedule sheet (currently written blank).

Open design question: whether reassignment should also re-run as a model
normalization pass on load, or only on edge-change + export.
