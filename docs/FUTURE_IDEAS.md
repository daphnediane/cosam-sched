# Future Ideas and Design Notes

Updated on: Mon Apr 13 11:26:25 2026

Open design questions, unexplored alternatives, and deferred ideas.
An IDEA item can be promoted to a work item by renaming it to another prefix
(e.g. `IDEA-033.md` → `REFACTOR-033.md`) while keeping the same number.

## Open Ideas

### [IDEA-033] DirectedEdge: endpoint_uuids() tuple accessor and #[endpoint] attribute rename

**Summary:** Deferred design idea: add an `endpoint_uuids()` tuple method to `DirectedEdge`
and optionally rename `#[edge_from]`/`#[edge_to]` to `#[endpoint]`.

**Description:** After renaming `from`/`to` → `left`/`right` on `DirectedEdge` (REFACTOR-032),
two further refinements were considered but deferred:

---

### [IDEA-039] Per-Membership Edge Flags (always_grouped / always_shown_in_group)

**Summary:** Explore restoring per-membership granularity for `always_grouped` and
`always_shown_in_group` if entity-level flags prove insufficient.

**Description:** Currently `always_grouped` and `always_shown_in_group` are entity-level fields
on `Presenter`, meaning they apply to **all** of a presenter's group memberships
equally.  This matches the old `schedule-to-html` Perl implementation behavior.

The old `PresenterToGroup` edge stored these as per-edge flags, allowing a
presenter to be `always_grouped` with respect to Group A but not Group B.  This
distinction was not actually used in the spreadsheet data, but the model
supported it.

---

### [IDEA-040] Move Group/Membership Business Logic from Schedule into PresenterEntityType

**Summary:** The presenter-group mutation helpers in `schedule/mod.rs` still embed business
logic directly rather than delegating to `PresenterEntityType` methods.  This
violates the thin-adapter principle from `docs/field-system.md`.

**Description:** During REFACTOR-036/037/038 the five edge entity types were removed and the
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
implementations.  Per field-system.md principle 9, these methods should take
typed IDs (not `NonNilUuid`) to avoid borrow checker conflicts in computed
field write closures:

```rust
// target: PresenterEntityType
pub fn add_member(storage: &mut EntityStorage, member: PresenterId, group: PresenterId) -> Result<(), InsertError> { ... }
pub fn add_grouped_member(storage: &mut EntityStorage, member: PresenterId, group: PresenterId) -> Result<(), InsertError> { ... }
pub fn add_shown_member(storage: &mut EntityStorage, member: PresenterId, group: PresenterId) -> Result<(), InsertError> { ... }
pub fn remove_member(storage: &mut EntityStorage, member: PresenterId, group: PresenterId) -> bool { ... }
pub fn set_explicit_group(storage: &mut EntityStorage, presenter: PresenterId, value: bool)  // already exists
pub fn clear_members(storage: &mut EntityStorage, group: PresenterId)  // already exists
```

`Schedule` methods would then be one-liners:

```rust
pub fn add_shown_member(&mut self, member: PresenterId, group: PresenterId) -> Result<(), InsertError> {
    PresenterEntityType::add_shown_member(&mut self.entities, member, group)
}
```

---

### [IDEA-043] Read-only entity resolution (lookup without creation)

**Summary:** Add read-only `lookup_*` variants to `EntityResolver` that take `&EntityStorage` instead of `&mut EntityStorage`.

**Description:** Currently `EntityResolver::resolve_string` and the `resolve_field_value`/`resolve_field_values`
methods all take `&mut EntityStorage` because `PresenterEntityType` may auto-create presenters
during resolution. However, some callers only need lookup (validation passes, UI display,
read-only queries) and should not require mutable access.

The v10-try1 codebase handled this with an `always_create: bool` parameter on
`update_or_create_presenter`. A cleaner Rust-idiomatic approach is to split by mutability:

* `lookup_string(&EntityStorage, &str) -> Option<Self::Id>` — read-only, no creation
* `resolve_string(&mut EntityStorage, &str) -> Result<Self::Id, FieldError>` — find-or-create

The `lookup_*` family would mirror the `resolve_*` family but take shared references.
The compiler enforces the distinction naturally — no boolean flag needed.

---

## Closed Ideas

* [IDEA-058] (Placeholder) One-line summary

---

## Placeholders

Rename `IDEA-###.md` to another prefix to promote an idea.

Stub ideas in `docs/work-item/new/` awaiting details:

* [IDEA-058] One-line summary

Use `perl scripts/work-item-update.pl --create IDEA` to add new stubs.

---

[IDEA-033]: work-item/idea/IDEA-033.md
[IDEA-039]: work-item/idea/IDEA-039.md
[IDEA-040]: work-item/idea/IDEA-040.md
[IDEA-043]: work-item/idea/IDEA-043.md
[IDEA-058]: work-item/new/IDEA-058.md
