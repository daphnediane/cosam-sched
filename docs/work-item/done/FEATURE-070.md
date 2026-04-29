# FEATURE-070: Eliminate EdgeDescriptor; fold target_field into FieldDescriptor

## Summary

Remove the separate `EdgeDescriptor` struct and inventory; encode CRDT-edge ownership and target field directly inside `CrdtFieldType::EdgeOwner` on the owner field.

**Note:** This work item was later superseded by REFACTOR-074, which reintroduced
`EdgeDescriptor` as a separate struct with `EdgeKind` encoding ownership direction.
The `EdgeOwner`/`EdgeTarget` variants in `CrdtFieldType` were removed; all edge
fields now use `CrdtFieldType::Derived`, and ownership is encoded in `EdgeKind`.

## Status

Completed

## Priority

High

## Description

Each edge relationship was described in two places:

1. The owner field's `CrdtFieldType::EdgeOwner(&'static EdgeDescriptor)`.
2. A separate `pub(crate) static EDGE_<NAME>: EdgeDescriptor = …;` plus its own `inventory::submit!` registration.

`EdgeDescriptor` carried `name` (debug only), `owner_field` (self-reference, redundant), `target_field` (the only piece not already on the owner), and `fields: &[EdgeFieldSpec]` (per-edge metadata, slated for removal by FEATURE-065). Collapsing the two leaves a simpler model where the owner field is the edge descriptor.

## Implementation Details

- Change `CrdtFieldType::EdgeOwner` from `EdgeOwner(&'static EdgeDescriptor)` to `EdgeOwner { target_field: &'static dyn NamedField }`.
- Add `crdt_type()` accessor to `NamedField` so callers can iterate `CollectedNamedField` and filter owner fields.
- Manual `Debug` impl for `CrdtFieldType` (since `dyn NamedField` doesn't implement `Debug`).
- Rewrite `edge_crdt::canonical_owner` and `canonical_owner_by_types` to iterate `all_named_fields()` filtered by `EdgeOwner { .. }` instead of `all_edge_descriptors()`.
- Replace `CanonicalOwner.descriptor` with direct `owner_field` / `target_field` fields plus method accessors (`owner_type()`, `target_type()`, `field_name()`).
- Rewrite `Schedule::rebuild_edges_from_doc` to iterate owner fields from `all_named_fields()`.
- Update `Schedule::edge_get_bool` / `edge_set_bool` to use the new `CanonicalOwner` shape; hardcode the `credited` default to `true` until FEATURE-065 retires both functions.
- Replace `edge_field!`'s `edge: &EDGE_X,` parameter with a bare `owner,` flag.
- Delete `crates/schedule-core/src/edge_descriptor.rs`, `EdgeFieldSpec`, `EdgeFieldDefault`, `CollectedEdge` inventory, and all `EDGE_<NAME>: EdgeDescriptor` statics in entity files.

## Acceptance Criteria

- `cargo test` passes with no behavior changes (463 tests).
- `cargo clippy --all-targets -- -D warnings` clean.
- `cargo fmt` clean.
- `edge_descriptor.rs` deleted along with all `EDGE_*` statics in entity modules.

## Notes

- Follow-up: a proc-macro crate (`schedule-macro`) could replace the declarative `macro_rules!` and add an `exclusive_with:` clause for cross-partition exclusivity — that's needed to cleanly redo FEATURE-065 (credited/uncredited presenter split). Tracked as a separate work item.
- The half-edge model — each field declares its own direction; no auto-pairing of inverse fields — is preserved.
