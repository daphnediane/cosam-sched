# Read-only entity resolution (lookup without creation)

## Summary

Add read-only `lookup_*` variants to `EntityResolver` that take `&EntityStorage` instead of `&mut EntityStorage`.

## Status

Open

## Priority

Low

## Description

Currently `EntityResolver::resolve_string` and the `resolve_field_value`/`resolve_field_values`
methods all take `&mut EntityStorage` because `PresenterEntityType` may auto-create presenters
during resolution. However, some callers only need lookup (validation passes, UI display,
read-only queries) and should not require mutable access.

The v10-try1 codebase handled this with an `always_create: bool` parameter on
`update_or_create_presenter`. A cleaner Rust-idiomatic approach is to split by mutability:

- `lookup_string(&EntityStorage, &str) -> Option<Self::Id>` — read-only, no creation
- `resolve_string(&mut EntityStorage, &str) -> Result<Self::Id, FieldError>` — find-or-create

The `lookup_*` family would mirror the `resolve_*` family but take shared references.
The compiler enforces the distinction naturally — no boolean flag needed.

### When to implement

When a concrete caller needs read-only resolution (e.g., validation, display, conflict
detection). Until then the current `&mut` API is correct for the import path.
