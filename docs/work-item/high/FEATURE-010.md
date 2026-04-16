# FieldValue, Error Types, CrdtFieldType

## Summary

Implement the universal `FieldValue` enum, error types, and CRDT field type annotation.

## Status

Open

## Priority

High

## Description

Core value and error types for the field system in `schedule-core`.

### Crate skeleton (prerequisite)

`schedule-core` is currently an empty directory. Before implementing any types,
create the crate skeleton:

- `crates/schedule-core/Cargo.toml` — with `uuid`, `chrono`, `serde`, `thiserror`
  as dependencies
- `crates/schedule-core/src/lib.rs` — module stubs: `pub mod field; pub mod entity; pub mod value;`
- Wire `crates/schedule-core` into the workspace `Cargo.toml` `members` list
- `cargo build` at workspace root passes

**Macro policy**: entity `Data` struct declarations must be hand-written and
visible. Proc-macros and `macro_rules!` are allowed for boilerplate (trait
impls, field accessor singletons, builders) as long as they do not hide the
struct definitions.

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
