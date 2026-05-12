# REFACTOR-112: Update ignored set_neighbors tests to current RawEdgeMap API

## Summary

Update the `#[ignore]`d `set_neighbors` tests in `schedule-core/src/edge/map.rs`
to compile and pass against the current `RawEdgeMap` API.

## Status

Open

## Priority

Low

## Description

The test `test_set_neighbors_replaces_and_patches_reverse` (and any related
`set_neighbors` tests) in `crates/schedule-core/src/edge/map.rs` are marked
`#[ignore]` with a TODO comment because they were written against an older API
and no longer compile or reflect the current `RawEdgeMap` structure (which uses
a `HashMap<NonNilUuid, HashMap<FieldId, Vec<FieldNodeId>>>` layout).

## Steps to Fix

1. Review what `set_neighbors` currently does in `RawEdgeMap`
2. Rewrite the test body to use the current `add_edge` / `set_neighbors` API
   and `FieldNodeId`-based accessors
3. Remove `#[ignore]` and the TODO comment
4. Confirm `cargo test -p schedule-core` passes with no ignored tests

## Acceptance Criteria

- `test_set_neighbors_replaces_and_patches_reverse` passes without `#[ignore]`
- No TODO comment remains in `map.rs` referencing this work item
