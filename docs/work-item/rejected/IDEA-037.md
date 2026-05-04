# Read-Only Entity Resolution (Lookup Without Creation)

## Summary

Add read-only `lookup_*` variants to entity resolution that take `&EntityStorage`
instead of `&mut EntityStorage`.

## Status

Superseded

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

## Superseded By

The core concept (mutability-based API split) is already implemented in
`crates/schedule-core/src/query/lookup.rs`:

- `lookup<E: EntityScannable>(schedule: &Schedule, ...)` — read-only
- `lookup_or_create<E: EntityCreatable>(schedule: &mut Schedule, ...)` — find-or-create
- Convenience helpers: `lookup_single`, `lookup_list`, `lookup_or_create_single`, `lookup_or_create_list`

Differences from IDEA-037:

- Uses `&Schedule` instead of `&EntityStorage` (EntityStorage was eliminated)
- Naming: `lookup`/`lookup_or_create` instead of `lookup_string`/`resolve_string`
- Return type: `Result<Vec<EntityId>, LookupError>` instead of `Option<EntityId>`
