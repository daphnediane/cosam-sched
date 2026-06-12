# FEATURE-146: Make Uniq ID prefix authoritative for type; reassign on type change

## Summary

Drop the `Kind`/`Panel Types` type columns from export and make a row's type
come solely from its Uniq ID prefix, reassigning the code (with an old-id
history) when an entity's type no longer matches its prefix.

## Status

In Progress

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

## Design (chosen approach)

A panel-like entity's type comes **directly from its Uniq ID prefix** — no
intermediate entity. The prefix (`GP` in `GP001`) selects the matching
`PanelType` via a prefix lookup; the part/session/suffix never affect the type.

- **No `PanelCode` entity.** An earlier iteration introduced a first-class
  `PanelCode` entity (one per `prefix`+`number`) to "own" the type relationship,
  with managed `panel_code` edges. It was dropped: the type depends only on the
  prefix, so the (prefix+number) granularity bought nothing for type derivation —
  it only matters for the *deferred* renumber, which can build a lightweight
  in-memory base index when it actually needs one. Forcing the concept into the
  UUID/entity/edge/CRDT system created persistence, rehydrate-ordering, import
  seen-tracking, and edge-mirroring friction for what is purely derived state.
- **`CodeHistory` value type** (in `tables/fields/code.rs`) ties the current code
  and its history together: the current code is stored decoded
  (`Option<PanelUniqId>`, read constantly) and the history as canonical id
  strings (decoded on demand). It enforces the stack invariants (no code
  simultaneously live and historical; `set_current` pushes the prior current into
  history and **reclaims** a returning code). Surfaced through one `HasCode`
  capability; the `code` and `old_codes` fields both delegate to it. `old_codes`
  supports set / add / remove — adding the current code is ignored.
- **Panel-like entities** (Panel, Break, Timeline) store their own full
  `PanelUniqId` plus the `old_codes` history (the `Old Uniq Id` column). The
  `code` write is a plain data mutation (`WriteFn::Bare`): parse → `set_current`.
- **`panel_type` on panel-like** becomes a **derived, read-only** field resolving
  `PanelType::find_by_prefix(code.type_prefix())`; the direct
  `panel_like → panel_type` edge is removed.
- **`old_codes` is history, not a reservation:** entries never block another
  entity from being assigned that code; allocation ignores `old_codes`.

## Implementation phases

1. **[DONE] `CodeHistory` + history field.** Merged the current `code` and
   `old_codes` history into one `CodeHistory` value type + `HasCode` capability in
   `tables/fields/code.rs`; added the `old_codes` field (CRDT list, set/add/remove)
   to Panel, Break, Timeline. (Superseded the earlier `PanelCode` entity +
   `panel_code` edge work, now removed.)
2. **[DONE] Derived `panel_type`.** Removed the direct `panel_like → panel_type`
   owner edges (and the reverse `panels`/`timelines`/`breaks` edges on
   `PanelType`). Added `PanelTypeEntityType::find_by_code` (prefix lookup);
   repointed every reader (workshop flag, xlsx export Kind/Panel-Types columns,
   grid type lookup) through it, and dropped the edge-set calls from xlsx and
   widget import. `Kind`/`Panel Types` remain accepted import columns but no
   longer override the prefix per row.
3. **[TODO] Drop export columns + `Old Uniq Id` round-trip.** Stop writing
   `Kind`/`Panel Types` on export (still imported as fallbacks); write the
   `Old Uniq Id` history column. **xlsx read/write currently ignore `Old Uniq Id`**
   — read should populate `old_codes`, write should emit the column.

## Acceptance Criteria

- A panel/timeline/break's panel type is determined solely by its Uniq ID prefix;
  no direct panel-type edge remains.
- Writing a new `code` records the prior code in `old_codes` and reclaims a
  returning code.
- `old_codes` does **not** block another entity from using that code.
- `Kind`/`Panel Types` are no longer written on export but still import as
  fallbacks; `Old Uniq Id` round-trips through xlsx.
- `cargo test --workspace` and `cargo clippy --workspace` green.

## Deferred (follow-up)

**Reassign-on-type-change / renumber.** The feature's namesake but its heavier
half — deferred until the derived `panel_type` + column work lands. When an
entity's type is changed so its prefix no longer matches its code (or a legacy
row was typed via `Kind`), the code is reassigned to a prefix-matching one.

- **Trigger:** reassign when the user changes an entity's type, with an
  export-time safety net that reassigns any remaining prefix/type mismatch.
- **Reclaim** (already implemented in `CodeHistory::set_current`): if the target
  prefix matches one of *this* entity's own `old_codes`, restore that id instead
  of allocating a new number — a clean round-trip when toggling a type back.
- **Allocation** (when no reclaimable old code exists): pick a free number under
  the target prefix, avoiding all *live* codes. Allocation does **not** avoid
  other entities' `old_codes` — `old_codes` is history, not a reservation. (This
  reverses the original FEATURE-146 draft, which reserved every historical id.)
- **Group consistency:** parts/sessions sharing a base (`<prefix><num>` differing
  only in `P`/`S`/suffix) must all receive the **same** new number, keeping the
  group intact. This is where the lightweight `(prefix, number)`-keyed in-memory
  base index would be built — to find and re-point every member of a base.

## Notes

Spun out of FEATURE-144 (breaks as a first-class entity). The `Old Uniq Id`
column already exists on the Schedule sheet (currently written blank).
This item went through two superseded designs: an `old_codes`-only field, then a
first-class `PanelCode` entity. Both were dropped in favor of deriving the type
directly from the prefix — `PanelCode` was UUID/entity machinery for what is
purely derived state, and users never see it.

**Open design question** (from the original draft, still open): whether
reassignment should also run as a model-normalization pass on load, or only on
type-change plus the export-time safety net.
