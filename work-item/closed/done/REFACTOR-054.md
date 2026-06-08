# REFACTOR-054: Add EntityType registry via inventory

## Summary

Register all entity types via `inventory::submit!` into a central `RegisteredEntityType`
collection, and expose a `registered_entity_types()` accessor.

## Status

Completed

## Priority

Medium

## Blocked By

- REFACTOR-052: Inventory field registration infrastructure

## Description

Adds a runtime-discoverable registry of all entity types, enabling generic tooling
(editor, converter) to enumerate entity types without a hard-coded list.

## Implementation Details

- Add `inventory::submit! { RegisteredEntityType { ... } }` in each entity type impl
  block (Panel, Presenter, EventRoom, HotelRoom, PanelType)
- Add a `pub fn registered_entity_types() -> impl Iterator<Item = &'static RegisteredEntityType>`
  free function in `entity.rs`
- Add integration test asserting all 5 type names are present in the registry
