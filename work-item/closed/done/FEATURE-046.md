# FEATURE-046: Bulk Field Updates (FieldSet::write_multiple)

## Summary

Add `FieldSet::write_multiple()` for atomic batch field updates with verification support.

## Status

Completed

## Priority

Medium

## Blocked By

- FEATURE-020: Query System (provides field matching infrastructure) — completed

## Related

- FEATURE-017: Builder pattern (will use `write_multiple` internally)
- FEATURE-043: Cross-field verification (provides the `verify_fn` infrastructure consumed here)

## Description

Atomic batch update method for setting multiple fields on a single entity. Essential for interdependent computed fields (e.g., `start_time`, `end_time`, `duration`) where multiple fields must be written and then verified together.

### API Design

```rust
/// Field reference for batch updates — either name string or direct descriptor.
pub enum FieldRef<E: EntityType> {
    /// Field name string (requires lookup in FieldSet).
    Name(&'static str),
    /// Direct field descriptor reference (zero-cost, compile-time checked).
    Descriptor(&'static FieldDescriptor<E>),
}

impl<E: EntityType> FieldSet<E> {
    /// Write multiple fields atomically and verify results.
    pub fn write_multiple(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        updates: &[(FieldRef<E>, FieldValue)],
    ) -> Result<(), FieldSetError>;
}

// Convenience From impls for ergonomic call sites:
// - "field_name".into() → FieldRef::Name
// -&FIELD_NAME.into() → FieldRef::Descriptor

#[derive(Debug, Error)]
pub enum FieldSetError {
    #[error("unknown field '{0}'")]
    UnknownField(String),
    #[error("duplicate field '{0}' in batch update")]
    DuplicateField(String),
    #[error("write failed for field '{field}': {error}")]
    WriteError { field: String, error: FieldError },
    #[error("verification failed for field '{field}': {error}")]
    VerificationError { field: String, error: VerificationError },
}
```

### Behavior

1. **Resolve**: For each `FieldRef`:
   - `Name(name)` → lookup field in FieldSet (error if not found)
   - `Descriptor(field)` → use directly (zero-cost)
2. **Validate**: Check all resolved fields exist
3. **De-duplicate**: Error on duplicate fields (or use last value with warning)
4. **Write phase**: Apply all writes to the schedule in order
5. **Verify phase**: Run `VerifiableField::verify()` for each field with `verify_fn`
6. **Error handling**: Return first error; writes are not rolled back

### Integration with FEATURE-043

This method is what calls the `verify()` callbacks after batch writes. Single-field writes skip verification; batch writes run verification for all fields with `verify_fn` set.

## Acceptance Criteria

- [x] `FieldRef<E>` enum with `Name` and `Descriptor` variants (+ `From` conversions)
- [x] `FieldSet::write_multiple()` method implemented
- [x] `FieldSetError` type with all variants (`UnknownField`, `DuplicateField`, `WriteError`, `VerificationError`)
- [x] Unknown field detection for `FieldRef::Name`
- [x] Duplicate field handling (by pointer identity; catches name+descriptor and canonical+alias)
- [x] Verification phase runs after writes
- [x] Unit tests for batch updates (`Name`, `Descriptor`, mixed)
- [x] Unit tests for verification integration
- [x] Unit tests for `VerifyFn::Bare` custom verification
- [x] Unit tests for `VerifyFn::Schedule` custom verification
- [x] Unit tests for `VerifyFn::ReRead` read-back verification

## Additional deliverables

- `FieldSet::write_many` — ergonomic wrapper accepting any `IntoFieldValue`-typed value, used by the `define_entity_builder!` macro in FEATURE-017.

## Notes

- Split out from FEATURE-020 to allow separate prioritization.
- Consumes FEATURE-043's verification callbacks.
- No rollback in `write_multiple` — first error aborts and earlier writes remain applied. Entity-level rollback is the builder's responsibility (FEATURE-017).
- `repeatable` field flag deferred; not needed for the current call sites.
