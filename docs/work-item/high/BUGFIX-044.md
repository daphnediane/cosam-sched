# Panel EntityResolver fails for EntityIdentifier and prefixed UUID string

## Summary

`PanelEntityType::resolve_field_value` returns errors for `EntityIdentifier` and prefixed UUID string inputs even when the panel exists in storage.

## Status

Open

## Priority

High

## Description

Two tests in `entity::tests` (mod.rs) fail:

- `resolve_field_value_entity_identifier` — `Err(Empty)` when resolving
  `FieldValue::EntityIdentifier(EntityUUID::Panel(panel_id))`
- `resolve_field_value_prefixed_uuid_string` — assertion failure when resolving
  `FieldValue::String("panel-<uuid>")`

The equivalent presenter tests (`resolve_by_entity_identifier`,
`resolve_by_prefixed_uuid_string`, `resolve_by_bare_uuid_string`,
`resolve_by_non_nil_uuid`) all pass because `PresenterEntityType` has a custom
`resolve_string` that calls `resolve_uuid_string` correctly.

Panel uses the default macro-generated `EntityResolver` impl. The `contains_uuid`
check in `resolve_next_field_value` (for `EntityIdentifier`) and `resolve_uuid_string`
(for prefixed strings) fails to find the panel even though `PanelBuilder::build`
successfully inserted it. Likely root cause is in `PanelEntityType`'s `TypedStorage`
dispatch or `EntityMap::get` not finding the panel by its UUID-derived ID.

## Steps to Fix

1. Add debug logging to `contains_uuid` and `resolve_uuid_string` to trace the
   exact UUID being looked up vs. what is stored.
2. Verify `PanelId::from_uuid(uuid)` round-trips correctly with the ID stored by
   `PanelBuilder::build`.
3. Check whether `TypedStorage::typed_map` for `PanelEntityType` returns the correct
   `EntityMap`.
4. Fix the root cause and ensure both tests pass.
