# Future Ideas and Design Notes

Updated on: Tue Apr 14 19:53:00 2026

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

### [IDEA-047] IDEA-047: Real-time peer-to-peer sync at convention events

**Summary:** Design and decide on local-network peer-to-peer sync for on-site use at events.

**Description:** The baseline sync mechanism is per-device automerge files in a shared folder
(OneDrive/iCloud Drive/etc.), which works well between sessions. At the
convention itself, internet access may be unreliable, and operators may want
real-time collaboration without waiting for cloud sync.

Automerge provides a built-in sync protocol (`sync::SyncState`,
`generate_sync_message`, `receive_sync_message`) that efficiently exchanges
only missing changes over any transport. This opens the door to local-network
peer-to-peer sync, but several design questions need answering first.

---

### [IDEA-048] IDEA-048: Extended config file handling

**Summary:** Extend the current `DeviceConfig` / `identity.toml` system with richer
identity fields, per-app metadata, and optional named profiles.

**Description:** The basic config system is already implemented in
`crates/schedule-data/src/crdt/actor.rs` (`DeviceConfig`).  This idea
records the extensions that were deferred from that initial implementation.

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
[IDEA-047]: work-item/idea/IDEA-047.md
[IDEA-048]: work-item/idea/IDEA-048.md
