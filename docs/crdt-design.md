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
└── edges: RawEdgeMap                   ← cache, rebuilt from relationship lists
```

Document path layout:

```text
/meta/schedule_id, /meta/created_at, /meta/generator, /meta/version
/entities/{type_name}/{uuid}/{field_name}     (per CrdtFieldType)
/entities/{type_name}/{uuid}/__deleted        (soft delete marker)
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

| Variant                      | automerge operation          | When to use                                               |
| ---------------------------- | ---------------------------- | --------------------------------------------------------- |
| `Scalar`                     | `put` / `get` (LWW)          | Short strings, numbers, booleans, enums, UUIDs            |
| `Text`                       | `splice_text` / `text` (RGA) | Long prose: `description`, `bio`, `notes`                 |
| `List`                       | `insert` / `delete` (list)   | Ordered multi-value fields                                |
| `Derived`                    | not stored                   | Computed from relationships; lives only in RAM            |
| `EdgeOwner { target_field }` | not stored here              | CRDT-canonical owner side of an edge relationship;        |
|                              |                              | carries the inverse/lookup field on the target entity as  |
|                              |                              | `&'static dyn NamedField`; `mirror_entity_fields`         |
|                              |                              | pre-creates the list `ObjId` via `ensure_owner_list`      |
| `EdgeTarget`                 | not stored here              | Non-owner (inverse/lookup) side; no CRDT storage;         |
|                              |                              | a single field may be the target of multiple owner fields |

Both `EdgeOwner` and `EdgeTarget` are treated like `Derived` by `crdt::write_field` /
`crdt::read_field`. Edge list storage is managed exclusively by the `edge_crdt` layer
(`list_append_unique`, `list_remove`, `read_list_as_uuids`).

The `EdgeOwner` variant carries the target field directly, eliminating the
separate `EdgeDescriptor` struct (FEATURE-070): the owner field _is_ the edge
descriptor.  `mirror_entity_fields` iterates each entity type's own field
descriptors and calls `ensure_owner_list` only for `EdgeOwner { .. }` fields,
making the scan O(fields-per-entity) and self-contained.
`canonical_owner(near, far)` is a constant-time check on the two supplied
fields' `crdt_type()` — no inventory traversal required.

## Field-to-CRDT Mapping by Entity

### PanelType

| Field                                                                                        | CrdtFieldType |
| -------------------------------------------------------------------------------------------- | ------------- |
| `prefix`, `panel_kind`                                                                       | `Scalar`      |
| `hidden`, `is_workshop`, `is_break`, `is_cafe`, `is_room_hours`, `is_timeline`, `is_private` | `Scalar`      |
| `color`, `bw`                                                                                | `Scalar`      |
| `panels` (computed)                                                                          | `EdgeTarget`  |

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
| `credited_presenters` (CRDT owner, target = `Presenter::FIELD_PANELS`, exclusive: `uncredited_presenters`) | `EdgeOwner`                  |
| `uncredited_presenters` (CRDT owner, target = `Presenter::FIELD_PANELS`, exclusive: `credited_presenters`) | `EdgeOwner`                  |
| `presenters` (derived union of both presenter lists)                                                       | `Derived`                    |
| `event_rooms` (CRDT owner, target = `EventRoom::FIELD_PANELS`)                                             | `EdgeOwner`                  |
| `panel_type` (CRDT owner, target = `PanelType::FIELD_PANELS`)                                              | `EdgeOwner`                  |

### Presenter

| Field                                                                         | CrdtFieldType |
| ----------------------------------------------------------------------------- | ------------- |
| `name`                                                                        | `Scalar`      |
| `bio`                                                                         | `Text`        |
| `rank`, `sort_rank`                                                           | `Scalar`      |
| `is_explicit_group`, `always_grouped`, `always_shown_in_group`                | `Scalar`      |
| `members` (CRDT owner, target = `FIELD_GROUPS`)                               | `EdgeOwner`   |
| `groups` (non-owner lookup side)                                              | `EdgeTarget`  |
| `panels` (derived union of credited/uncredited panels, non-owner lookup side) | `EdgeTarget`  |

### EventRoom

| Field                                                               | CrdtFieldType |
| ------------------------------------------------------------------- | ------------- |
| `room_name`, `long_name`                                            | `Scalar`      |
| `sort_key`                                                          | `Scalar`      |
| `hotel_rooms` (CRDT owner, target = `HotelRoom::FIELD_EVENT_ROOMS`) | `EdgeOwner`   |
| `panels` (computed, non-owner lookup side)                          | `EdgeTarget`  |

### HotelRoom

| Field                                           | CrdtFieldType |
| ----------------------------------------------- | ------------- |
| `hotel_room_name`                               | `Scalar`      |
| `event_rooms` (computed, non-owner lookup side) | `EdgeTarget`  |

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
following a **panels-outward** ownership rule. The CRDT owner field carries
`CrdtFieldType::EdgeOwner { target_field: &TARGET_FIELD }` and the inverse
lookup side carries `CrdtFieldType::EdgeTarget`. The owner field's own name and
entity type combined with `target_field` is enough for `mirror_entity_fields`
to pre-create the right list via `ensure_owner_list` at entity-insertion time,
replacing the former `ensure_all_owner_lists_for_type` global scan.

| Relation                       | Owner              | Field on owner          |
| ------------------------------ | ------------------ | ----------------------- |
| Panel ↔ Presenter (credited)   | Panel              | `credited_presenters`   |
| Panel ↔ Presenter (uncredited) | Panel              | `uncredited_presenters` |
| Panel ↔ EventRoom              | Panel              | `event_room_ids`        |
| Panel → PanelType              | Panel              | `panel_type_id`         |
| EventRoom ↔ HotelRoom          | EventRoom          | `hotel_room_ids`        |
| Presenter → Presenter group    | Presenter (member) | `group_ids`             |

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

Entities are never hard-deleted from a CRDT document. Instead, a `deleted: bool`
scalar field (CrdtFieldType::Scalar) marks an entity as removed. Queries and
export filter out deleted entities by default. This preserves causal history and
avoids tombstone conflicts.

Soft deletes are implemented alongside the full automerge integration in
Phase 3; no hard-delete code path exists.

## Phase Plan

- **Phase 2** (complete): `CrdtFieldType` annotations on all field descriptors.
- **Phase 3** (current): Authoritative automerge doc under `Schedule`.
  - FEATURE-022 — Automerge-backed Schedule storage (cache mirrors doc).
  - FEATURE-023 — CRDT-backed edges via relationship lists on canonical owners.
  - FEATURE-024 — Change tracking, merge, and conflict surfacing.
- **Phase 4**: File formats (save/load, multi-year archive, XLSX, widget JSON)
  built on top of `Schedule::save` / `load`.
- **Phase 8** (future): Multi-device sync, conflict UI, causal history browser.
