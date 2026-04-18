# FEATURE-051: Add field_type to FieldDescriptor

## Summary

Add a `field_type: FieldType` field to `FieldDescriptor` and populate it in all
existing static field descriptors across every entity file.

## Status

Open

## Priority

High

## Blocked By

- META-048: FieldValue / FieldType / Converter Overhaul (parent)

## Description

`FieldDescriptor` currently has `crdt_type: CrdtFieldType` to declare CRDT routing,
but no field for the value's logical type. Adding `field_type: FieldType` allows
callers (converters, importers, UI) to know what type a field expects without
calling read/write.

### Changes

- `field.rs`: add `pub field_type: FieldType` to `FieldDescriptor<E>` struct
- `field_macros.rs`: add `field_type` argument to all macro invocations that build
  a `FieldDescriptor` (each macro generates the correct `FieldType` variant)
- All entity files (`panel.rs`, `presenter.rs`, `panel_type.rs`, `event_room.rs`,
  `hotel_room.rs`): add `field_type` to every static `FieldDescriptor` initializer

### Field type mapping

| Macro / pattern     | FieldType                                         |
| ------------------- | ------------------------------------------------- |
| `req_string_field!` | `FieldType::Single(FieldTypeItem::String)`        |
| `opt_string_field!` | `FieldType::Optional(FieldTypeItem::String)`      |
| `opt_text_field!`   | `FieldType::Optional(FieldTypeItem::Text)`        |
| `bool_field!`       | `FieldType::Single(FieldTypeItem::Boolean)`       |
| `opt_i64_field!`    | `FieldType::Optional(FieldTypeItem::Integer)`     |
| `edge_list_field!`  | `FieldType::List(FieldTypeItem::EntityId("..."))` |

### Acceptance Criteria

- Every `FieldDescriptor` in the codebase has a `field_type` field set
- `cargo test` passes
- No `FieldDescriptor` initializer leaves `field_type` defaulted or missing
