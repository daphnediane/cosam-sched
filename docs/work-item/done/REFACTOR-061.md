# REFACTOR-061: Introduce FieldDescriptorAny, FieldId, and FieldNodeId

## Summary

Add type-erased field identity (`FieldId`) and field-based edge endpoint (`FieldNodeId`) types as
the foundation for the FieldNodeId-based edge system.

## Status

Completed

## Priority

High

## Description

This is Phase 1 of the FieldNodeId edge system refactor. Introduces:

- `FieldDescriptorAny` — object-safe trait on `FieldDescriptor<E>` exposing `field_id()`,
  `entity_type_name()`, and `name()` without a type parameter.
- `FieldId` — newtype over `usize` holding the address of a `&'static FieldDescriptor<E>`;
  provides a stable, type-erased identity for any field singleton.
- `FieldNodeId` — combines a `FieldId` with a `NonNilUuid` to identify "entity X's field Y",
  the new unit of edge endpoint representation.

## Implementation Details

- Add `FieldDescriptorAny` to `field.rs`; implement for `FieldDescriptor<E>`.
- Create `field_node_id.rs` with `FieldId`, `FieldNodeId`, and helper `FieldNodeId::of::<E>()`.
- Re-export from `lib.rs`.
- Full `#[cfg(test)]` coverage: round-trip equality, hash consistency, `FieldId::of` identity.
