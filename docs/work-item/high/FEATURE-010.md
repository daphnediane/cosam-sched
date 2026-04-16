# FieldValue, Error Types, CrdtFieldType

## Summary

Implement the universal `FieldValue` enum, error types, and CRDT field type annotation.

## Status

Open

## Priority

High

## Description

Core value and error types for the field system in `schedule-core`.

### FieldValue

Universal value enum supporting: String, Text (prose — distinct variant for CRDT
routing), Integer, Float, Boolean, DateTime, Duration, List, NonNilUuid,
EntityIdentifier, None.

### Error types

- `FieldError` — top-level error for field operations
- `ConversionError` — type conversion failures
- `ValidationError` — field validation failures

All use `thiserror`.

### CrdtFieldType

```text
Scalar   — LWW via put_scalar / read_scalar
Text     — Prose RGA via splice_text / read_text
List     — OR-Set-equivalent via list_add/remove / read_list
Derived  — Computed from relationships; NOT stored in CRDT
```

See `docs/crdt-design.md` for the field-to-CRDT mapping rationale.

## Acceptance Criteria

- FieldValue variants cover all needed types
- FieldValue implements Display, PartialEq, Clone, Debug
- Error types use thiserror with descriptive messages
- CrdtFieldType enum has all four variants
- Unit tests for FieldValue conversions and Display
