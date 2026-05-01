# BUGFIX-072: FIELD_MEMBERS / FIELD_GROUPS near/far confusion in presenter.rs and panel.rs

## Summary

Several homogeneous-edge queries on the presenter member/group relationship
use the near/far field pair swapped from what their docs and field names
advertise. Introduce `FIELD_*_NEAR` / `FIELD_*_FAR` aliases to make the
intent explicit at each call site and fix the inverted queries.

## Status

Done

## Priority

Medium

## Description

The edge storage convention is **"field name = far side of the edge"**, as
documented in `crates/schedule-core/src/edge_map.rs:35-40`:

```text
map[member_uuid][FIELD_GROUPS]  = [(FIELD_MEMBERS, group_uuid), ...]
map[group_uuid][FIELD_MEMBERS]  = [(FIELD_GROUPS,  member_uuid), ...]
```

So under this convention:

- `connected_entities((id, FIELD_MEMBERS), &FIELD_GROUPS)` returns the
  **members** of `id` (`id` acting as a group).
- `connected_entities((id, FIELD_GROUPS), &FIELD_MEMBERS)` returns the
  **groups** that `id` belongs to (`id` acting as a member).

Because the two fields look symmetric and their roles depend on which
side is the near node, several sites in the codebase use the swapped
pair and therefore compute the wrong set. The bugs are latent because
most call sites *union* both directions and come out with a consistent
(if mis-labelled) result, but a few sites rely on the specific direction.

### Confirmed swapped sites

1. **`FIELD_INCLUSIVE_GROUPS`** in
   `crates/schedule-core/src/presenter.rs` (~L820-839).
   Doc: "Inclusive groups — all groups this presenter belongs to,
   transitively." Code:

   ```rust
   sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(
       FieldNodeId::new(id, &FIELD_MEMBERS),
       &FIELD_GROUPS,
   )
   ```

   This actually returns transitive **members** of `id`, not its groups.

2. **`FIELD_INCLUSIVE_MEMBERS`** in the same file (~L842-862).
   Doc: "Inclusive members — all members of this group, transitively."
   Code uses `(id, FIELD_GROUPS)` toward `&FIELD_MEMBERS`, which actually
   returns transitive **groups** of `id`.

   `FIELD_INCLUSIVE_GROUPS` and `FIELD_INCLUSIVE_MEMBERS` are simply
   swapped with each other.

3. **`panel.rs` `always_grouped` branch** (~L1128-1148). Comment says
   "This member always appears under their group's name" — implying we
   want the groups the presenter belongs to — but the code queries
   `(presenter_id, FIELD_MEMBERS)` toward `&FIELD_GROUPS`, which returns
   the presenter's **members** instead of its groups.

### Likely correct but fragile sites

1. **`FIELD_INCLUSIVE_PANELS`** in `presenter.rs` (~L894-944). The two
   internal comments ("Panels of all transitive groups (upward)" and
   "Panels of all transitive members (downward)") are each paired with
   the opposite-direction `inclusive_edges` call. The union is the same
   set either way, but the comments are misleading.

2. **Homogeneous-edge tests** in `crates/schedule-core/src/schedule.rs`
   (`inclusive_edges_from_transitive_closure`,
   `inclusive_edges_to_transitive_closure`, and the helpers around them)
   name the variable at `FIELD_MEMBERS` "member_id" and the one at
   `FIELD_GROUPS` "group_id", which is the opposite of the edge-map
   convention. The tests still pass because they assert symmetric round
   trips, not absolute role labels, but the naming misleads readers and
   has already produced real bugs downstream.

3. **`inclusive_edges` doc comment** at `schedule.rs:802-808` claims
   "`inclusive_edges(alice_members, &FIELD_GROUPS)` returns all groups
   alice belongs to". Under the documented edge-map convention that call
   actually returns alice's members.

## How Found

During the `schedule-macro` migration (FEATURE-071) I attempted to "fix"
`FIELD_IS_GROUP` based on a misreading of the convention. Re-examining
`edge_map.rs` and working through the edge-map docstring confirmed the
original `FIELD_IS_GROUP` code was correct and pointed out the above
sites as the real problems.

## Steps to Fix

### API refactoring (to enable correct usage)

Instead of aliases, refactored the edge API to make correct usage more ergonomic:

1. Changed `edge_add` and `edge_remove` signatures to accept multiple targets via iterator:
   - `edge_add(near, edge, far_nodes: impl IntoIterator<Item = impl DynamicEntityId>)`
   - `edge_remove(near, edge, far_nodes: impl IntoIterator<Item = impl DynamicEntityId>)`
   - Both now return `Vec<NonNilUuid>` of actual changes made

2. Introduced explicit `FullEdge` constants for homogeneous edges:
   - `pub const EDGE_GROUPS: FullEdge` - from FIELD_GROUPS to FIELD_MEMBERS
   - `pub const EDGE_MEMBERS: FullEdge` - from FIELD_MEMBERS to FIELD_GROUPS

3. Made `FIELD_GROUPS` and `FIELD_MEMBERS` private (`pub(self)`) to prevent accidental misuse

### Fixes

- `FIELD_INCLUSIVE_GROUPS`: use `EDGE_GROUPS` constant (was incorrectly using FIELD_MEMBERS→FIELD_GROUPS)
- `FIELD_INCLUSIVE_MEMBERS`: use `EDGE_MEMBERS` constant (was incorrectly using FIELD_GROUPS→FIELD_MEMBERS)
- `panel.rs always_grouped branch`: use `EDGE_GROUPS` to fetch presenter's groups, then `EDGE_MEMBERS` to enumerate siblings
- `FIELD_INCLUSIVE_PANELS`: updated to use `EDGE_GROUPS`/`EDGE_MEMBERS` constants with correct direction
- All integration tests updated to use new API and correct edge constants

## Testing

Regression tests added (see Implementation section for details):

- Basic unit tests in `presenter.rs` for direct edge relationships
- Integration tests in `tests/edges_integration.rs` for transitive closure and symmetry
- All 400 lib tests pass. All 5 integration tests pass.

## Implementation

**Fixed:** All homogeneous-edge queries now use explicit `FullEdge` constants:

- `presenter.rs`: `FIELD_INCLUSIVE_GROUPS`, `FIELD_INCLUSIVE_MEMBERS`, `FIELD_IS_GROUP` use `EDGE_GROUPS`/`EDGE_MEMBERS`
- `panel.rs`: `FIELD_INCLUSIVE_PRESENTERS` and `always_grouped` branch migrated to `EDGE_GROUPS`/`EDGE_MEMBERS`
- `EDGE_GROUPS` and `EDGE_MEMBERS` are now `pub` for cross-module use

**Documentation:** Added "Homogeneous edges and explicit FullEdge constants" section to `field-system.md` explaining the pattern and why to avoid dynamic `.edge_to()` construction.

Added regression tests in `presenter.rs` (basic unit tests):

- `test_field_inclusive_groups_leaf_member_returns_parent_group` - Verifies FIELD_INCLUSIVE_GROUPS returns the direct parent group
- `test_field_inclusive_members_group_returns_direct_members` - Verifies FIELD_INCLUSIVE_MEMBERS returns direct members of a group
- `test_edge_add_remove_symmetry_groups_and_members` - Verifies add/remove operations are symmetric in both directions

Added integration tests in `tests/edges_integration.rs`:

- `test_inclusive_groups_transitive_closure` - Verifies FIELD_INCLUSIVE_GROUPS returns both direct and transitive parent groups
- `test_inclusive_members_transitive_closure` - Verifies FIELD_INCLUSIVE_MEMBERS returns direct + transitive members (nested groups)
- `test_edge_add_multiple_targets_symmetry` - Verifies adding/removing multiple targets maintains symmetry
- `test_edge_add_from_group_side_symmetry` - Verifies edge_add from group side (EDGE_MEMBERS) works correctly
- `test_edge_add_multiple_members_from_group_side` - Verifies adding multiple members from group side

- All 400 lib tests pass. All 5 integration tests pass.