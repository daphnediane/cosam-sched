# REFACTOR-067: Add typed `FieldNodeId<E>` and rename `FieldNodeId` to `RuntimeFieldNodeId`

## Summary

Add compile-time typed `FieldNodeId<E>` type similar to `EntityId<E>`, and rename existing `FieldNodeId` to `RuntimeFieldNodeId` for consistency with the entity ID pattern.

## Status

Completed

## Priority

High

## Blocked By

- REFACTOR-066: Consolidate field registry and FieldId improvements

## Description

This refactor introduces a typed `FieldNodeId<E>` type that provides compile-time type safety for field-based edge endpoints, similar to how `EntityId<E>` works for entities. The existing untyped `FieldNodeId` is renamed to `RuntimeFieldNodeId` for consistency with `RuntimeEntityId`.

Key changes:

- Rename `FieldNodeId` to `RuntimeFieldNodeId` (no structural changes)
- Add new typed `FieldNodeId<E>` with `PhantomData<E>` for compile-time type safety
- Entity type information is looked up via `FieldId::as_named_field()` (thanks to REFACTOR-066)
- Add conversion methods between typed and runtime types
- Add macro constructors for ergonomic construction

## Implementation Details

### 1. Rename `FieldNodeId` to `RuntimeFieldNodeId`

- File: `crates/schedule-core/src/field_node_id.rs`
- Rename struct only (no structural changes - already has `field: FieldId` and `entity: NonNilUuid`)
- Add helper method `entity_type_name(&self) -> Option<&'static str>` using `self.field.as_named_field()`
- Update call sites: edge_map.rs, schedule.rs, edge_cache.rs

### 2. Add `FieldNodeId<E>` type

- File: `crates/schedule-core/src/field_node_id.rs`
- Structure with `field: FieldId`, `entity: NonNilUuid`, `_marker: PhantomData<fn() -> E>`
- Implement `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Hash`
- Constructors: `unsafe new`, `from_descriptor`, `from_entity_id`, `from_runtime_entity_id`
- Accessors: `field()`, `entity()`, `entity_id()`

### 3. Add conversion methods

- `FieldNodeId<E> → RuntimeFieldNodeId`: `to_runtime()`
- `RuntimeFieldNodeId → FieldNodeId<E>`: `try_as_typed<E>()` (uses FieldId lookup)
- `RuntimeFieldNodeId ↔ RuntimeEntityId`: conversion methods

### 4. Add macro constructor

- File: `crates/schedule-core/src/field_macros.rs`
- Macro `field_node_id!` with arms for typed and runtime construction

### 5. Update exports and tests

- File: `crates/schedule-core/src/lib.rs`
- Re-export both types
- Add comprehensive tests for conversions and macro

## Notes

- No need to store `entity_type_name` in the struct - it's available via `FieldId::as_named_field()` lookup
- Updating existing code to use the new typed `FieldNodeId<E>` types is deferred to future work
- Serialization is optional and can be added later if needed
