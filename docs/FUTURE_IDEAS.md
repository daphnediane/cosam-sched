# Future Ideas and Design Notes

Updated on: Mon Apr 13 21:40:32 2026

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

### [IDEA-046] IDEA-046: Generic FieldValue to FieldValue conversion system

**Summary:** Add generic support for arbitrary FieldValue to FieldValue conversions with customizable conversion strategies, including lookup-only and create-capable variants

**Description:** Currently the `resolve_field_value` and `resolve_field_values` methods on `EntityResolver` only handle converting a `FieldValue` to `Option<EntityType::Id>` or `Vec<EntityType::Id>`. This is limiting for cases where we need to convert between different `FieldValue` kinds before final entity resolution.

A more flexible system would support generic `FieldValue` to `FieldValue` conversions with customizable strategies. This would enable:

* **Tagged presenter support**: Conversions like `"P:Name"` → `Presenter` entity with rank, or `"G:Group=Member"` → group membership relationships
* **Custom conversion pipelines**: Chain multiple conversions (e.g., string → tagged string → entity reference)
* **Type-specific conversion logic**: Each entity type can define its own conversion rules

---

## Closed Ideas

* [IDEA-040] (Completed) The presenter-group mutation helpers in `schedule/mod.rs` still embed business
logic directly rather than delegating to `PresenterEntityType` methods.  This
violates the thin-adapter principle from `docs/field-system.md`.

---

## Placeholders

Rename `IDEA-###.md` to another prefix to promote an idea.

*No IDEA placeholders.*

Use `perl scripts/work-item-update.pl --create IDEA` to add new stubs.

---

[IDEA-033]: work-item/idea/IDEA-033.md
[IDEA-039]: work-item/idea/IDEA-039.md
[IDEA-040]: work-item/done/IDEA-040.md
[IDEA-043]: work-item/idea/IDEA-043.md
[IDEA-046]: work-item/idea/IDEA-046.md
