# Query API Surface

## Summary

Implement public query families for all entity types with ranked index matching.

## Status

Not Started

## Priority

High

## Description

Port and extend query capabilities from `schedule-core/src/edit/find.rs` into `schedule-data`, providing typed query families per entity type with ranked `FieldSet` index matching.

## Implementation Details

- Implement query families per entity type:
  - `list_<thing>` — sorted by internal ID with state filters (active/inactive)
  - `all_<thing>s` — unsorted entity objects with field/value access
  - `find_<thing>` — by `FieldMatch`, returns internal IDs
  - `lookup_<thing>` — by `FieldMatch`, returns entity data objects
- Ensure lookups by indexed fields are supported (room names, presenter name, panel uid/name, etc.)
- Use `FieldSet` scoring (`NotMatch`/`WeakMatch`/`StrongMatch`/`ExactMatch`) in query/lookup paths
- Add explicit internal-ID lookup methods for every entity type
- Port relevant find logic from `schedule-core/src/edit/find.rs`

## Acceptance Criteria

- All entity types have complete query family methods on `Schedule`
- Index-based lookups return ranked results with proper priority scoring
- State filters correctly exclude inactive entities by default
- Internal-ID lookups are O(1) via HashMap
- Query API documentation complete with examples
