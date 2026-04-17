# REFACTOR-049: Restructure FieldValue → FieldValueItem + cardinality

## Summary

Split the flat `FieldValue` enum into `FieldValueItem` (scalars only) and
`FieldValue` (`Single`/`Optional`/`List` wrappers), removing `None`,
`NonNilUuid`, and `EntityIdentifier` variants.

## Status

Open

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
    Optional(Option<FieldValueItem>),
    List(Vec<FieldValueItem>),
}
```

### Files changed

- `value.rs`: add `FieldValueItem`, redefine `FieldValue`, remove `EntityIdentifier` enum,
  move `into_*` methods to `FieldValueItem`, add `FieldValue::into_single/optional/list`,
  add `FieldValue::is_absent()`
- `field.rs`: update `ReadFn`/`WriteFn` — `Bare` returns `FieldValue` (not `Option<FieldValue>`);
  `Schedule` variants return `FieldValue`/`Result<FieldValue, FieldError>`
- `field_macros.rs`: update all macros to produce/consume the new structure
- `field_set.rs`: update `read_field_value`/`write_field_value` if signatures change
- All entity files (`panel.rs`, `presenter.rs`, `panel_type.rs`, `event_room.rs`,
  `hotel_room.rs`): update every field read/write function and associated tests

### Acceptance Criteria

- `cargo test` passes with no regressions
- No `FieldValue::None` or `FieldValue::NonNilUuid` or `FieldValue::EntityIdentifier`
  remain in non-test code
- Absent optional fields return `FieldValue::Optional(None)`; empty lists return
  `FieldValue::List(vec![])`
