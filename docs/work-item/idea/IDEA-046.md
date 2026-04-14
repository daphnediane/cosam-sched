# IDEA-046: Generic FieldValue to FieldValue conversion system

## Summary

Add generic support for arbitrary FieldValue to FieldValue conversions with customizable conversion strategies, including lookup-only and create-capable variants

## Status

Open

## Priority

Low

## Description

Currently the `resolve_field_value` and `resolve_field_values` methods on `EntityResolver` only handle converting a `FieldValue` to `Option<EntityType::Id>` or `Vec<EntityType::Id>`. This is limiting for cases where we need to convert between different `FieldValue` kinds before final entity resolution.

A more flexible system would support generic `FieldValue` to `FieldValue` conversions with customizable strategies. This would enable:

- **Tagged presenter support**: Conversions like `"P:Name"` → `Presenter` entity with rank, or `"G:Group=Member"` → group membership relationships
- **Custom conversion pipelines**: Chain multiple conversions (e.g., string → tagged string → entity reference)
- **Type-specific conversion logic**: Each entity type can define its own conversion rules

## Design

Building on IDEA-043's read-only vs. create-capable distinction, each entity type should provide two conversion families:

### Lookup-only conversions (read-only)

- Take `&EntityStorage` (shared reference)
- Return `Result<FieldValue, FieldError>` or `Option<FieldValue>`
- Never create new entities
- Used for validation, display, read-only queries

### Create-capable conversions (mutable)

- Take `&mut EntityStorage` (mutable reference)
- Return `Result<FieldValue, FieldError>` or `Option<FieldValue>`
- May create new entities as needed
- Used for import, editing, mutation operations

### Conversion trait

The design should follow the existing `resolve_next_field_value` pattern (see `EntityResolver` in `crates/schedule-data/src/entity/mod.rs`), which returns `(Vec<Self::Id>, Vec<FieldValue>)` — resolved IDs and additional work queue items.

A generic conversion system should:

- Use a work queue iteration pattern similar to current implementation
- Support generic `resolve_field_value<T>` methods that iterate the work queue
- Provide hookable methods to combine zero or more looked-up/created FieldValues into the expected type
- Dispatch based on FieldValue contents through type-specific handlers

Proposed design:

```rust
trait FieldValueConverter {
    // Lookup-only variant (shared reference, no creation)
    fn lookup_next_field_value<T>(
        storage: &EntityStorage,
        value: FieldValue,
    ) -> Result<(Vec<T>, Vec<FieldValue>), FieldError>;

    // Create-capable variant (mutable reference, may create)
    fn resolve_next_field_value<T>(
        storage: &mut EntityStorage,
        value: FieldValue,
    ) -> Result<(Vec<T>, Vec<FieldValue>), FieldError>;

    // Hookable method to combine results into expected type
    fn combine_results<T>(
        results: Vec<T>,
    ) -> Result<T, FieldError>;

    // Hookable dispatch based on (FieldValue variant, T) combination
    fn dispatch_conversion<T>(
        storage: &EntityStorage,
        value: FieldValue,
    ) -> Result<(Vec<T>, Vec<FieldValue>), FieldError>;

    fn dispatch_conversion_mut<T>(
        storage: &mut EntityStorage,
        value: FieldValue,
    ) -> Result<(Vec<T>, Vec<FieldValue>), FieldError>;
}
```

The internals of `lookup_next_field_value<T>` and `resolve_next_field_value<T>` should be hookable based on the combination of:

- **Source FieldValue variant** (e.g., `String`, `List`, `NonNilUuid`, `EntityIdentifier`)
- **Desired type T** (e.g., `String`, `EntityType::Id`, `List<String>`)

This allows different conversion strategies depending on the (FieldValue variant, T) pair. For example:

- `FieldValue::String("Alice,Bob")` → `List<String>`: split on comma
- `FieldValue::String("Alice")` → `String`: passthrough
- `FieldValue::String("Alice")` → `Presenter::Id`: resolve via `resolve_string`
- `FieldValue::List([String("Alice"), String("Bob")])` → `List<String>`: passthrough
- `FieldValue::List([String("Alice"), String("Bob")])` → `Vec<Presenter::Id>`: resolve each element

Example usage pattern:

```rust
// To get List<String>
fn resolve_field_value_list_string(
    storage: &mut EntityStorage,
    value: FieldValue,
) -> Result<Vec<String>, FieldError> {
    let mut work_queue = vec![value];
    let mut results = Vec::new();

    while let Some(item) = work_queue.pop() {
        let (resolved, additional) =
            Self::resolve_next_field_value::<String>(storage, item)?;
        results.extend(resolved);
        work_queue.extend(additional);
    }

    Self::combine_results(results)
}
```

The `resolve_next_field_value<T>` method would dispatch based on FieldValue contents:

- `FieldValue::List(items)` → return items as additional work
- `FieldValue::String(s)` with commas → split and return as additional work
- `FieldValue::String(s)` without commas → resolve to single T
- `FieldValue::NonNilUuid` → resolve to entity ID, convert to T if needed

Each entity type implements the trait with its specific conversion logic (e.g., Presenter's tag parsing, rank assignment, group membership).

### Current vs. proposed

**Current**: `FieldValue` → `EntityType::Id` (single step, hardcoded, entity-specific)

The current `resolve_next_field_value` returns `(Vec<Self::Id>, Vec<FieldValue>)` but is only implemented for entity ID resolution. Each entity type hardcodes its conversion logic.

**Proposed**: `FieldValue` → generic type `T` via work-queue iteration with hookable combination

The proposed system generalizes the work-queue pattern to support arbitrary output types `T`:

- Generic `resolve_next_field_value<T>` that can produce any type, not just entity IDs
- Hookable `combine_results<T>` to merge zero or more results into the expected output
- Type-specific dispatch logic for each FieldValue variant
- Maintains the existing `(Vec<T>, Vec<FieldValue>)` return pattern for work queue iteration

This enables:

- Converting to `List<String>`, single `String`, entity IDs, or any other type
- Custom merge strategies (e.g., comma-join for strings, list concatenation)
- Reusable conversion logic across different output types
- Testable, composable conversion pipeline

## Open questions

- Should conversions be composable/chained, or is a single conversion per entity type sufficient?
- How to handle conversion errors that might be recoverable vs. fatal?
- Should the conversion trait be part of `EntityResolver` or separate?
- Interaction with existing `resolve_field_value`/`resolve_field_values` — deprecate or extend?
- Should `ConversionResult` include error variants, or use `Result<ConversionResult, FieldError>`?
- How should merge strategies be specified — enum parameter, trait method, or separate configuration?
- For nested conversions, should additional work be processed immediately or deferred to caller?

## Related

- IDEA-043: Read-only entity resolution (lookup without creation)
- Current implementation in `crates/schedule-data/src/entity/mod.rs` (EntityResolver trait)
- Presenter tag parsing in `crates/schedule-data/src/entity/presenter.rs`
