# DirectedEdge: endpoint_uuids() tuple accessor and #[endpoint] attribute rename

## Summary

Deferred design idea: add an `endpoint_uuids()` tuple method to `DirectedEdge`
and optionally rename `#[edge_from]`/`#[edge_to]` to `#[endpoint]`.

## Status

Open

## Priority

Low

## Description

After renaming `from`/`to` → `left`/`right` on `DirectedEdge` (REFACTOR-032),
two further refinements were considered but deferred:

### 1. `endpoint_uuids()` convenience method

Add a default method to `DirectedEdge` returning both endpoint UUIDs as a tuple:

```rust
fn endpoint_uuids(&self) -> (NonNilUuid, NonNilUuid) {
    (self.left_uuid(), self.right_uuid())
}
```

**Rationale:** Useful for generic code that needs both endpoints simultaneously
(e.g. `EdgeIndex::remove(left, right, edge_uuid)`) without naming them
individually.  The homogeneous `(NonNilUuid, NonNilUuid)` type is simpler than
two separate calls in those contexts.

**Not done because:** The existing call sites are few and explicit
`left_uuid()`/`right_uuid()` is already clear.  The tuple variant would only
reduce boilerplate in `EdgeIndex` calls, which are internal implementation
detail anyway.  Can be added cheaply if it proves useful.

### 2. `#[endpoint]` attribute rename

Rename `#[edge_from(Entity)]` / `#[edge_to(Entity)]` macro attributes to
a single `#[endpoint(Entity, side = left)]` / `#[endpoint(Entity, side = right)]`
or a two-attribute scheme `#[left_endpoint(Entity)]` / `#[right_endpoint(Entity)]`.

**Rationale:** `#[edge_from]` / `#[edge_to]` are positional macro markers and
not part of the public API, but they still carry the `from`/`to` directionality
in source code.  Renaming to `endpoint`-based attributes would be fully
consistent with the `left`/`right` method names.

**Not done because:** The attributes are invisible to library consumers (they
only appear in edge-entity struct definitions) and the benefit is purely
cosmetic.  Also `#[edge_from]` / `#[edge_to]` communicate "this is a UUID
pointing at the left/right entity" clearly enough in context.  Low return for
the churn across all 5 edge entity files.

### Decision

Keep `left_id()`/`right_id()`/`left_uuid()`/`right_uuid()` as the primary API.
Keep `#[edge_from]`/`#[edge_to]` attribute names unchanged.
Promote this idea to a `REFACTOR-###` work item if either:

- A third call site needs `endpoint_uuids()`, or
- The attribute names cause confusion for new contributors.
