# Move Group/Membership Business Logic from Schedule into PresenterEntityType

## Summary

The presenter-group mutation helpers in `schedule/mod.rs` still embed business
logic directly rather than delegating to `PresenterEntityType` methods.  This
violates the thin-adapter principle from `docs/field-system.md`.

## Status

Open

## Priority

Low

## Description

During REFACTOR-036/037/038 the five edge entity types were removed and the
`PresenterToGroupEntityType` methods were replaced.  The Schedule helpers were
rewritten to manipulate backing fields and reverse indexes directly:

```rust
// schedule/mod.rs — logic lives here, not in PresenterEntityType
pub fn add_shown_member(&mut self, member: PresenterId, group: PresenterId) -> Result<(), InsertError> {
    let members = self.entities.presenters_by_group.entry(group_uuid).or_default();
    if !members.contains(&member_uuid) { members.push(member_uuid); }
    if let Some(data) = self.entities.presenters.get_mut(&member_uuid) {
        data.always_shown_in_group = true;
        ...
    }
    Ok(())
}
```

The design principle (see `docs/field-system.md`) says `Schedule` should be a
thin coordination layer; business logic should live in `EntityType`
implementations.  The equivalent methods should be:

```rust
// target: PresenterEntityType
pub fn add_member(storage: &mut EntityStorage, member: NonNilUuid, group: NonNilUuid) { ... }
pub fn add_grouped_member(storage: &mut EntityStorage, ...) { ... }
pub fn add_shown_member(storage: &mut EntityStorage, ...) { ... }
pub fn remove_member(storage: &mut EntityStorage, ...) -> bool { ... }
pub fn set_explicit_group(storage: &mut EntityStorage, ...) { ... }  // already exists
pub fn clear_members(storage: &mut EntityStorage, ...) { ... }       // already exists
```

`Schedule` methods would then be one-liners:

```rust
pub fn add_shown_member(&mut self, member: PresenterId, group: PresenterId) -> Result<(), InsertError> {
    PresenterEntityType::add_shown_member(&mut self.entities, member.non_nil_uuid(), group.non_nil_uuid());
    Ok(())
}
```

### Affected methods in `schedule/mod.rs`

- `mark_presenter_group` — calls `PresenterEntityType::set_explicit_group` (already correct)
- `set_is_group` — calls `PresenterEntityType::set_explicit_group` + `clear_members` (already correct)
- `unmark_presenter_group` — inline flag read + `set_explicit_group` call
- `add_member` — business logic inline
- `add_grouped_member` — business logic inline
- `add_shown_member` — business logic inline
- `remove_member` — business logic inline

### Why it was left here

The refactor prioritised correctness and test coverage.  Moving the logic one
level deeper (into `PresenterEntityType`) was deferred to avoid scope creep.
`set_explicit_group` and `clear_members` were already on `PresenterEntityType`
as a partial precedent.

### Macro limitation context

Relationship-affecting `#[write(...)]` closures already receive
`schedule: &mut Schedule` as their first argument, so `&mut EntityStorage` is
available via `schedule.entities`.

However, within such a closure `entity: &mut EntityData` is already a mutable
borrow into the entity's own storage map (e.g., `schedule.entities.panels`).
Rust therefore rejects any call that takes `&mut EntityStorage` as a whole,
even if the callee only touches other fields, because the borrow checker works
at the struct level for function arguments.  The closures work around this by
splitting borrows manually (accessing `schedule.entities.presenters` and
`schedule.entities.panels_by_presenter` as distinct fields).

For `Schedule` methods — which hold no pre-existing entity borrow — delegating
to `EntityType::method(&mut self.entities, ...)` is straightforward with no
technical blocker.

## Notes

Deferred from REFACTOR-036/037/038.  Address as part of a future tidy-up pass
once higher-priority items (FEATURE-010 edit command system, FEATURE-009 query
system) are underway.
