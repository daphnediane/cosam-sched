# Investigate EntityId type-safety holes in `new` and `Exact`

## Summary

`EntityId::new(Uuid)` and `UuidPreference::Exact(NonNilUuid)` both accept a
UUID without verifying it belongs to entity type `E`. Investigate whether these
can be tightened so that `unsafe` search covers all type-membership trust points.

## Status

Open

## Priority

Low

## Description

After REFACTOR-041, `EntityId::from_uuid(NonNilUuid)` is `unsafe` because the
caller must guarantee the UUID identifies an entity of type `E`. However, two
safe constructors have the same implicit trust:

### `EntityId::new(Uuid) -> Option<Self>`

Used by the `Deserialize` impl. The nil check is the only validation — there is
no verification that the UUID belongs to type `E`. Serde's type-directed
deserialization provides the trust boundary (the surrounding schema says "this
field is a `PanelTypeId`"), but nothing prevents a caller from writing
`EntityId::<Panel>::new(some_presenter_uuid)`.

### `UuidPreference::Exact(NonNilUuid)`

Passed to `EntityId::from_preference()`, which constructs the `EntityId<E>`
without checking type membership. Same trust gap — the caller asserts the UUID
is for entity type `E`.

### Possible approaches

- Make `new` unsafe and add an `unsafe` block in the `Deserialize` impl with a
  SAFETY comment documenting the serde trust boundary
- Introduce a `TrustedUuid<E>` newtype that can only be constructed by
  `Schedule` (or similar registry), so safe construction always goes through
  a registry lookup
- Accept the current design — serde deserialization inherently trusts the data
  source, and `Exact` is a builder concern where the caller owns the UUID

### Goal

A grep for `unsafe` should reveal every point where type-membership trust is
assumed. Currently `new` and `from_preference(Exact(...))` are invisible to
that search.

## Related

- REFACTOR-041: Made `from_uuid` unsafe on `EntityId` and `RuntimeEntityId`
- FEATURE-012: Original EntityId implementation
