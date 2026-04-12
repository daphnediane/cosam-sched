# Refactor EdgeReverseMap to bidirectional EdgeMap

## Summary

Replace `EdgeReverseMap<L, R>` with a generic bidirectional `EdgeMap<L, R>` and migrate all call sites.

## Status

Completed

## Priority

High

## Description

`EdgeReverseMap<L, R>` only indexed by the right endpoint.  A bidirectional
`EdgeMap<L, R>` stores `by_left` and `by_right` indexes simultaneously, giving
O(1) lookup in both directions without ad-hoc duplicated storage.

Changes required:

- Create `schedule/edge_map.rs` with the new `EdgeMap<L, R>` type.
- Re-export from `schedule/mod.rs`.
- Update `EntityStorage` fields: rename `presenters_by_group` →
  `presenter_group_members`; change all five edge fields to `EdgeMap`.
- Migrate all call sites in `entity/panel.rs`, `entity/presenter.rs`,
  `entity/event_room.rs`, `entity/hotel_room.rs`, `entity/panel_type.rs`,
  and `schedule/mod.rs` tests to the new `by_left` / `by_right` / `add` /
  `remove` / `update_by_left` / `update_by_right` API.

## Acceptance Criteria

- `cargo test` passes with no errors.
- `cargo clippy` produces no warnings.
- Old `EdgeReverseMap` is no longer used for any active field (may be kept
  for backward-compat references until fully removed).
