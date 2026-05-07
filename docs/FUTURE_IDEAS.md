# Future Ideas and Design Notes

Updated on: Wed May  6 21:04:03 2026

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
[IDEA-080]: work-item/idea/IDEA-080.md
[IDEA-101]: work-item/idea/IDEA-101.md
