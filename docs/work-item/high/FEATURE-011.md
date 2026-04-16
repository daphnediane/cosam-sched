# Field Traits + FieldDescriptor

## Summary

Implement the field trait hierarchy and generic `FieldDescriptor` type that replaces the proc-macro's per-field unit structs.

## Status

Open

## Priority

High

## Blocked By

- FEATURE-010: FieldValue, error types, CrdtFieldType

## Description

### Trait hierarchy

```text
NamedField                    name(), display_name(), description()
├── SimpleReadableField<E>    read(&data) → Option<FieldValue>
│   └── (blanket) ReadableField<E>
├── SimpleWritableField<E>    write(&mut data, FieldValue) → Result
│   └── (blanket) WritableField<E>
└── IndexableField<E>         match_field(query, &data) → Option<MatchPriority>
```

Blanket impls promote Simple variants to Full variants (which also accept a
database/schedule context parameter).

### FieldDescriptor

Generic struct with fn pointers replacing per-field unit structs:

```rust
pub struct FieldDescriptor<E: EntityType> {
    pub name: &'static str,
    pub display: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
    pub required: bool,
    pub crdt_type: CrdtFieldType,
    pub read_fn: fn(&E::Data) -> Option<FieldValue>,
    pub write_fn: Option<fn(&mut E::Data, FieldValue) -> Result<(), FieldError>>,
}
```

Declared as `static` values. Non-capturing closures coerce to fn pointers.
Implements `NamedField`, `SimpleReadableField<E>`, `SimpleWritableField<E>`.

Optional `macro_rules!` helpers may reduce boilerplate for common field patterns.

## Acceptance Criteria

- All traits compile with blanket impls
- FieldDescriptor implements all required traits
- Static field descriptors with fn-pointer closures compile
- Unit tests with a mock entity verifying read/write through descriptors
