# REFACTOR-055: Unify field registration via `define_field!` and add `IntoFieldValue` trait

## Summary

Add `define_field!` macro to bundle hand-written `FieldDescriptor` statics with
`inventory::submit!`, and add `IntoFieldValue` trait hierarchy for type-deduced
`field_value!(expr)` construction.

## Status

Completed

## Priority

Medium

## Description

Two related improvements to reduce boilerplate and prevent silent omission of fields from
the registry:

1. **`define_field!` macro** — hand-written `FieldDescriptor` statics currently require a
   separate `inventory::submit!` call after each one. Forgetting it silently omits the
   field from the registry with no compiler error. The new `define_field!` macro wraps
   both into a single declaration. Affects 8 hand-written statics across `panel.rs`,
   `presenter.rs`, `event_room.rs`, and `panel_type.rs`.

2. **`IntoFieldValue` trait hierarchy** — constructing `FieldValue` values currently
   requires naming the type variant explicitly (`field_string!`, `field_datetime!`, etc.)
   because `macro_rules!` cannot dispatch on types. Adding `IntoFieldValueItem` +
   `IntoFieldValue` traits with blanket `impl`s for all scalar types, `Option<T>`, and
   `Vec<T>` allows a single `field_value!(expr)` macro arm to select the right variant
   via Rust's trait dispatch.

No proc macros — both improvements use `macro_rules!` + traits, preserving full
visibility of `FieldDescriptor` literal bodies.

## Implementation Details

- `field_macros.rs`: add `define_field!` macro
- `value.rs`: add `IntoFieldValueItem` + `IntoFieldValue` traits and impls
- `value_macros.rs`: add `($e:expr)` arm to `field_value!` (last arm, after existing)
- Migrate 8 statics: `FIELD_CODE`, `FIELD_START_TIME`, `FIELD_END_TIME`, `FIELD_DURATION`
  (panel.rs), `FIELD_RANK`, `FIELD_IS_GROUP` (presenter.rs), `FIELD_LONG_NAME`
  (event_room.rs), `FIELD_DISPLAY_NAME` (panel_type.rs)
- Existing `field_string!`, `field_datetime!` etc. remain as-is (backward compatible)
