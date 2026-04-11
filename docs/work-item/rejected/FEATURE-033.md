# Unify entity insertion API and move EdgeIndex maintenance to entity types

## Summary

Unify `add_entity` and `add_edge` into a single insertion path, moving
EdgeIndex (and any per-type cache) maintenance responsibility into each
`EntityType` implementation.

## Status

Superseded

## Priority

Medium

## Description

`EntityStorage` currently has two distinct insertion methods:

- `add_entity<T>` — for node entities (Panel, Presenter, etc.)
- `add_edge<T>` / `add_edge_with_policy<T>` — for edge entities, additionally
  maintaining the `EdgeIndex` for that edge type

This split is artificial. The distinction that edge entities need cache
maintenance is a property of the entity type, not of the caller. The same
applies to `UuidPreference` collision policies, which `add_entity` does not
currently handle but should.

### Desired design

Each `EntityType` implementation provides hooks (or a trait method) called
during insert/remove, giving it the opportunity to update its own caches:

```rust
pub trait EntityTypeHooks: EntityType + TypedStorage {
    /// Called after the entity is inserted into its HashMap.
    fn on_insert(storage: &mut EntityStorage, data: &Self::Data) {}
    /// Called before the entity is removed from its HashMap.
    fn on_remove(storage: &mut EntityStorage, data: &Self::Data) {}
}
```

Edge entity types implement `on_insert`/`on_remove` to update their
`EdgeIndex`. `PresenterToGroupEntityType` would also update any additional
membership caches it maintains. Node entity types use the default no-op.

`EntityStorage::add_entity<T>` becomes the single insertion path for all
entity kinds, with collision policy handled uniformly via `UuidPreference`.
`add_edge` / `add_edge_with_policy` can be kept as convenience wrappers or
removed once all callers are updated.

### Notes

- `PresenterToGroupEntityType` is the primary motivating case: it likely needs
  additional caches (e.g., fast group-membership lookups) beyond the standard
  `EdgeIndex`, and those caches should be managed by that type, not by
  `EntityStorage`.
- The `UuidPreference::Edge { from, to }` variant already supports
  deterministic UUID derivation from endpoints; collision policy for this case
  should follow the same `EdgePolicy` mechanism.

## Acceptance Criteria

- [ ] `EntityTypeHooks` trait (or equivalent) defined with `on_insert` / `on_remove`
- [ ] All edge entity types implement hook to maintain their `EdgeIndex`
- [ ] `PresenterToGroupEntityType` maintains any additional caches via hook
- [ ] `EntityStorage::add_entity` applies collision policy uniformly
- [ ] `add_edge` / `add_edge_with_policy` simplified or removed
- [ ] All existing tests pass; new tests cover hook invocation

## Superseded By

REFACTOR-037 (Virtual edge refactor — EntityStorage reverse indexes and hook
system) implements entity type hooks (`on_insert`/`on_remove`/`on_update`) for
maintaining reverse relationship indexes, which is the core idea here.  The
separate `add_edge` / `add_edge_with_policy` infrastructure is removed entirely
rather than unified, since there are no longer any edge entity types.

## Dependencies

- [FEATURE-008] Schedule container and EntityStorage
- [FEATURE-034] Schedule method delegation to entity types
