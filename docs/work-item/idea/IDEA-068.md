# IDEA-068: Add Copy bound to DynamicEntityId trait

## Summary

Consider adding `Copy` as a supertrait of `DynamicEntityId` so that references
and by-value usage are interchangeable without ownership gymnastics.

## Status

Open

## Priority

Low

## Description

`DynamicEntityId` (and by extension `DynamicFieldNodeId`, `TypedFieldNodeId`)
currently do not require `Copy`.  The only concrete implementors
(`EntityId<E>`, `RuntimeEntityId`, `FieldNodeId<E>`, `RuntimeFieldNodeId`) are
all `Copy`.

Adding `Copy` as a supertrait would allow callers to use `impl DynamicEntityId`
parameters by value multiple times without borrow/clone workarounds, and would
let `&impl DynamicEntityId` auto-deref to the trait methods without needing
blanket impls for references.

### Alternatives considered

- **Blanket `impl<T: DynamicEntityId> DynamicEntityId for &T`** — works but
  doubles the dispatch surface and complicates trait coherence.
- **Keep current design** — callers extract `entity_uuid()` / `field()` before
  consuming calls; slightly more verbose but safe.

### Open questions

- Would requiring `Copy` prevent any future non-`Copy` implementors we might
  want (e.g., an owned entity handle with drop behavior)?
