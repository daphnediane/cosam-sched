# REFACTOR-074: Introduce HalfEdgeDescriptor and EdgeKind

## Summary

Split edge fields out of `FieldDescriptor<E>` into a new `HalfEdgeDescriptor` struct; add
`EdgeKind` enum with ownership direction and exclusivity information.

## Status

Completed

## Priority

Medium

## Description

This refactor adds edge field descriptor infrastructure and splits edge fields out of `FieldDescriptor<E>`:

- Rename `field_id()` to `edge_id()`
- Adding `EdgeKind` enum with `Target { source_fields }` and `Owner { target_field, exclusive_with }`
- Adding `HalfEdgeDescriptor` struct for edge fields (replaces `FieldDescriptor<E>` for edges)
- Adding `NamedField::try_as_half_edge()` method to distinguish edge fields
- Removing `target_field` payload from `CrdtFieldType::EdgeOwner` (now in `EdgeKind`)
- Moving `exclusive_with` from macro closures into `EdgeKind::Owner`
- Updating `FieldSet<E>` to store `FieldDescriptor<E>` and `HalfEdgeDescriptor` in separate vectors
- Switching edge field statics from `FieldDescriptor<E>` to `HalfEdgeDescriptor`

## Implementation Details

- Added `EdgeKind` enum with `Owner { target_field, exclusive_with }` and `Target { source_fields }` variants
- Added `HalfEdgeDescriptor` struct for edge fields (not generic over entity type)
- Added `NamedField::try_as_half_edge()` method to distinguish edge fields from regular fields
- Removed `EdgeOwner` and `EdgeTarget` variants from `CrdtFieldType` (all edge fields now use `Derived`)
- Added derives (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`) to `CrdtFieldType`
- Removed the `HalfEdge` trait entirely, replacing trait object abstractions with concrete `HalfEdgeDescriptor`
- Updated `EdgeKind` enum fields to use `&HalfEdgeDescriptor` instead of `&dyn HalfEdge`
- Updated `FullEdge` struct fields to use `&HalfEdgeDescriptor` instead of `&dyn HalfEdge`
- Removed unsafe transmutes in `field/set.rs` that cast `&HalfEdgeDescriptor` to `&dyn HalfEdge`
- Updated `FieldSet` to store `FieldDescriptor<E>` and `HalfEdgeDescriptor` in separate vectors
- Updated schedule/edge.rs function parameters to use `&HalfEdgeDescriptor`
- Migrated all field descriptors to new macro system (accessor_field_properties!, edge_field_properties!, callback_field_properties!)
- Removed define_field! macro entirely
- Moved crdt_type into CommonFieldData
- Added global registry module with O(1) lookups
- Reorganized schedule-core modules into logical subdirectory structure
