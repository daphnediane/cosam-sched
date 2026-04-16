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
deterministic identities like edges and spreadsheet imports).

### Three-struct entity pattern

Each entity type has three hand-written, visible struct declarations:

```text
<Entity>CommonData  (pub)          — user-facing serializable fields only
<Entity>InternalData (pub(crate))  — CommonData + typed UUID + runtime backing
                                     (e.g. time_slot for Panel)
<Entity>Data        (pub)          — export / API view: CommonData + string code
                                     + projected fields + relationship IDs from
                                     edge maps
```

`EntityType` carries **two** associated types:

- `type InternalData` — the `pub(crate)` runtime storage struct; the field
  system operates on this
- `type Data` — the `pub` export/API struct; produced by `export()`

The concrete types are `pub(crate)`, but the associated type slots in the
`pub` trait are also `pub` — external code can use them via the trait alias
(`E::InternalData`, `E::Data`) even though it cannot construct `PanelInternalData`
directly.

### EntityType trait

Core trait for all entity types:

```rust
pub trait EntityType {
    type InternalData: Clone + Send + Sync + fmt::Debug;
    type Data: Clone + Serialize + Deserialize<'_>;
    type Id: TypedId;

    const TYPE_NAME: &'static str;
    fn field_set() -> &'static FieldSet<Self>;
    fn export(internal: &Self::InternalData, schedule: &Schedule) -> Self::Data;
    fn validate(data: &Self::InternalData) -> Vec<ValidationError>;
}
```

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

### UuidPreference

Enum passed to entity builders to control UUID assignment:

- `GenerateNew` *(default)* — generate a v7 UUID; use for new entities with
  no external natural key
- `FromV5 { name: String }` — derive a deterministic v5 UUID from an
  entity-type namespace (supplied by the builder) and a natural-key string
  (e.g. `"GP001"`, presenter name, room name); re-importing the same
  spreadsheet produces the same UUIDs
- `Exact(NonNilUuid)` — use a specific UUID directly; for round-tripping
  serialized entities

Most business logic should not name `UuidPreference` directly — it is
primarily a builder concern.

## Acceptance Criteria

- `EntityId<E>` is Send + Sync + Clone + Copy + Hash + Eq
- EntityKind correctly identifies all entity types
- RuntimeEntityId can round-trip through serialization
- Unit tests for EntityId creation, comparison, and display
