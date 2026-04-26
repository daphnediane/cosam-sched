# REFACTOR-066: Consolidate field registry and FieldId improvements

## Summary

Eliminate per-entity-type `CollectedField<E>` registries, merge `FieldDescriptorAny` into `NamedField`,
and improve `FieldId` conversions with a global registry and type-safe downcasting.

## Status

Completed

## Priority

High

## Blocked By

- REFACTOR-061: FieldDescriptorAny / FieldId / FieldNodeId foundation

## Description

This refactor consolidates field registration infrastructure and simplifies type-erased field access:

**FieldDescriptorAny merger:**

- Removed `FieldDescriptorAny` trait entirely
- Merged its `field_id()` and `entity_type_name()` methods into `NamedField`
- Updated `EdgeDescriptor` to use `&'static dyn NamedField` instead of `&'static dyn FieldDescriptorAny`
- Eliminated the redundant trait layer

**Global field registry:**

- Added single global `CollectedNamedField` registry for all field descriptors
- Removed per-entity-type `CollectedField<E>` registries and `inventory::collect!(CollectedField<E>)` declarations
- Field macros now submit only to the global registry
- `FieldSet::from_inventory()` filters global registry by `entity_type_name()` and uses `std::any::Any::downcast_ref`

**FieldId improvements:**

- Changed `FieldId::of()` from `&'static FieldDescriptor<E>` to `&'static dyn NamedField`
- Added `FieldId::as_named_field()` for round-trip conversion via global registry lookup
- Added `FieldId::try_as_descriptor<E>()` for type-safe conversion using `downcast_ref`
- Added `std::any::Any` supertrait to `NamedField` to enable safe downcasting

This eliminates duplicate registries, reduces trait hierarchy complexity, and uses standard library
type-safe downcasting instead of manual pointer casts.

## Implementation Details

**FieldDescriptorAny removal:**

- Removed `FieldDescriptorAny` trait from `field.rs`
- Added `field_id()` and `entity_type_name()` methods to `NamedField` trait
- Implemented these methods for `FieldDescriptor<E>`
- Updated `EdgeDescriptor` fields from `&'static dyn FieldDescriptorAny` to `&'static dyn NamedField`
- Updated imports in `edge_descriptor.rs`, `edge_cache.rs`, `edge_map.rs`, and tests

**Global registry:**

- Added `CollectedNamedField(pub &'static dyn NamedField)` struct in `field.rs`
- Added `all_named_fields()` iterator function
- Removed `CollectedField<E>` struct from `entity.rs`
- Removed `inventory::collect!(CollectedField<E>)` from panel.rs, presenter.rs, event_room.rs,
  hotel_room.rs, and panel_type.rs
- Updated `stored_field!`, `edge_field!`, and `define_field!` macros to submit only to global registry

**FieldId improvements:**

- Changed `FieldId::of(field: &'static dyn NamedField)` signature
- Added `FieldId::as_named_field()` method with global registry lookup
- Added `FieldId::try_as_descriptor<E>()` using `std::any::Any::downcast_ref`
- Added `std::any::Any` super-trait to `NamedField` trait
- Marked `FieldId::from_raw()` as `pub(crate) unsafe` with safety documentation
- Updated `FieldSet::from_inventory()` to use global registry with `downcast_ref`
- Updated test calls to `FieldId::of()` to remove generic parameter

**Documentation:**

- Updated `field-system.md` to reference `CollectedNamedField` and `downcast_ref`
- Updated `architecture.md` to reflect single global registry approach
- Added follow-up notes to REFACTOR-052.md and REFACTOR-053.md

## Testing

All 469 tests pass. The simplified registry maintains the same functionality with less code
and fewer trait layers.
