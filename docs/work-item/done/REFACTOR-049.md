# REFACTOR-049: Restructure FieldValue → FieldValueItem + cardinality

## Summary

Split the flat `FieldValue` enum into `FieldValueItem` (scalars only) and
`FieldValue` (`Single`/`List` wrappers), removing `None`,
`NonNilUuid`, and `EntityIdentifier` variants.

## Status

Completed

## Priority

High

## Blocked By

- META-048: FieldValue / FieldType / Converter Overhaul (parent)

## Description

The current `FieldValue` enum mixes scalar data, list cardinality, and absence
into a single flat structure. This makes it hard to reason about field types and
requires special-casing `None` everywhere.

### New structure

```rust
pub enum FieldValueItem {
    String(String),
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(NaiveDateTime),
    Duration(Duration),
    EntityId(RuntimeEntityId),   // replaces NonNilUuid + EntityIdentifier
}

pub enum FieldValue {
    Single(FieldValueItem),
    List(Vec<FieldValueItem>),
}
```

### Files changed

- `value.rs`: add `FieldValueItem`, redefine `FieldValue`, remove `EntityIdentifier` enum,
  move `into_*` methods to `FieldValueItem`, add `FieldValue::into_single/list`,
  add `FieldValue::is_empty()` (note: used `is_empty()` instead of planned `is_absent()`)
  `Schedule` variants return `FieldValue`/`Result<FieldValue, FieldError>`
- `value_macros.rs`: new file with convenience macros for creating `FieldValue` instances
- `field_macros.rs`: update all macros to produce/consume the new structure
- `field_set.rs`: update `read_field_value`/`write_field_value` if signatures change
- All entity files (`panel.rs`, `presenter.rs`, `panel_type.rs`, `event_room.rs`,
  `hotel_room.rs`): update every field read/write function and associated tests

### Acceptance Criteria

- `cargo test` passes with no regressions
- No `FieldValue::None` or `FieldValue::NonNilUuid` or `FieldValue::EntityIdentifier`
  remain in non-test code
- Absent optional fields return `None`; empty lists return `FieldValue::List(vec![])`

### Implementation Details

**Completed:**

- `FieldValueItem` enum created with variants: `String`, `Text`, `Integer`, `Float`, `Boolean`, `DateTime`, `Duration`, `EntityIdentifier`
- `FieldValue` enum redefined with `Single(FieldValueItem)` and `List(Vec<FieldValueItem>)` wrappers
- Removed `None`, `NonNilUuid` variants from `FieldValue`
- Moved `into_*` methods to `FieldValueItem`
- Added `FieldValue::into_single()`, `into_list()`, `is_empty()`, `is_single()` methods
- Added `value_macros.rs` with convenience macros for creating FieldValue instances
- All entity files updated to use new structure
- All tests pass (206 passed, 0 failed)

**Differences from plan:**

- Used `EntityIdentifier` variant name in `FieldValueItem` instead of `EntityId` (naming consistency with existing code)
- Removed `Optional` variant from `FieldValue` (originally planned) - unnecessary since absent optional fields can return `None` from read functions directly
- Implemented `is_empty()` instead of planned `is_absent()` - more semantically accurate for checking empty lists
- Added `value_macros.rs` file (not in original plan) to provide convenient macro helpers for creating FieldValue instances
