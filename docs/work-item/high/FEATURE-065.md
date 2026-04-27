# FEATURE-065: Split credited_presenters into separate CRDT edge fields

## Summary

Convert `credited_presenters` and `uncredited_presenters` on Panel from computed/derived fields
into actual edge storage fields, eliminating the `credited` per-edge boolean and its CRDT
`presenters_meta` map.

## Status

Open

## Priority

High

## Blocked By

- REFACTOR-064: Updated Schedule edge APIs
- FEATURE-070: Eliminate EdgeDescriptor; fold target_field into FieldDescriptor *(done)*

## Description

Currently Panel stores one CRDT list (`presenters`) plus a parallel `presenters_meta` map with a
`credited` boolean per entry.  `credited_presenters` and `uncredited_presenters` are computed
fields that filter by that boolean, and `add_credited_presenters` / `add_uncredited_presenters`
are write-only helpers that toggle it.

After FEATURE-070, `CrdtFieldType::EdgeOwner` carries `target_field` directly, so adding new
owner-side edge fields is straightforward.  The `EdgeFieldSpec` / `EdgeFieldDefault` schema for
per-edge metadata was already removed by FEATURE-070; only the runtime helpers remain.

The remaining work is to turn the partition into two first-class CRDT lists.

## Implementation Details

### New `Panel` fields

- `FIELD_CREDITED_PRESENTERS` — `EdgeOwner { target_field: &Presenter::FIELD_PANELS }`, rw.
  Custom write fn: replaces the credited list, and for any presenter in the new list also
  removes them from `FIELD_UNCREDITED_PRESENTERS` (cross-partition exclusivity).
- `FIELD_UNCREDITED_PRESENTERS` — symmetric: `EdgeOwner { … }`, rw.  Write fn removes from
  `FIELD_CREDITED_PRESENTERS` for any presenter in the new list.
- `FIELD_PRESENTERS` — kept as a `Derived` read-only union of the two partitions (no CRDT
  storage of its own).  Used by `compute_credits`, `FIELD_INCLUSIVE_PRESENTERS`, and external
  callers that don't care about the partition.
- `FIELD_ADD_CREDITED_PRESENTERS` — write-only `Derived`.  For each presenter in the value:
  remove from uncredited, add to credited.
- `FIELD_ADD_UNCREDITED_PRESENTERS` — symmetric: remove from credited, add to uncredited.
- `FIELD_REMOVE_PRESENTERS` — write-only `Derived`.  Removes each presenter from both
  partitions (no-op if absent).

### `Presenter` side

- `FIELD_PANELS` becomes read-only `EdgeTarget`.  Read fn unions edges via both
  `Panel::FIELD_CREDITED_PRESENTERS` and `Panel::FIELD_UNCREDITED_PRESENTERS`.
- `FIELD_ADD_CREDITED_PANELS` — adds this presenter to `Panel::FIELD_CREDITED_PRESENTERS`,
  removes from uncredited (mirror of `Panel::FIELD_ADD_CREDITED_PRESENTERS`).
- `FIELD_ADD_UNCREDITED_PANELS` — symmetric.
- `FIELD_REMOVE_PANELS` — removes this presenter from both partitions on each named panel.

### Removals

- `EdgeFieldSpec` and `EdgeFieldDefault` are already gone (FEATURE-070).
- `Schedule::edge_get_bool` / `edge_set_bool` — delete (no remaining callers after the split).
- `edge_crdt::meta_field_name` / `read_edge_meta_bool` / `write_edge_meta_bool` — delete.
- `canonical_owner_by_types` — delete (only `edge_get_bool`/`edge_set_bool` use it).
- All `presenters_meta` test fixtures and the `edge_meta_save_load_round_trip` test — delete.

### CRDT schema

Pre-alpha breaking change is acceptable.  Replace `presenters` + `presenters_meta` on each
panel with `credited_presenters` and `uncredited_presenters` lists.  No migration code.

## Acceptance Criteria

- All call sites of `edge_get_bool`/`edge_set_bool` removed; the functions are deleted.
- `presenters_meta` no longer appears in saved documents.
- Tests cover: writing one partition does not affect the other; moving a presenter via the
  `add_*` helpers correctly removes from the opposite partition; `FIELD_PANELS` on Presenter
  unions both directions.
- `cargo test` / `clippy` / `fmt` clean.

## Notes

- Half-edge model: each of the four owner fields (Panel ↔ credited, Panel ↔ uncredited) and
  their inverse `FIELD_PANELS` declares its own direction.  The cross-partition exclusivity
  is implemented by hand-written write closures on `FIELD_CREDITED_PRESENTERS` and
  `FIELD_UNCREDITED_PRESENTERS`; if more cases like this emerge a declarative
  `exclusive_with:` macro clause may be worth adding (see plan notes).
