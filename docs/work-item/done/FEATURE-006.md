# UUID-based Identity and Typed ID Wrappers

## Summary

Implement UUID-based entity identity with compile-time type-safe ID wrappers.

## Status

Completed

## Priority

High

## Description

All entities are identified by `uuid::NonNilUuid` (v7 for new entities, v5 for
deterministic edge identities).

### Typed ID Wrappers

Each entity type gets a newtype wrapper (e.g., `PanelId(NonNilUuid)`) providing:

- `TypedId` trait: `non_nil_uuid()`, `uuid()`, `kind()`, `from_uuid()`,
  `try_from_raw_uuid()`
- `Display` impl with entity-prefixed format (e.g., `panel-<uuid>`)
- `Serialize`/`Deserialize` as transparent UUID strings
- `From`/`Into` conversions

### EntityKind Enum

Discriminant enum identifying which entity type a UUID belongs to:
Panel, Presenter, EventRoom, HotelRoom, PanelType, and edge entity variants.

### EntityUUID

Tagged union pairing a `NonNilUuid` with its `EntityKind`, returned by
`Schedule::identify()`.

### Entity Registry

`HashMap<NonNilUuid, EntityKind>` in the Schedule for UUID → kind lookup.

## Acceptance Criteria

- Typed IDs prevent mixing panel UUIDs with presenter UUIDs at compile time
- Serde round-trip for all ID types
- `EntityKind` covers all entity and edge-entity types
- `identify()` resolves any UUID to its typed wrapper
