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

`find_or_create_tagged_presenter(storage, input)` — parses presenter credit
strings from spreadsheet cells. Handles tagged forms like `G:Name`, `P:Name=Group`.

### Updater

- Bulk field updates via field name + FieldValue pairs
- Validation before applying
- Integration with edit command system (FEATURE-021)

## Acceptance Criteria

- Can find entities by exact field match
- Can find entities by text search across indexable fields
- Match results ordered by priority
- Bulk updates apply correctly and validate
- Unit tests for find, match, and update paths
