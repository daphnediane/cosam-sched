# BUGFIX-072: FIELD_MEMBERS / FIELD_GROUPS near/far confusion in presenter.rs and panel.rs

## Summary

Several homogeneous-edge queries on the presenter member/group relationship
use the near/far field pair swapped from what their docs and field names
advertise. Introduce `FIELD_*_NEAR` / `FIELD_*_FAR` aliases to make the
intent explicit at each call site and fix the inverted queries.

## Status

Open

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

### Aliases (clarity pass)

Introduce re-export-style aliases in `presenter.rs` alongside the
existing fields, so each call site reads unambiguously:

```rust
pub use FIELD_MEMBERS as FIELD_MEMBERS_NEAR;     // group's pointer at its members
pub use FIELD_GROUPS  as FIELD_MEMBERS_FAR;      // member's reverse pointer
pub use FIELD_GROUPS  as FIELD_GROUPS_NEAR;      // member's pointer at its groups
pub use FIELD_MEMBERS as FIELD_GROUPS_FAR;       // group's reverse pointer
```

(Exact alias mechanism TBD — may need `static` re-references rather than
`pub use` depending on how inventory handles re-exports.)

Then rewrite the bugged call sites using the aliases so the intent is
self-documenting:

```rust
// members of id (id is the group)
connected_entities((id, &FIELD_MEMBERS_NEAR), &FIELD_MEMBERS_FAR)

// groups id belongs to (id is the member)
connected_entities((id, &FIELD_GROUPS_NEAR), &FIELD_GROUPS_FAR)
```

### Fixes

- `FIELD_INCLUSIVE_GROUPS`: use `(id, &FIELD_GROUPS_NEAR), &FIELD_GROUPS_FAR`.
- `FIELD_INCLUSIVE_MEMBERS`: use `(id, &FIELD_MEMBERS_NEAR), &FIELD_MEMBERS_FAR`.
- `panel.rs always_grouped branch`: use the `GROUPS_NEAR` / `GROUPS_FAR`
  pair to fetch the presenter's groups, then iterate each group's
  `MEMBERS_NEAR` / `MEMBERS_FAR` to enumerate siblings.
- `FIELD_INCLUSIVE_PANELS`: rewrite the inner comments + call-site pair
  so the "upward" branch uses `GROUPS_NEAR/FAR` and the "downward"
  branch uses `MEMBERS_NEAR/FAR`. Behaviour is unchanged.
- `schedule.rs` tests / doc comment: rename `member_id`/`group_id` and
  the `inclusive_edges` example so the labels match the convention.

## Testing

- Add regression tests mirroring `test_is_group_implicit_via_member_edge`:
  - `FIELD_INCLUSIVE_GROUPS` on a leaf member returns its parent group
    (direct + transitive).
  - `FIELD_INCLUSIVE_MEMBERS` on a group returns its direct + transitive
    members.
- For the `always_grouped` branch, add a panel export test where a
  member flagged `always_grouped` is expected to surface under its
  group's display name, with a sibling-credited check.
- Full `cargo test -p schedule-core --lib` must stay green.
