# Future Ideas and Design Notes

Updated on: Wed Apr 15 23:40:58 2026

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

### [IDEA-038] Generic FieldValue Conversion System

**Summary:** Add generic support for arbitrary FieldValue-to-FieldValue conversions with
customizable conversion strategies, including lookup-only and create-capable variants.

**Description:** Currently `resolve_field_value` only handles converting a `FieldValue` to entity
IDs. A more flexible system would support generic conversions, enabling:

* **Tagged presenter support**: `"P:Name"` → Presenter with rank
* **Custom conversion pipelines**: Chain multiple conversions
* **Type-specific logic**: Each entity type defines its own rules

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

## Placeholders

Rename `IDEA-###.md` to another prefix to promote an idea.

*No IDEA placeholders.*

Use `perl scripts/work-item-update.pl --create IDEA` to add new stubs.

---

[IDEA-036]: work-item/idea/IDEA-036.md
[IDEA-037]: work-item/idea/IDEA-037.md
[IDEA-038]: work-item/idea/IDEA-038.md
[IDEA-039]: work-item/idea/IDEA-039.md
[IDEA-040]: work-item/idea/IDEA-040.md
