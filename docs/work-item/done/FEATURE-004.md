# Field System: Traits, FieldValue, FieldSet, Validation

## Summary

Implement the field trait hierarchy, universal `FieldValue` enum, `FieldSet` registry,
and validation infrastructure.

## Status

Completed

## Priority

High

## Description

The field system provides type-safe, generic access to entity fields for editing,
querying, import/export, and display.

### Trait Hierarchy

```text
NamedField                    name(), display_name(), description()
├── SimpleReadableField<T>    read(&entity) → Option<FieldValue>
│   └── (blanket) ReadableField<T>
├── SimpleWritableField<T>    write(&mut entity, FieldValue) → Result
│   └── (blanket) WritableField<T>
├── SimpleCheckedField<T>     validate(&mut entity, &FieldValue) → Result
│   └── (blanket) CheckedField<T>
├── IndexableField<T>         match_field(query, &entity) → Option<MatchPriority>
├── ReadableField<T>          read(&Schedule, &entity) → Option<FieldValue>
├── WritableField<T>          write(&Schedule, &mut entity, FieldValue) → Result
└── CheckedField<T>           validate(&Schedule, &mut entity, &FieldValue) → Result
```

### FieldValue

Universal value enum supporting: String, Integer, Float, Boolean, DateTime,
Duration, List, Map, Optional variants, and NonNilUuid.

### FieldSet

Static registry per entity type containing:

- Ordered list of field references
- Name → field lookup map (including aliases)
- Required field names
- Indexable fields with priority

### Validation

- `ValidationError` and `ConversionError` types
- Required-field checks
- Type conversion validation

## Acceptance Criteria

- All traits compile with blanket impls
- FieldValue round-trips through Display
- FieldSet lookup by name and alias works
- Unit tests for read, write, validate paths
