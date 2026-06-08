# REFACTOR-041: Remove EntityKind enum, use type strings directly

## Summary

Replace the `EntityKind` enum with direct use of `EntityType::TYPE_NAME` strings,
following the v10-try3 design. This eliminates the central enum that required
modification for every new entity type.

## Status

Done

## Priority

High

## Description

The `EntityKind` enum in `entity.rs` served two purposes:

1. Tagging `RuntimeEntityId` with the entity type for dynamic dispatch
2. Providing v5 UUID namespaces for deterministic ID generation

Both are now handled without a central enum:

- `RuntimeEntityId` uses `type_name: String` (from `EntityType::TYPE_NAME`)
- `EntityType::uuid_namespace()` provides per-type v5 namespaces directly
  on the trait (returns `&'static Uuid` via internal `LazyLock`)

### Changes Made

**1. `RuntimeEntityId` refactor** (`entity.rs`)

- Uses `{ uuid: NonNilUuid, type_name: String }`
- `from_typed<E>()` constructs from `EntityId<E>`
- `try_as_typed<E>()` for type-safe downcasting (uses `unsafe` `from_uuid` after
  verifying `type_name == E::TYPE_NAME`)
- `from_uuid()` is `unsafe` — caller must ensure UUID/type_name correspondence
- `Display` shows `"TypeName:uuid"`

**2. `EntityId` constructor refactor** (`entity.rs`)

- `from_preference(UuidPreference) -> Self` — primary constructor for new
  entities; resolves `UuidPreference` using `E::UUID_NAMESPACE` (no external
  namespace parameter needed)
- `new(Uuid) -> Option<Self>` — validates bare UUID (rejects nil); for
  deserialization
- `from_uuid(NonNilUuid) -> Self` — **`unsafe`**; caller must ensure the UUID
  belongs to entity type `E`. Code with a UUID→type registry (e.g. `Schedule`)
  can call this safely after verifying the type.

**3. `UuidPreference` simplified** (`entity.rs`)

- `resolve()` method removed — resolution logic moved into
  `EntityId::from_preference()`
- Enum variants unchanged: `GenerateNew`, `FromV5 { name }`, `Exact(NonNilUuid)`

**4. `EntityType` trait** (`entity.rs`)

- Added `fn uuid_namespace() -> &'static Uuid` — per-type v5 namespace computed
  once from `TYPE_NAME` via internal `LazyLock`
- All entity type implementations updated to provide `uuid_namespace()`

**5. `EntityKind` enum removed** (`entity.rs`)

- Enum definition, `uuid_namespace()`, `Display` impl, and serde derives deleted

**6. Tests updated** (`entity.rs`, `field.rs`, `field_set.rs`, `panel_type.rs`)

- `UuidPreference` tests rewritten as `EntityId::from_preference` tests
- `RuntimeEntityId` tests use `unsafe` blocks for `from_uuid`
- `EntityId` tests use `unsafe` blocks for `from_uuid`
- All mock `EntityType` impls updated with `UUID_NAMESPACE`

### Design Rationale

- **Entity types own their identity**: Each type's `TYPE_NAME` is the canonical
  identifier, not a central registry enum
- **Entity types own their namespace**: `uuid_namespace()` on the trait means
  `EntityId::from_preference` can resolve without external parameters
- **Safety via `unsafe`**: `from_uuid` on both `EntityId` and `RuntimeEntityId`
  is `unsafe` because the caller must guarantee the UUID actually identifies an
  entity of the claimed type — only code with a UUID→type registry can verify this
- **Simpler serialization**: RuntimeEntityId serializes as `{uuid, typeName}`
- **No registry needed for type checks**: `try_as_typed` uses compile-time
  `E::TYPE_NAME` comparison
- **Extensibility**: Adding a new entity type requires no changes to shared code

## Acceptance Criteria

- [x] `EntityKind` enum removed from `entity.rs`
- [x] `RuntimeEntityId` uses `type_name: String`
- [x] `UuidPreference::resolve()` removed; replaced by `EntityId::from_preference()`
- [x] `EntityId::from_uuid()` and `RuntimeEntityId::from_uuid()` are `unsafe`
- [x] `EntityType` trait includes `uuid_namespace()`
- [x] All tests pass (121 tests)
- [x] Documentation updated
- [x] No compiler warnings

## Notes

See v10-try3/crates/schedule-field/src/entity.rs for the original target design.
The `uuid_namespace()` approach uses `Uuid::new_v5(&Uuid::NAMESPACE_OID, TYPE_NAME)`
to derive a stable, unique namespace per entity type.
