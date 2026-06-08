# REFACTOR-052: Inventory field registration infrastructure

## Summary

Add `CollectedField<E>`, `RegisteredEntityType`, `order` field on `FieldDescriptor`,
`FieldSet::from_inventory`, and update field macros to self-submit via inventory.

## Status

Completed

## Priority

High

## Description

Foundational infrastructure for migrating field and entity type registration from
manual `FieldSet::new(&[...])` lists to inventory-based self-registration.

## Implementation Details

- Add `order: u32` to `FieldDescriptor` struct in `field.rs`
- Add `CollectedField<E: EntityType>` wrapper struct and `inventory::collect!` to `entity.rs`
- Add `RegisteredEntityType` struct and `inventory::collect!(RegisteredEntityType)` to `entity.rs`
- Add `FieldSet::from_inventory()` constructor to `field_set.rs` (sorts by `order`)
- Update all field macros in `field_macros.rs` to accept `order:` parameter and emit
  `inventory::submit! { CollectedField::<$entity>(&$static_name) }`
- Update `field_set.rs` test mock fields to include `order:` values
- Keep `FieldSet::new()` public for tests

## Follow-up

In REFACTOR-066 (FieldId conversions refactor), `CollectedField<E>` was removed
and replaced with a single global `CollectedNamedField` registry. Field descriptors
now submit `&'static dyn NamedField` to the global registry, and `FieldSet::from_inventory`
filters by entity type name and uses `std::any::Any::downcast_ref` for type-safe
conversion to `&'static FieldDescriptor<E>`.
