# Read-Only Entity Resolution (Lookup Without Creation)

## Summary

Add read-only `lookup_*` variants to entity resolution that take `&EntityStorage`
instead of `&mut EntityStorage`.

## Status

Open

## Priority

Low

## Description

Currently entity resolution methods (e.g., presenter name lookup) take
`&mut EntityStorage` because they may auto-create entities during resolution.
Some callers only need lookup (validation, display, read-only queries) and
should not require mutable access.

### Proposed approach

Split by mutability:

- `lookup_string(&EntityStorage, &str) -> Option<EntityId>` — read-only
- `resolve_string(&mut EntityStorage, &str) -> Result<EntityId, FieldError>` — find-or-create

The compiler enforces the distinction naturally — no boolean flag needed.

### When to implement

When a concrete caller needs read-only resolution (e.g., validation, display,
conflict detection). Until then the current `&mut` API is correct for the
import path.
