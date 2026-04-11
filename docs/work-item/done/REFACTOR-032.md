# Rename DirectedEdge from/to endpoints to left/right

## Summary

Rename the `from`/`to` endpoint naming on `DirectedEdge` to `left`/`right`
throughout the codebase.

## Status

Completed

## Priority

High

## Blocked By

- FEATURE-008: edge uniqueness policies (in progress — shares the same files)

## Description

The `DirectedEdge` trait methods `from_id()` / `from_uuid()` take `&self` but
are named `from_*`, which conflicts with Rust's convention that `from_*`
methods are infallible constructors (the `From` trait family).  Clippy
correctly flags this as `methods called from_* usually take no self`.

More fundamentally, the names carry false directional semantics.  A
`PanelToPresenter` edge does not *flow* from Panel to Presenter; "Panel hosted
by Presenter" is equally valid as "Presenter hosts Panel".  The two sides are
positionally distinguishable (`left` vs `right`) but not semantically ordered.

### Changes required

- **`entity/mod.rs`** — `DirectedEdge` trait:
  - `type FromId` → `type LeftId`
  - `type ToId` → `type RightId`
  - `fn from_id()` → `fn left_id()`
  - `fn to_id()` → `fn right_id()`
  - `fn from_uuid()` → `fn left_uuid()`
  - `fn to_uuid()` → `fn right_uuid()`
  - `fn is_self_loop()` — unchanged (uses `left_uuid == right_uuid`)

- **`schedule-macro`** — codegen for `impl DirectedEdge`:
  - Keep `#[edge_from]` / `#[edge_to]` macro attributes as-is (they are
    positional markers, not user-facing method names; changing them is a
    separate concern)
  - Generated `impl DirectedEdge` block: rename `from_id`/`to_id` method
    bodies and associated type assignments

- **All 5 edge entity files** — generated `impl DirectedEdge` calls:
  - `panel_to_presenter.rs`, `panel_to_event_room.rs`,
    `panel_to_panel_type.rs`, `event_room_to_hotel_room.rs`,
    `presenter_to_group.rs`
  - Struct fields `panel_uuid`/`presenter_uuid` etc. are fine — those are
    domain names, not the `from`/`to` generic endpoints

- **`schedule/mod.rs`** — all `.from_uuid()` / `.to_uuid()` call sites
  → `.left_uuid()` / `.right_uuid()`

- **`schedule/storage.rs`** — doc comment references

- **`docs/field-system.md`** and **`docs/system-analysis.md`** — update
  trait documentation tables

### What does NOT change

- The trait name `DirectedEdge` — "directed" correctly means the two sides
  are not interchangeable, even in a many-to-many relationship
- `EdgeIndex::outgoing()` / `incoming()` — these describe index direction,
  not edge semantics
- Edge entity type names (`PanelToPresenter`, etc.) — `To` is English
  preposition, not a Rust conversion pattern
- `#[edge_from]` / `#[edge_to]` macro attributes — separate concern,
  can be renamed to `#[edge_left]` / `#[edge_right]` in a follow-up

## Acceptance Criteria

- `cargo clippy -- -D warnings` passes with no `from_*` method lint
- All edge entity files use `left_id()`/`right_id()`/`left_uuid()`/`right_uuid()`
- `EdgeIndex` queries (outgoing/incoming) still work correctly
- All existing tests pass unchanged
- Documentation updated
