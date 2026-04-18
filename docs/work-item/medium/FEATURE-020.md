# Query System

## Summary

Implement field-based search, matching, and bulk update operations.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-019: Schedule container + EntityStorage

## Description

The query system enables finding and updating entities using field-based
criteria rather than direct UUID access.

### Finder

- `FieldMatch` — criteria struct with field name, operator, and value
- `find::<T>(matches)` → list of matching UUIDs
- `get_many::<T>(matches)` → list of matching entity data references

### Matching / Indexing

- `IndexableField<E>` for fields that participate in text search
- `MatchPriority` (u8) with levels: ExactMatch, StrongMatch, AverageMatch,
  WeakMatch, NoMatch
- Custom match closures per field

### Presenter tag-string import

Note that `Kind:` is optional, and if not provided, the rank of an existing
presenter matching name or group will not be changed. But if creating a presenter
without `Kind:` it should default to `P` (Presenter) even though that isn't the
lowest rank.

`find_tagged_presenter(storage, input)` — parses presenter credit
strings from spreadsheet cells. Handles tagged forms like `G:Name`, `P:Name=Group`.
Returns error if can not found or if found but lower rank for presenter or group.
For group only stuff like `=Group`, `==Group` or `I:==Group` returns the PresenterId of the group, otherwise return the PresenterId of the member or name given.

`find_or_create_tagged_presenter(storage, input)` — parses presenter credit
strings from spreadsheet cells. Handles tagged forms like `G:Name`, `P:Name=Group`.
Creates presenter / group if not found and updates ranks / relationship.

### Related

- Bulk field updates: see FEATURE-046
- Tagged format: `Kind:Name=Group` in spreadsheet format docs

## Acceptance Criteria

- Can find entities by exact field match
- Can find entities by text search across indexable fields
- Match results ordered by priority
- Unit tests for find and match paths
