# CRDT Design

CRDT-backed storage design for offline collaborative editing. The automerge
document is the **single source of truth** inside every `Schedule`; the
in-memory `HashMap` entity store and `RawEdgeMap` are derived caches rebuilt
from the document on load/merge and kept in sync on every write. There is no
"CRDT-off" mode and no optional feature flag.

## Storage Model

```text
Schedule
├── doc: automerge::AutoCommit          ← source of truth
├── entities: HashMap<TypeId, HashMap<Uuid, Box<InternalData>>>  ← cache
├── edges: RawEdgeMap                   ← cache, rebuilt from relationship lists
└── transitive_edge_cache: RefCell<Option<TransitiveEdgeCache>>  ← cache for homogeneous edges
```

Document path layout:

```text
/meta/schedule_id, /meta/created_at, /meta/generator, /meta/version
/entities/{type_name}/{uuid}/{field_name}     (per CrdtFieldType)
/entities/{type_name}/{uuid}/__deleted        (soft delete marker)
/entities/{type_name}/{uuid}/__extra/{key}    (data extra fields; LWW scalar)
```

Every field write flows `FieldValue → automerge op → doc`, then the cache is
refreshed from the new doc state. Every read is from the cache; a merge /
`apply_changes` triggers a full cache rebuild.

## Library Choice

**automerge** (single-library approach). Rationale:

- Mature Rust implementation with good performance
- Supports LWW scalars, RGA text, and list operations natively
- JSON-compatible document model fits the schedule data shape
- Active maintenance and broad adoption

No fallback library is planned. If automerge proves unsuitable, this document
will be updated with an alternative.

## CrdtFieldType

Every `FieldDescriptor` carries a `CrdtFieldType` annotation that controls how
the field maps to automerge storage:

| Variant   | automerge operation          | When to use                                    |
| --------- | ---------------------------- | ---------------------------------------------- |
| `Scalar`  | `put` / `get` (LWW)          | Short strings, numbers, booleans, enums, UUIDs |
| `Text`    | `splice_text` / `text` (RGA) | Long prose: `description`, `bio`, `notes`      |
| `List`    | `insert` / `delete` (list)   | Ordered multi-value fields                     |
| `Derived` | not stored                   | Computed from relationships; lives only in RAM |

Edge relationship fields use `CrdtFieldType::Derived`. Edge ownership direction
is encoded in `EdgeKind` (within `HalfEdgeDescriptor`), not in `CrdtFieldType`.
Edge list storage is managed exclusively by the `crdt::edge` layer
(`list_append_unique`, `list_remove_uuid`, `read_owner_list`).

The `HalfEdgeDescriptor` struct carries edge metadata including
ownership direction (`EdgeKind::Owner { target_field, exclusive_with }` vs
`EdgeKind::Target { source_fields }`). `canonical_owner(near, far)` resolves
ownership by checking each side's `edge_kind()` — constant time, no inventory
traversal required.

## Field-to-CRDT Mapping by Entity

### PanelType

| Field                                                                                        | CrdtFieldType |
| -------------------------------------------------------------------------------------------- | ------------- |
| `prefix`, `panel_kind`                                                                       | `Scalar`      |
| `hidden`, `is_workshop`, `is_break`, `is_cafe`, `is_room_hours`, `is_timeline`, `is_private` | `Scalar`      |
| `color`, `bw`                                                                                | `Scalar`      |
| `panels` (computed)                                                                          | `Derived`     |

### Panel

| Field                                                                                                      | CrdtFieldType                |
| ---------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `uid`, `name`                                                                                              | `Scalar`                     |
| `description`                                                                                              | `Text`                       |
| `note`, `notes_non_printing`, `workshop_notes`, `power_needs`, `av_notes`                                  | `Text`                       |
| `difficulty`, `prereq`, `cost`, `ticket_url`, `simpletix_event`, `simpletix_link`, `alt_panelist`          | `Scalar`                     |
| `sewing_machines`, `is_free`, `is_kids`, `is_full`, `have_ticket_image`, `hide_panelist`                   | `Scalar`                     |
| `capacity`, `seats_sold`, `pre_reg_max`                                                                    | `Scalar`                     |
| `time_slot` (start, duration)                                                                              | `Scalar` (two scalar fields) |
| `credited_presenters` (CRDT owner, target = `Presenter::FIELD_PANELS`, exclusive: `uncredited_presenters`) | `Derived`                    |
| `uncredited_presenters` (CRDT owner, target = `Presenter::FIELD_PANELS`, exclusive: `credited_presenters`) | `Derived`                    |
| `presenters` (derived union of both presenter lists)                                                       | `Derived`                    |
| `event_rooms` (CRDT owner, target = `EventRoom::FIELD_PANELS`)                                             | `Derived`                    |
| `panel_type` (CRDT owner, target = `PanelType::FIELD_PANELS`)                                              | `Derived`                    |

### Presenter

| Field                                                                         | CrdtFieldType |
| ----------------------------------------------------------------------------- | ------------- |
| `name`                                                                        | `Scalar`      |
| `bio`                                                                         | `Text`        |
| `rank`, `sort_index`                                                          | `Scalar`      |
| `is_explicit_group`, `always_grouped`, `always_shown_in_group`                | `Scalar`      |
| `members` (CRDT owner, target = `FIELD_GROUPS`)                               | `Derived`     |
| `groups` (non-owner lookup side)                                              | `Derived`     |
| `panels` (derived union of credited/uncredited panels, non-owner lookup side) | `Derived`     |

### EventRoom

| Field                                                               | CrdtFieldType |
| ------------------------------------------------------------------- | ------------- |
| `room_name`, `long_name`                                            | `Scalar`      |
| `sort_key`                                                          | `Scalar`      |
| `hotel_rooms` (CRDT owner, target = `HotelRoom::FIELD_EVENT_ROOMS`) | `Derived`     |
| `panels` (computed, non-owner lookup side)                          | `Derived`     |

### HotelRoom

| Field                                           | CrdtFieldType |
| ----------------------------------------------- | ------------- |
| `hotel_room_name`                               | `Scalar`      |
| `event_rooms` (computed, non-owner lookup side) | `Derived`     |

## Merge Semantics

### Scalar fields (LWW)

Last write wins, disambiguated by Lamport timestamp. Concurrent edits to the
same scalar field resolve to the write with the higher timestamp. No
application-level merge logic required.

### Text fields (RGA)

automerge's RGA algorithm merges concurrent text edits character-by-character.
Concurrent insertions at the same position are ordered deterministically.
Applications see the merged result without manual intervention.

### Relationships (edges as owner list fields)

Edges are stored in automerge as `ObjType::List` objects on a canonical owner entity,
following a **panels-outward** ownership rule. Edge ownership direction is encoded
in `EdgeKind` (within `HalfEdgeDescriptor`), not in `CrdtFieldType`. All edge fields
use `CrdtFieldType::Derived`. The `EdgeKind::Owner { target_field, exclusive_with }`
variant on the owner side carries the inverse/lookup field reference, enabling
`mirror_entity_fields` to pre-create the right list via `ensure_owner_list` at
entity-insertion time.

| Relation                       | Owner              | Field on owner          |
| ------------------------------ | ------------------ | ----------------------- |
| Panel ↔ Presenter (credited)   | Panel              | `credited_presenters`   |
| Panel ↔ Presenter (uncredited) | Panel              | `uncredited_presenters` |
| Panel ↔ EventRoom              | Panel              | `event_rooms`           |
| Panel → PanelType              | Panel              | `panel_type`            |
| EventRoom ↔ HotelRoom          | EventRoom          | `hotel_rooms`           |
| Presenter → Presenter group    | Presenter (member) | `members`               |

Automerge list operations give add-wins resolution for concurrent edge
mutations: an add and a concurrent remove of the same target UUID resolve to
the add. `RawEdgeMap` is a fast bidirectional in-memory index rebuilt from
these owner lists on load and maintained incrementally on every write.

### `edge_set` incremental mirroring

`Schedule::edge_set` calls `RawEdgeMap::set_neighbors`, which returns
`(added, removed)` UUID diffs. `mirror_edge_set` applies these as incremental
`list_append_unique` / `list_remove_uuid` operations rather than full list
rewrites, preserving concurrent adds from other replicas (add-wins semantics).

### Entity identity

Entity UUIDs are immutable after creation. New entities always get a fresh v7
UUID. Merging two schedules with non-overlapping UUIDs produces a union; merging
schedules that both edited the same UUID (same entity) applies field-level merge
semantics above.

## Soft Deletes

Entities are never hard-deleted from a CRDT document. Instead, a `__deleted: bool`
scalar field marks an entity as removed. Queries and
export filter out deleted entities by default. This preserves causal history and
avoids tombstone conflicts.

Soft deletes are implemented as part of the full automerge integration.

## Conflict Surfacing

`Schedule::conflicts_for<E>(id, field_name)` surfaces every concurrent value
for a scalar field. This is useful for UI conflict resolution when two replicas
edited the same scalar field without observing each other's changes.

- Returns an empty vec when the field is unset
- Returns a single-element vec when there is no conflict (the same value as normal read)
- Returns **all** concurrent writers' values when two or more replicas wrote different scalars

Only scalar fields are supported; derived, text, and list fields yield an empty vec
(they have their own per-character or per-item conflict semantics).

## Save / Load / Merge

### Save

`Schedule::save()` serializes the entire authoritative CRDT document to a compact
byte blob suitable for on-disk persistence or transport. This is a pure pass-through
to `AutoCommit::save`; the in-memory cache contributes nothing.

`Schedule::save_to_file()` serializes to the versioned native file format with a
binary envelope containing magic bytes, format version, metadata JSON, and the
automerge document. Metadata (schedule UUID, creation timestamp, generator, edit
version) is embedded so `load_from_file` can restore it exactly.

### Load

`Schedule::load(bytes)` decodes an automerge document from bytes and rebuilds
a `Schedule` from it: the HashMap cache is rehydrated by replaying every
non-deleted entity through its registered `RegisteredEntityType::rehydrate_fn`.

`Schedule::load_from_file(bytes)` decodes the versioned native file format,
restoring both entity data (including CRDT history) and schedule metadata.

### Merge

`Schedule::merge(&mut other)` merges `other`'s automerge document into this one
and rebuilds the cache to the unified state. Both replicas remain usable — this is
a symmetric join, not a move.

### Change Tracking

`Schedule::get_heads()` returns the change hashes identifying the current head(s)
of the CRDT document. `Schedule::get_changes_since(have_deps)` encodes every
change the doc has observed that is not reachable from `have_deps` as bytes.
`Schedule::apply_changes(changes)` applies a batch of encoded automerge changes
and rebuilds the cache.

These methods enable delta sync for multi-device collaboration: the requester
sends its heads, the responder returns the delta, and the requester applies it.

## Edge CRDT Operations

### Owner List Management

`ensure_owner_list(doc, owner_type, owner_uuid, field_name)` ensures that the empty
list object exists at `owner.field_name` so that concurrent replicas both inherit
the same `ObjId` when they later add entries. This is called by `Schedule::insert`
for every canonical owner field on the inserted entity type.

### Incremental Edge Operations

`list_append_unique(doc, owner_type, owner_uuid, target_type, field_name, target_uuid)`
incrementally appends `target_uuid` to `owner.field_name` if not already present.
Used by `Schedule::edge_add` so concurrent adds from two replicas converge to the
union rather than LWW.

`list_remove_uuid(doc, owner_type, owner_uuid, target_type, field_name, target_uuid)`
incrementally removes every occurrence of `target_uuid` from `owner.field_name`.
Used by `Schedule::edge_remove` so concurrent add-vs-unobserved-remove resolves
add-wins.

### Full List Rewrite

`write_owner_list(doc, owner_type, owner_uuid, target_type, field_name, target_uuids)`
performs a replace-style full-list rewrite. Used only internally when the caller
explicitly wants LWW-on-the-whole-list semantics (reasonable for user-driven bulk
"replace" actions).

### Per-Edge Metadata

`read_edge_meta_bool(doc, owner_type, owner_uuid, field_name, target_uuid, prop_name, default)`
reads a boolean per-edge property from the `{field_name}_meta` map.

`write_edge_meta_bool(doc, owner_type, owner_uuid, field_name, target_uuid, prop_name, value)`
writes a boolean per-edge property into the `{field_name}_meta` map (LWW).

Path: `entities/{owner_type}/{owner_uuid}/{meta_field}/{target_uuid}/{prop_name}`

### Canonical Ownership Resolution

`canonical_owner(near_field, far_field)` resolves CRDT ownership for an edge given
both field descriptors. Each field knows its own `EdgeKind`, so resolution is a
constant-time check on the two supplied fields:

- If `near_field` is `Owner { target_field }` and `target_field` identifies
  `far_field`, `near` is the owner.
- Else if `far_field` is `Owner { target_field }` and `target_field` identifies
  `near_field`, `far` is the owner.
- Otherwise the pair is not a recognized edge.

Taking both fields makes the lookup unambiguous even when multiple edge types exist
between the same pair of entity types (e.g., `credited_presenters` and
`uncredited_presenters` both target `HALF_EDGE_PANELS`).

## Extra Fields (`__extra` Map)

Unknown or not-yet-promoted XLSX columns with plain data values are stored in a
per-entity `__extra` nested automerge map instead of a dedicated scalar field.
This allows arbitrary column names to survive save/load and be CRDT-merged between
users without requiring a code change for each new column.

### Document path

```text
/entities/{type_name}/{uuid}/__extra/{column_name}   → ScalarValue::Str (LWW)
```

Each key is an independent LWW scalar — the same last-write-wins semantics as any
other scalar field. Concurrent edits to different keys do not conflict; concurrent
edits to the same key resolve by Lamport timestamp.

### API

```rust
schedule.read_extra_field(uuid, key)             -> Option<String>
schedule.write_extra_field(uuid, key, value)     -> CrdtResult<()>
schedule.delete_extra_field(uuid, key)           -> CrdtResult<()>
schedule.list_extra_fields(uuid)                 -> Vec<(String, String)>
```

These mirror the `read_edge_meta_bool` / `write_edge_meta_bool` pattern from
`crdt/edge.rs` — per-entity dynamic map access without going through the typed
field system.

### What goes in `__extra` vs. a `FieldDescriptor`

| Data kind                                | Storage                                  |
| ---------------------------------------- | ---------------------------------------- |
| Well-known, schema-declared fields       | `FieldDescriptor` (dedicated CRDT field) |
| Declared-but-lightweight data columns    | `ExtraFieldDescriptor` → `__extra` map   |
| Truly unknown plain-value XLSX columns   | auto-routed to `__extra` map on import   |
| Formula columns (Lstart, Lend, End Time) | in-memory sidecar only (not in CRDT)     |

Formula column values are never stored in `__extra`; they are ephemeral per-session
data held in `ScheduleSidecar` (see §Sidecar below).

When a column earns a proper `FieldDescriptor`, remove its `ExtraFieldDescriptor`.
The `__extra` entry for that key becomes unreachable (old data remains in the doc
but is superseded by the dedicated field path on next write).

## Sidecar (Ephemeral Per-Session Data)

`ScheduleSidecar` holds per-entity data that is ephemeral to the current editing
session and is **never** serialized into the CRDT document or the `.cosam` file.

```rust
pub struct ScheduleSidecar {
    entries: HashMap<NonNilUuid, EntitySidecar>,
}

pub struct EntitySidecar {
    pub origin: Option<EntityOrigin>,
    pub formula_extras: HashMap<String, SidecarFormulaField>,
    pub xlsx_sort_key: Option<(u32, u32)>,
}
```

| Field            | Purpose                                                                               |
| ---------------- | ------------------------------------------------------------------------------------- |
| `origin`         | Where the entity came from: `Xlsx { file_path, sheet, row, time }` or `Editor { at }` |
| `formula_extras` | Formula-cell columns from import (formula string + display value)                     |
| `xlsx_sort_key`  | Original sheet position `(col, row)` used to assign `sort_index`                      |

`ScheduleSidecar` is cleared on `load_from_file` and on `load`. It is NOT cleared on
`save_to_file` — the sidecar must survive an in-session save to support the same-session
`update_xlsx` workflow (import → edit → save → update_xlsx without re-importing).

## ChangeState Tracking

`Schedule` maintains a `change_tracker: HashMap<NonNilUuid, ChangeState>` that records
the mutation state of each entity since the last save.

```rust
pub enum ChangeState { Added, Modified, Deleted, #[default] Unchanged }
```

| Event                            | Resulting state                                |
| -------------------------------- | ---------------------------------------------- |
| Entity created (mirror enabled)  | `Added`                                        |
| Field written on existing entity | `Modified` (unless already `Added`)            |
| Entity removed                   | `Deleted` (always, even if previously `Added`) |
| `save_to_file` succeeds          | tracker cleared (all become `Unchanged`)       |
| `load` / `load_from_file`        | tracker cleared (empty = all `Unchanged`)      |

The "sticky Added" rule: writing a field on a newly created entity (state `Added`) does
not downgrade it to `Modified`. The state only escalates: `Unchanged` → `Modified` → `Deleted`.

`entity_change_state(uuid) -> ChangeState` is the public query; `Unchanged` is returned
for any UUID not in the map (entities that have never been mutated this session).

The tracker is not persisted; it exists only to let the XLSX update-in-place path and
the editor UI know what has changed without a full diff.

## Cache Rehydration

`Schedule::rebuild_cache_from_doc()` discards the in-memory cache and fully
reconstitutes it from the current CRDT document. Used by `load` / `apply_changes` /
`merge`.

Runs under `Schedule::with_mirror_disabled` so replayed entity and edge writes
don't emit redundant changes against the doc we just read from.

The process:

1. Wipe the cache — merge can resurrect soft-deleted UUIDs (add-wins against a
   delete), retarget edges, and generally change which entities exist.
2. Snapshot (type_name, rehydrate_fn, uuids) under an immutable borrow of the doc.
3. Apply each rehydrate with the mirror disabled.
4. Rebuild edges from the doc by iterating every owner field and replaying its
   list into `RawEdgeMap`.
