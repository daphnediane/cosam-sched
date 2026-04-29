# REFACTOR-074: Introduce EdgeDescriptor and HalfEdge trait hierarchy

## Summary

Split edge fields out of `FieldDescriptor<E>` into a new `EdgeDescriptor<E>` struct; add
`HalfEdge`, `TypedField<E>`, and `TypedHalfEdge<E>` traits so that `FieldNodeId` can only be
constructed from edge fields.

## Status

In Progress

## Priority

Medium

## Description

Currently `FieldNodeId<E>` holds `&'static FieldDescriptor<E>`, which allows any field (scalar,
text, derived, etc.) to be used as a field node ID. This refactor enforces that only half-edge
fields can appear in `FieldNodeId` by:

- Rename `field_id()` to `edge_id()`
- Adding `HalfEdge : NamedField` trait with `edge_id()` and `edge_kind() -> &EdgeKind`
- Adding `EdgeKind` enum with `Target { source_fields }` and `Owner { target_field, exclusive_with }`
- Adding `EdgeDescriptor<E>` — a unified struct for all edge fields (owner and target)
- Adding `TypedField<E>` blanket supertrait over `ReadableField + WritableField + VerifiableField`
- Adding `TypedHalfEdge<E>` blanket over `HalfEdge + TypedField<E>`; stored in `FieldNodeId<E>`
- Rename `field_node_id::FieldRef` to `EdgeRef`
- Changing `EdgeRef` to hold `&'static dyn HalfEdge` (was `&'static dyn NamedField`)
- Removing `target_field` payload from `CrdtFieldType::EdgeOwner` (now in `EdgeKind`)
- Moving `exclusive_with` from macro closures into `EdgeKind::Owner`
- Updating `FieldSet<E>` to hold `dyn TypedField<E>`
- Updating the `define_field!` macro to emit `EdgeDescriptor` for edge fields

## Implementation Details

Two-phase implementation:

**Phase A (additive):** Add `EdgeKind`, new traits, `EdgeDescriptor<E>`, update `CrdtFieldType`
to remove `EdgeOwner`/`EdgeTarget` (all edges now `Derived`), update `FieldSet`. Commit as standalone.

**Phase B (breaking):** Change `EdgeRef` inner type, update `FieldNodeId`/`RuntimeFieldNodeId`,
update macro, update entity statics, update schedule.rs, update tests. One commit.

**Phase A completed:**

- Added `EdgeKind` enum with `Owner { target_field, exclusive_with }` and `Target { source_fields }`
- Added `HalfEdge` trait extending `NamedField` with `edge_kind()` and `edge_id()`
- Added `EdgeDescriptor<E>` struct for edge fields
- Removed `EdgeOwner` and `EdgeTarget` variants from `CrdtFieldType` (all edge fields now use `Derived`)
- Updated `define_field!` macro to emit `EdgeDescriptor` for edge fields
- Added derives (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`) to `CrdtFieldType`

**Phase B:** In Progress
