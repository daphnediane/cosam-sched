# Generic FieldValue Conversion System

## Summary

Add generic support for arbitrary FieldValue-to-FieldValue conversions with
customizable conversion strategies, including lookup-only and create-capable variants.

## Status

Open

## Priority

Low

## Description

Currently `resolve_field_value` only handles converting a `FieldValue` to entity
IDs. A more flexible system would support generic conversions, enabling:

- **Tagged presenter support**: `"P:Name"` → Presenter with rank
- **Custom conversion pipelines**: Chain multiple conversions
- **Type-specific logic**: Each entity type defines its own rules

### Design sketch

A `FieldValueConverter` trait with work-queue iteration pattern:

- `lookup_next_field_value<T>(&EntityStorage, FieldValue)` — read-only
- `resolve_next_field_value<T>(&mut EntityStorage, FieldValue)` — create-capable
- Hookable `combine_results<T>` for merging results
- Dispatch based on (FieldValue variant, target type T) combination

### When to implement

When the import path needs more than entity ID resolution (e.g., tagged
presenter import with rank assignment, group membership creation).

## Related

- IDEA-037: Read-only entity resolution
