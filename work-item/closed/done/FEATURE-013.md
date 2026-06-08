# FieldSet Registry

## Summary

Implement the static `FieldSet` registry for per-entity-type field metadata lookup.

## Status

Completed

## Priority

High

## Blocked By

- FEATURE-011: Field traits + FieldDescriptor
- FEATURE-012: EntityType, EntityId, EntityKind

## Description

`FieldSet<E>` is a static registry holding all field descriptors for an entity type,
built once in a `LazyLock` and returned by `EntityType::field_set()`.

### Capabilities

- **Name/alias lookup**: `get_by_name(&str) -> Option<&FieldDescriptor<E>>`
- **Required fields**: `required_fields() -> &[&FieldDescriptor<E>]`
- **Indexable fields**: `indexable_fields() -> &[&FieldDescriptor<E>]`
- **Readable/writable lists**: separate iterators for read-only and read-write fields
- **CRDT field list**: `crdt_fields() -> &[(name, CrdtFieldType)]` for materialization
- **Field-value read/write**: `read_field_value(name, &data)` and `write_field_value(name, &mut data, value)`
- **Index matching**: `match_index(query, &data)` across all indexable fields

### Implementation

Constructed from a slice of `&'static FieldDescriptor<E>` references. Internal
`HashMap<&str, usize>` for O(1) name/alias lookup built at init time.

## Acceptance Criteria

- Name and alias lookup returns correct descriptors
- Required/indexable/readable/writable partitions are correct
- `read_field_value` and `write_field_value` dispatch correctly
- Unit tests for all lookup paths
