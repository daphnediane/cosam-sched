# Query System

## Summary

Implement field-based search, matching, and bulk update operations.

## Status

Open

## Priority

Medium

## Description

The query system enables finding and updating entities using field-based
criteria rather than direct UUID access.

### Finder

- `FieldMatch` — criteria struct with field name, operator, and value
- `QueryOptions` — pagination, sorting, field filters
- `find::<T>(matches, options)` → list of matching UUIDs
- `get_many::<T>(matches, options)` → list of matching entity data references

### Matching / Indexing

- `IndexableField<T>` trait for fields that participate in text search
- `MatchPriority` (u8) with standard levels: ExactMatch(255), StrongMatch(200),
  AverageMatch(100), WeakMatch(50), NoMatch(0)
- `FieldMatchResult` with entity UUID, match priority, field priority, field name
- Custom match closures per field (e.g., Panel name with word-boundary matching)

### Updater

- Bulk field updates via field name + FieldValue pairs
- Validation before applying updates
- Integration with edit command system (FEATURE-010) for undo support

## Acceptance Criteria

- Can find entities by exact field match
- Can find entities by text search across indexable fields
- Match results are ordered by priority
- Bulk updates apply correctly and validate
- Unit tests for find, match, and update paths
