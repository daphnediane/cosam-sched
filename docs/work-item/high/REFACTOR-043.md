# Update EntityFields macro for UUID-based entity IDs

## Summary

Update `schedule-macro/src/lib.rs` to emit `entity_uuid: uuid::Uuid` in generated `*Data` structs, generate a `new()` constructor with `Uuid::new_v4()`, generate a `to_public()` method, and replace `FieldTypeCategory::EntityId`/`InternalId` with `Uuid`.

## Status

Open

## Priority

High

## Description

Part of REFACTOR-037. The `#[derive(EntityFields)]` macro generates internal `*Data` structs and trait implementations. This phase updates those generated items for the UUID migration.

Changes to `crates/schedule-macro/src/lib.rs`:

* Generated `*Data` struct field: `entity_id: crate::entity::EntityId` → `entity_uuid: uuid::Uuid`
* Generated `impl InternalData for *Data`:
  * `entity_id(&self) -> EntityId` → `uuid(&self) -> uuid::Uuid`
  * `set_entity_id(&mut self, id: EntityId)` → `set_uuid(&mut self, uuid: uuid::Uuid)`
* New generated method `pub fn new(...all_stored_fields...) -> Self` that sets `entity_uuid: uuid::Uuid::new_v4()` and the provided field values
* New generated method `pub fn to_public(&self) -> OriginalStruct` that clones all stored and computed-backing fields from `*Data` into a new instance of the original struct
* `FieldTypeCategory` enum: remove `EntityId` and `InternalId` variants; add `Uuid`
* `get_field_type_category`: map `"Uuid"` → `FieldTypeCategory::Uuid`; remove `"EntityId"` and `"InternalId"` branches
* `is_supported_type`: replace `"EntityId"` and `"InternalId"` with `"Uuid"`
* `supports_automatic_write`: same replacement
* Read/write conversion match arms: handle `FieldTypeCategory::Uuid` using `FieldValue::Uuid(entity.#field_name)`

The `new()` constructor generation requires collecting all stored field names and types from the struct definition to emit the function signature.

The `to_public()` generation iterates all fields in `stored_field_names_for_copy` and emits `field: self.field.clone()` for each.

## Acceptance Criteria

* Each entity's `*Data::new(...)` constructor compiles and calls `Uuid::new_v4()`
* Each entity's `*Data::to_public()` returns the original struct with all fields cloned
* `FieldTypeCategory::Uuid` handles read/write/index operations
* `EntityId`/`InternalId` variants completely removed from macro

## Notes

* The macro emits `uuid::Uuid::new_v4()` as a token stream — no runtime `uuid` dependency needed in `schedule-macro` crate itself
* The `new()` constructor must exclude computed fields from its parameters; they start as defaults
* See parent: REFACTOR-037
