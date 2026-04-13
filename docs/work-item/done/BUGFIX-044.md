# Panel EntityResolver fails for EntityIdentifier and prefixed UUID string

## Summary

`PanelEntityType::resolve_field_value` returns errors for `EntityIdentifier` and prefixed UUID string inputs even when the panel exists in storage.

## Status

Completed

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

## Root Cause

PanelEntityType had a custom `resolve_field_value` implementation that only handled
`FieldValue::NonNilUuid` and returned `LookupError::Empty` for all other types,
including `FieldValue::EntityIdentifier` and `FieldValue::String`. This custom
implementation was incomplete and out of date compared to the default
`EntityResolver` implementation.

## Resolution

Removed the custom `PanelEntityType::resolve_field_value` implementation entirely.
The default `EntityResolver` implementation correctly handles:

- `FieldValue::EntityIdentifier` through `resolve_next_field_value`
- `FieldValue::String` (both bare and prefixed UUIDs) through `resolve_string` → `resolve_uuid_string`
- `FieldValue::NonNilUuid` through `contains_uuid`

Both failing tests now pass:

- `resolve_field_value_entity_identifier`
- `resolve_field_value_prefixed_uuid_string`

## Verification

All entity resolution tests pass, confirming that PanelEntityType now uses the
standard EntityResolver path and works consistently with other entity types.
