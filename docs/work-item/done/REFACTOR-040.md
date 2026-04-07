# Update FieldValue to use Uuid variants

## Summary

Replace `FieldValue::EntityId(EntityId)` with `FieldValue::Uuid(Uuid)` and remove `FieldValue::InternalId(InternalId)` from `field/mod.rs`.

## Status

Completed

## Priority

High

## Description

Part of REFACTOR-037. `FieldValue` is the universal field value enum used by the field system for reading, writing, and displaying entity data. It currently has two variants tied to the old ID types:

* `EntityId(EntityId)` — wraps a `u64` entity ID
* `InternalId(InternalId)` — wraps an `InternalId` struct (type_name + u64)

Both are being removed as part of the entity ID migration. Replacements:

* `FieldValue::EntityId(EntityId)` → `FieldValue::Uuid(uuid::Uuid)`
* `FieldValue::InternalId(InternalId)` → removed (no replacement; `InternalId` struct is gone)

Also update `FieldValue::Display` impl for the `Uuid` variant.

The `field/mod.rs` currently imports `use crate::EntityId` and `use crate::InternalId` — remove both imports.

## Acceptance Criteria

* `FieldValue::EntityId` variant replaced with `FieldValue::Uuid(Uuid)`
* `FieldValue::InternalId` variant removed
* `Display` impl updated for the new variant
* No remaining references to old `EntityId` or `InternalId` in `field/mod.rs`

## Notes

* The `FieldValue::Uuid` variant is used by the macro for `#[field]`-annotated fields of type `Uuid`
* See parent: REFACTOR-037
