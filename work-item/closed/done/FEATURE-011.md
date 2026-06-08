# Field Traits + FieldDescriptor

## Summary

Implement the field trait hierarchy and generic `FieldDescriptor` type that replaces the old proc-macro's generated per-field unit structs.

## Status

Completed

## Priority

High

## Blocked By

- FEATURE-010: FieldValue, error types, CrdtFieldType

## Description

### Trait hierarchy

Four flat traits — no `Simple*` or `Schedule*` sub-variants:

```text
NamedField          name(), display_name(), description(), aliases()
ReadableField<E>    read(EntityId<E>, &Schedule) → Option<FieldValue>
WritableField<E>    write(EntityId<E>, &mut Schedule, FieldValue) → Result<(), FieldError>
IndexableField<E>   match_field(&str, &InternalData) → Option<MatchPriority>
```

The caller-facing API is always `(EntityId<E>, &[mut] Schedule)`. Internal
dispatch between data-only and schedule-aware paths is handled by `ReadFn<E>`
and `WriteFn<E>` enums inside `FieldDescriptor`.

### ReadFn / WriteFn enums

```rust
pub enum ReadFn<E: EntityType> {
    Bare(fn(&E::InternalData) -> Option<FieldValue>),
    Schedule(fn(&Schedule, EntityId<E>) -> Option<FieldValue>),
}

pub enum WriteFn<E: EntityType> {
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Used for edge mutations (e.g. add_presenters). Fn handles its own
    /// entity lookup internally, avoiding any double-&mut borrow.
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
}

pub type IndexFn<E> = fn(&str, &<E as EntityType>::InternalData) -> Option<MatchPriority>;
```

### FieldDescriptor

Generic struct with enum fn pointers replacing per-field unit structs:

```rust
pub struct FieldDescriptor<E: EntityType> {
    pub name: &'static str,
    pub display: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
    pub required: bool,
    pub crdt_type: CrdtFieldType,
    pub read_fn:  Option<ReadFn<E>>,  // None → write-only
    pub write_fn: Option<WriteFn<E>>,  // None → read-only
    pub index_fn: Option<IndexFn<E>>,  // None → not indexable
}
```

Declared as `static` values. Non-capturing closures coerce to fn pointers.
Implements all four traits directly (no blanket chain).

Optional `macro_rules!` or proc-macro helpers may reduce boilerplate for common
field patterns, provided entity `Data` struct declarations remain hand-written
and visible (not hidden inside a macro invocation).

## Acceptance Criteria

- Four flat traits compile: `NamedField`, `ReadableField<E>`, `WritableField<E>`, `IndexableField<E>`
- `ReadFn<E>` and `WriteFn<E>` enums declared with `Bare` and `Schedule` variants
- `FieldDescriptor<E>` carries all enum fn fields and implements all four traits
- Static `Bare` descriptors (data fields) compile and pass read/write tests
- Write-only descriptor (`read_fn: None`) returns `FieldError::WriteOnly`
- Static `Schedule` descriptors compile (may use `todo!()` bodies pending FEATURE-008)
- Read-only descriptor (`write_fn: None`) returns `FieldError::ReadOnly`
- `IndexFn` descriptor correctly returns `MatchPriority` variants
- Unit tests with a mock entity verifying read/write/index through descriptors
