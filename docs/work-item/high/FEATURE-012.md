# EntityType, EntityId, EntityKind

## Summary

Implement UUID-based entity identity with compile-time type-safe ID wrappers.

## Status

Open

## Priority

High

## Blocked By

- FEATURE-010: FieldValue, error types, CrdtFieldType

## Description

All entities are identified by `uuid::NonNilUuid` (v7 for new entities, v5 for
deterministic identities like edges).

### EntityType trait

Core trait for all entity types, defining:

- `type Data` — the internal data struct
- `TYPE_NAME: &'static str`
- `field_set() -> &'static FieldSet<Self>`
- `validate(&Data) -> Vec<ValidationError>`

### EntityId

`EntityId<E>` — generic typed wrapper around `NonNilUuid` with `PhantomData<E>`.
Provides compile-time type safety for entity references.

### EntityKind

Enum identifying which entity type a UUID belongs to. Variants: Panel, Presenter,
EventRoom, HotelRoom, PanelType.

### RuntimeEntityId

For dynamic identification when the entity type isn't known at compile time.
Pairs a `NonNilUuid` with an `EntityKind`.

### TypedId trait

Uniform interface for all entity IDs backed by `NonNilUuid`. Provides
`uuid()`, `from_uuid()`, Display, serde support.

## Acceptance Criteria

- `EntityId<E>` is Send + Sync + Clone + Copy + Hash + Eq
- EntityKind correctly identifies all entity types
- RuntimeEntityId can round-trip through serialization
- Unit tests for EntityId creation, comparison, and display
