# Future Ideas and Design Notes

Updated on: Wed May  6 17:42:30 2026

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

### [IDEA-080] IDEA-080: Update Schedule from Spreadsheet (Merge Import)

**Summary:** Design for merging a new XLSX import into an existing CRDT-tracked schedule
rather than always starting from a clean slate.

**Description:** The current `import_xlsx` implementation always creates a fresh `Schedule` from
scratch. The convention workflow involves iterative edits to a live spreadsheet,
and it would be useful to re-import without losing manual edits made inside the
editor (e.g., notes, tags, or structural changes applied after the last import).

A merge-based import would:

* Treat the XLSX as the authoritative source for spreadsheet-resident fields
  (name, times, rooms, panelists, costs, etc.)
* Preserve fields set only in the editor that have no spreadsheet column
* Use the existing CRDT merge infrastructure to converge the two states

This is intentionally deferred because:

* It requires careful field-ownership semantics (which fields "belong" to the
  spreadsheet vs. the editor)
* The CRDT merge model needs to be well-established first (FEATURE-022/023)
* A clean-slate import is sufficient for the current workflow

---

### [IDEA-081] IDEA-081: Import Provenance / SourceInfo Sidecar

**Summary:** Track where each entity came from (file, sheet, row) in a UUID-indexed sidecar
structure separate from the CRDT schedule document.

**Description:** During XLSX import every entity has an origin: which file it was read from, which
sheet, and which row. This "source info" is useful for:

* Displaying provenance in the editor ("imported from 2026.xlsx row 42")
* Round-trip update workflows (knowing which entities were xlsx-imported vs.
  created in the editor)
* Future merge-import (IDEA-080): knowing a row's origin helps decide authority

**Why not in the CRDT entity?**

SourceInfo is import-specific and changes every re-import, so storing it as CRDT
fields creates unnecessary history and awkward merge semantics (two replicas that
import the same xlsx agree on source info, but a replica that created an entity
programmatically has no source info, causing spurious conflicts).

**Proposed design: UUID-indexed sidecar:**

A `HashMap<NonNilUuid, SourceInfo>` stored alongside the schedule but outside the
automerge doc. Possibilities:

* In-memory only (lost on save/load — acceptable if only used for import→export
  within one session)
* Serialized into the native file envelope (an extra JSON chunk after the automerge
  blob, indexed by UUID)
* A separate `.provenance` file alongside the `.cosam` file

The sidecar should also cover non-xlsx sources (e.g., "created in editor at time T")
so it generalizes beyond just xlsx.

**Open questions:**

* Does the sidecar need to survive save/load for the current use cases?
* Should SourceInfo be shared with the extra-metadata sidecar (IDEA-082)?
* What format: flat JSON map, or a structured envelope with version/type?

---

### [IDEA-082] IDEA-082: Extended Entity Metadata (Unknown XLSX Columns)

**Summary:** Preserve unknown XLSX columns across import/export without encoding them as
first-class entity fields, and decide how this interacts with CRDT merge.

**Description:** When importing an XLSX spreadsheet, columns that are not recognized by the
importer (e.g., custom convention-specific fields, computed legacy columns like
`Lstart`/`Lend`, or future columns not yet in the schema) are currently silently
dropped. For round-trip fidelity (import → edit → export → same spreadsheet) they
should be preserved.

---

### [IDEA-083] IDEA-083: Separate Hotel Room sheet in XLSX import/export

**Summary:** Add a dedicated `Hotels` sheet to the XLSX format for richer hotel-room metadata.

**Description:** Currently hotel rooms are expressed as a single column (`Hotel Room`) in the Rooms sheet,
limited to one hotel room name per event room. A dedicated `Hotels` sheet would allow richer
metadata (sort key, long name, notes) and cleaner round-trips, mirroring how rooms and panel
types already get their own sheets.

Proposed sheet name: `Hotels` (with `Hotel Rooms` as a fallback alias).

Proposed columns:

* Hotel Room — canonical name (key for linking from the Rooms sheet)
* Sort Key — optional integer for ordering
* Long Name — optional display name

Implementation notes:

* Import: teach `read/rooms.rs` to look for a Hotels sheet and create `HotelRoomEntityType`
  entities from it; the `Hotel Room` column in the Rooms sheet would still be accepted as a
  fallback for files without the separate sheet.
* Export: add `write_hotel_rooms_sheet()` in `xlsx/write/export.rs` alongside the existing
  `write_rooms_sheet()`; suppress the `Hotel Room` column from the Rooms sheet when the
  separate sheet is written.
* The `EDGE_HOTEL_ROOMS` relationship in `event_room.rs` and `columns::room_map::HOTEL_ROOM`
  are the key integration points.

---

### [IDEA-101] IDEA-101: Decide what ScheduleMetadata.version is for

**Summary:** Decide the long-term use of `ScheduleMetadata.version` and update its doc comment and all
call sites accordingly.

**Description:** `ScheduleMetadata` has a `version: u32` field whose doc comment says "Monotonically
increasing edit version counter" but the user says it is a file-format/schema version that
should stay at `0`. There is a discrepancy between the comment and the intended use.

---

## Closed Ideas

* [IDEA-037] (Superseded) Add read-only `lookup_*` variants to entity resolution that take `&EntityStorage`
instead of `&mut EntityStorage`.
* [IDEA-042] (Completed) `EntityId::new(Uuid)` and `UuidPreference::Exact(NonNilUuid)` both accept a
UUID without verifying it belongs to entity type `E`. Investigate whether these
can be tightened so that `unsafe` search covers all type-membership trust points.
* [IDEA-042] (Superseded) `EntityId::new(Uuid)` and `UuidPreference::Exact(NonNilUuid)` both accept a
UUID without verifying it belongs to entity type `E`. Investigate whether these
can be tightened so that `unsafe` search covers all type-membership trust points.

---

## Placeholders

Rename `IDEA-###.md` to another prefix to promote an idea.

*No IDEA placeholders.*

Use `perl scripts/work-item-update.pl --create IDEA` to add new stubs.

---

[IDEA-036]: work-item/idea/IDEA-036.md
[IDEA-037]: work-item/superseded/IDEA-037.md
[IDEA-039]: work-item/idea/IDEA-039.md
[IDEA-040]: work-item/idea/IDEA-040.md
[IDEA-042]: work-item/done/IDEA-042.md
[IDEA-044]: work-item/idea/IDEA-044.md
[IDEA-077]: work-item/idea/IDEA-077.md
[IDEA-080]: work-item/idea/IDEA-080.md
[IDEA-081]: work-item/idea/IDEA-081.md
[IDEA-082]: work-item/idea/IDEA-082.md
[IDEA-083]: work-item/idea/IDEA-083.md
[IDEA-101]: work-item/idea/IDEA-101.md
