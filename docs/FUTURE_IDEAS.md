# Future Ideas and Design Notes

Updated on: Sat May  2 09:36:44 2026

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

### [IDEA-077] IDEA-077: Consider list cardinality support for accessor_field_properties

**Summary:** Evaluate whether accessor_field_properties should support add/remove operations for list cardinality fields, and document the work required to implement it.

**Description:** The accessor_field_properties macro currently sets add_fn and remove_fn to None for all fields, with a TODO comment to revisit if list cardinality support is implemented. This idea explores whether accessor fields (computed fields that read/write to underlying storage) should support add/remove operations for list fields.

Currently, add/remove operations are only supported for edge fields through the AddEdge/RemoveEdge variants. Supporting add/remove for accessor list fields would require:

1. **Determine use cases**: Identify which accessor fields with list cardinality should support add/remove operations (e.g., adding to a list field vs. replacing the entire list)

2. **Add new AddFn/RemoveFn variants**: Create new callback variants for accessor field add/remove operations, possibly:
   * AddFn::BareList - for bare function add operations on lists
   * AddFn::ScheduleList - for schedule-aware add operations on lists
   * RemoveFn::BareList - for bare function remove operations on lists
   * RemoveFn::ScheduleList - for schedule-aware remove operations on lists

3. **Implement AddableField/RemovableField for FieldDescriptor**: The FieldDescriptor already implements these traits, but they would need to handle the new list-specific variants

4. **Update conversion support**: The conversion layer (field_value_to_runtime_entity_ids and similar functions) may need updates to handle list add/remove operations for non-edge types. Currently these conversions are primarily designed for entity IDs in edge contexts.

5. **Update accessor_field_properties macro**: Add logic to generate appropriate add_fn/remove_fn based on:
   * Field cardinality (Single vs. List)
   * Whether add/remove operations are desired for the field
   * The type of callback needed (bare vs. schedule)

6. **Update stored_output.rs**: Modify the macro to conditionally generate add_fn/remove_fn instead of always setting them to None

7. **Testing**: Add comprehensive tests for add/remove operations on accessor list fields

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
[IDEA-077]: work-item/idea/IDEA-077.md
