# FEATURE-050: Add FieldTypeItem and FieldType enums

## Summary

Add `FieldTypeItem` (scalar type tags) and `FieldType` (`Single`/`Optional`/`List`
wrappers) to `value.rs` as `Copy` type-level mirrors of `FieldValueItem`/`FieldValue`.

## Status

Open

## Priority

High

## Blocked By

- REFACTOR-049: Restructure FieldValue → FieldValueItem + cardinality
- META-048: FieldValue / FieldType / Converter Overhaul (parent)

## Description

Ports and improves the `FieldType` enum from v10-try3 (`schedule-field/src/type_kind.rs`).
The v10-try3 version has a bare `List` variant; this version uses `List(FieldTypeItem)`
to carry the element type, avoiding `Box` and preventing `List<List<_>>`.

### New types (pure addition to `value.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldTypeItem {
    String, Text, Integer, Float, Boolean, DateTime, Duration,
    /// Typed entity reference — the &'static str is EntityType::TYPE_NAME.
    EntityId(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    Single(FieldTypeItem),
    Optional(FieldTypeItem),
    List(FieldTypeItem),
}
```

### Methods and traits

- `Display` for `FieldTypeItem` — e.g. `"Integer"`, `"EntityId(presenter)"`
- `Display` for `FieldType` — `Single(x)` → `"{x}"`, `Optional(x)` → `"{x}?"`,
  `List(x)` → `"List<{x}>"`
- `FieldType::item_type(self) -> FieldTypeItem`
- `FieldType::is_single / is_optional / is_list`
- `FieldType::of(value: &FieldValue) -> Option<FieldType>` — infer from a FieldValue
  (returns `None` for `EntityId` variants since the type name is not `'static`)

### Acceptance Criteria

- Both enums are `Copy` (verified by compiler)
- All Display, predicate, and `of` methods covered by tests
- No changes to any file outside `value.rs`
