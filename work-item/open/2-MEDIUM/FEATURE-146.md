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
2. **[TODO] Derived `panel_type`.** Replace the direct `panel_like → panel_type`
   edge with a derived read resolving `PanelType` by the code's prefix.
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

- **Renumber-on-set:** renumbering a base (e.g. `GP001` → `GP002`) re-points every
  entity sharing that base, prefers prior forms, and may consult `old_codes`. This
  is where a lightweight `(prefix, number)`-keyed in-memory index would be built.

## Notes

Spun out of FEATURE-144 (breaks as a first-class entity). The `Old Uniq Id`
column already exists on the Schedule sheet (currently written blank).
This item went through two superseded designs: an `old_codes`-only field, then a
first-class `PanelCode` entity. Both were dropped in favor of deriving the type
directly from the prefix — `PanelCode` was UUID/entity machinery for what is
purely derived state, and users never see it.
