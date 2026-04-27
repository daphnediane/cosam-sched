# FEATURE-068: Add Copy bound to DynamicEntityId trait

## Summary

Add `Copy` as a super-trait of `DynamicEntityId` so that by-value usage of id
parameters is ergonomic without ownership gymnastics.

## Status

Completed

## Priority

Low

## Description

`DynamicEntityId` (and by extension `DynamicFieldNodeId`, `TypedFieldNodeId`)
currently do not require `Copy`.  The only concrete implementors
(`EntityId<E>`, `RuntimeEntityId`, `FieldNodeId<E>`, `RuntimeFieldNodeId`) are
all `Copy`.

Adding `Copy` as a super-trait would allow callers to use `impl DynamicEntityId`
parameters by value multiple times without borrow/clone workarounds, and would
let `&impl DynamicEntityId` auto-deref to the trait methods without needing
blanket impls for references.

### Decision

All four concrete implementors are already `Copy`. Adding the bound prevents
future non-`Copy` implementors, but owned entity handles with drop behavior are
not planned; if needed the bound can be relaxed then.
