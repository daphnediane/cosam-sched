# REFACTOR-053: Migrate entity types to inventory field registration

## Summary

Replace the manual `FieldSet::new(&[...])` list in each entity type module with
`FieldSet::from_inventory()`, letting fields self-register via `inventory::submit!`.

## Status

Completed

## Priority

High

## Blocked By

- REFACTOR-052: Inventory field registration infrastructure

## Description

For each of the 5 entity type modules, update field declarations to include `order:`
values and remove the manual FieldSet list. One commit per entity type.

## Implementation Details

For each entity type (Panel, Presenter, EventRoom, HotelRoom, PanelType):

1. Add `inventory::collect!(CollectedField<XxxEntityType>)` at top of module
2. Update all field macro invocations to add `order: NNN`
3. Add manual `inventory::submit!` calls for hand-written descriptors (time projections,
   bespoke edge fields) not declared via macros
4. Replace `static XXX_FIELD_SET: LazyLock<FieldSet<XxxEntityType>> = LazyLock::new(|| FieldSet::new(&[...]))` with `FieldSet::from_inventory()`
5. Run `cargo test` to verify `field_set_contains_all_declared_fields` passes

Order values: assign multiples of 100 based on current `FieldSet::new` list position,
leaving gaps for future fields.

## Follow-up

In REFACTOR-066 (FieldId conversions refactor), the per-entity-type `CollectedField<E>`
registries were removed and replaced with a single global `CollectedNamedField` registry.
Entity type modules no longer declare `inventory::collect!(CollectedField<E>)`, and field
macros submit only to the global registry.
