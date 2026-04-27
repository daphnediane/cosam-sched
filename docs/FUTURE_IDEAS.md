# Future Ideas and Design Notes

Updated on: Mon Apr 27 02:17:53 2026

Open design questions, unexplored alternatives, and deferred ideas.
An IDEA item can be promoted to a work item by renaming it to another prefix
(e.g. `IDEA-033.md` → `REFACTOR-033.md`) while keeping the same number.

## Open Ideas

### [IDEA-036] Per-Membership Edge Flags (always_grouped / always_shown_in_group)

**Summary:** Explore restoring per-membership granularity for `always_grouped` and
`always_shown_in_group` if entity-level flags prove insufficient.

**Description:** Currently `always_grouped` and `always_shown_in_group` are entity-level fields
on `Presenter`, meaning they apply to **all** of a presenter's group memberships
equally. This matches the old `schedule-to-html` Perl implementation behavior.

The old `PresenterToGroup` edge stored these as per-edge flags, allowing a
presenter to be `always_grouped` with respect to Group A but not Group B. This
distinction was not actually used in the spreadsheet data, but the model
supported it.

---

### [IDEA-037] Read-Only Entity Resolution (Lookup Without Creation)

**Summary:** Add read-only `lookup_*` variants to entity resolution that take `&EntityStorage`
instead of `&mut EntityStorage`.

**Description:** Currently entity resolution methods (e.g., presenter name lookup) take
`&mut EntityStorage` because they may auto-create entities during resolution.
Some callers only need lookup (validation, display, read-only queries) and
should not require mutable access.

---

### [IDEA-039] Real-Time Peer-to-Peer Sync at Convention Events

**Summary:** Design and decide on local-network peer-to-peer sync for on-site use at events.

**Description:** The baseline sync mechanism is per-device automerge files in a shared folder
(OneDrive/iCloud Drive/etc.), which works well between sessions. At the
convention itself, internet access may be unreliable, and operators may want
real-time collaboration without waiting for cloud sync.

Automerge provides a built-in sync protocol that efficiently exchanges only
missing changes over any transport.

---

### [IDEA-040] Extended Config File Handling

**Summary:** Extend the `DeviceConfig` / `identity.toml` system with richer identity fields,
per-app metadata, and optional named profiles.

**Description:** The basic config system stores a display name and per-app actor UUIDs. This idea
records extensions deferred from the initial implementation:

---

### [IDEA-042] Investigate EntityId type-safety holes in `new` and `Exact`

**Summary:** `EntityId::new(Uuid)` and `UuidPreference::Exact(NonNilUuid)` both accept a
UUID without verifying it belongs to entity type `E`. Investigate whether these
can be tightened so that `unsafe` search covers all type-membership trust points.

**Description:** After REFACTOR-041, `EntityId::from_uuid(NonNilUuid)` is `unsafe` because the
caller must guarantee the UUID identifies an entity of type `E`. However, two
safe constructors have the same implicit trust:

---

### [IDEA-044] IDEA-044: Reconsider `required` flag on FieldDescriptor

**Summary:** The `required: bool` field on `FieldDescriptor` may conflict with design goals around soft deletion and flexible data structures.

**Description:** ### Current State

`FieldDescriptor` has a `required: bool` field, and `FieldSet` tracks `required_fields()` — fields that must have values. Current tests enforce that `PanelType` fields like `prefix` and `panel_kind` are required.

---

### [IDEA-068] IDEA-068: Add Copy bound to DynamicEntityId trait

**Summary:** Consider adding `Copy` as a super-trait of `DynamicEntityId` so that references
and by-value usage are interchangeable without ownership gymnastics.

**Description:** `DynamicEntityId` (and by extension `DynamicFieldNodeId`, `TypedFieldNodeId`)
currently do not require `Copy`.  The only concrete implementors
(`EntityId<E>`, `RuntimeEntityId`, `FieldNodeId<E>`, `RuntimeFieldNodeId`) are
all `Copy`.

Adding `Copy` as a super-trait would allow callers to use `impl DynamicEntityId`
parameters by value multiple times without borrow/clone workarounds, and would
let `&impl DynamicEntityId` auto-deref to the trait methods without needing
blanket impls for references.

---

### [IDEA-069] IDEA-069: Add EdgeOwner/EdgeTarget variants to CrdtFieldType

**Summary:** Encode CRDT edge ownership direction directly in `CrdtFieldType` instead of
relying solely on `EdgeDescriptor` and `canonical_owner()`.

**Description:** Currently all edge-field descriptors use `CrdtFieldType::Derived`, which the
CRDT mirror layer skips entirely during `mirror_entity_fields`.  Ownership
direction lives only in `EdgeDescriptor`, and mirror functions must call
`canonical_owner()` to resolve it at runtime.

Adding `EdgeOwner` / `EdgeTarget` variants to `CrdtFieldType` would:

* Encode CRDT ownership direction directly in the field descriptor
* Enable mirror functions to derive canonical ownership from field descriptors
  without the `canonical_owner()` lookup
* Potentially allow `mirror_entity_fields` to handle edge list mirroring
  automatically during hydration, eliminating the separate
  `ensure_all_owner_lists_for_type` setup pass

---

## Placeholders

Rename `IDEA-###.md` to another prefix to promote an idea.

*No IDEA placeholders.*

Use `perl scripts/work-item-update.pl --create IDEA` to add new stubs.

---

[IDEA-036]: work-item/idea/IDEA-036.md
[IDEA-037]: work-item/idea/IDEA-037.md
[IDEA-039]: work-item/idea/IDEA-039.md
[IDEA-040]: work-item/idea/IDEA-040.md
[IDEA-042]: work-item/idea/IDEA-042.md
[IDEA-044]: work-item/idea/IDEA-044.md
[IDEA-068]: work-item/idea/IDEA-068.md
[IDEA-069]: work-item/idea/IDEA-069.md
