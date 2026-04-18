# FEATURE-038: FieldValueConverter System

## Summary

Add a type-safe `FieldValueConverter<M>` trait and driver functions for converting
`FieldValue` inputs to typed Rust outputs via a work-queue iteration pattern.

## Status

Completed

## Priority

High

## Blocked By

- FEATURE-051: Add field\_type to FieldDescriptor
- META-048: FieldValue / FieldType / Converter Overhaul (parent)

## Description

Promoted from IDEA-038. Implements the generic conversion system needed by the import
pipeline (e.g., tagged presenter `"P:Name"` → `EntityId<PresenterEntityType>` with
rank assignment).

### Design

**`FieldTypeMapping` trait** — maps a marker type to a Rust output type:

```rust
pub trait FieldTypeMapping: 'static {
    type Output;
    fn field_type_item() -> FieldTypeItem;
    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError>;
    fn to_field_value_item(output: Self::Output) -> FieldValueItem;
}
```

Standard marker types: `AsString`, `AsText`, `AsInteger`, `AsFloat`, `AsBoolean`,
`AsDateTime`, `AsDuration`, `AsEntityId<E: EntityType>`.

**`FieldValueConverter<M: FieldTypeMapping>` trait**:

```rust
pub trait FieldValueConverter<M: FieldTypeMapping> {
    fn lookup_next(&self, schedule: &Schedule, input: FieldValueItem)
        -> Option<Result<M::Output, ConversionError>>;
    fn resolve_next(&self, schedule: &mut Schedule, input: FieldValueItem)
        -> Option<Result<M::Output, ConversionError>> { /* delegates */ }
    fn select_one(&self, outputs: Vec<M::Output>)
        -> Result<Option<M::Output>, ConversionError> { /* first */ }
}
```

**Six driver functions**: `lookup_one / lookup_optional / lookup_many` (read-only) and
`resolve_one / resolve_optional / resolve_many` (mutable). Drivers expand
`FieldValue::List` as a work queue and handle `Optional(None)` as empty input.

IDEA-037 (read-only vs. mutable resolution) is captured by the `lookup_*` /
`resolve_*` split. IDEA-037 can be marked Completed when this feature lands.

### Files

- New: `crates/schedule-core/src/converter.rs`
- Edit: `crates/schedule-core/src/lib.rs` — add `pub mod converter;`

### Acceptance Criteria

- All six driver functions implemented and tested
- `AsEntityId<E>` correctly validates entity type name on extraction
- `resolve_next` default delegates to `lookup_next`
- `cargo test` passes

## Related

- IDEA-037: Read-only entity resolution (captured by this feature's lookup/resolve split)
