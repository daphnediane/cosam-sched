# REFACTOR-074: Introduce EdgeDescriptor and HalfEdge trait hierarchy

## Summary

Split edge fields out of `FieldDescriptor<E>` into a new `EdgeDescriptor<E>` struct; add
`HalfEdge`, `TypedField<E>`, and `TypedHalfEdge<E>` traits.

## Status

In Progress

## Priority

Medium

## Description

This refactor adds the edge field trait hierarchy and splits edge fields out of `FieldDescriptor<E>`:

- Rename `field_id()` to `edge_id()`
- Adding `HalfEdge : NamedField` trait with `edge_id()` and `edge_kind() -> &EdgeKind`
- Adding `EdgeKind` enum with `Target { source_fields }` and `Owner { target_field, exclusive_with }`
- Adding `EdgeDescriptor<E>` — a unified struct for all edge fields (owner and target)
- Adding `TypedField<E>` blanket supertrait over `ReadableField + WritableField`
- Adding `TypedHalfEdge<E>` blanket over `HalfEdge + TypedField<E>`
- Removing `target_field` payload from `CrdtFieldType::EdgeOwner` (now in `EdgeKind`)
- Moving `exclusive_with` from macro closures into `EdgeKind::Owner`
- Updating `FieldSet<E>` to hold `dyn TypedField<E>`
- Switching edge field statics from `FieldDescriptor<E>` to `EdgeDescriptor<E>`

## Implementation Details

Two-phase implementation:

**Phase A (additive):** Add `EdgeKind`, new traits, `EdgeDescriptor<E>`, update `CrdtFieldType`
to remove `EdgeOwner`/`EdgeTarget` (all edges now `Derived`), update `FieldSet`. Commit as standalone.

**Phase B (breaking):** Switch edge field statics from `FieldDescriptor<E>` to `EdgeDescriptor<E>`,
update macro to emit `EdgeDescriptor` for edge fields, update `FieldSet` to hold `dyn TypedField<E>`,
update schedule.rs to use `HalfEdge` trait, update tests. One commit.

**Phase A completed:**

- Added `EdgeKind` enum with `Owner { target_field, exclusive_with }` and `Target { source_fields }`
- Added `HalfEdge` trait extending `NamedField` with `edge_kind()` and `edge_id()`
- Added `EdgeDescriptor<E>` struct for edge fields
- Removed `EdgeOwner` and `EdgeTarget` variants from `CrdtFieldType` (all edge fields now use `Derived`)
- Added derives (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`) to `CrdtFieldType`
- Added `TypedField<E>` blanket supertrait
- Added `TypedHalfEdge<E>` blanket trait

**Phase B:** In Progress - need to switch edge field statics from `FieldDescriptor<E>` to `EdgeDescriptor<E>`
