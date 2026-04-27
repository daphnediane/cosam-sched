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
/entities/{type_name}/{uuid}/{edge_field}_meta/{target_uuid}/{meta_field}  (per-edge metadata)
```

The `{edge_field}_meta` maps are written only when a per-edge field deviates from
its default. A missing entry is equivalent to the default value. For example, the
Panel ↔ Presenter `credited` flag is absent when `true` (the default) and present
only when explicitly set to `false`.

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

| Variant      | automerge operation          | When to use                                               |
| ------------ | ---------------------------- | --------------------------------------------------------- |
| `Scalar`     | `put` / `get` (LWW)          | Short strings, numbers, booleans, enums, UUIDs            |
| `Text`       | `splice_text` / `text` (RGA) | Long prose: `description`, `bio`, `notes`                 |
| `List`       | `insert` / `delete` (list)   | Ordered multi-value fields                                |
| `Derived`    | not stored                   | Computed from relationships; lives only in RAM            |
| `EdgeOwner`  | not stored here              | CRDT-canonical owner side of an edge relationship;        |
|              |                              | carries `&'static EdgeDescriptor`; `mirror_entity_fields` |
|              |                              | pre-creates the list `ObjId` via `ensure_owner_list`      |
| `EdgeTarget` | not stored here              | Non-owner (inverse/lookup) side; no CRDT storage;         |
|              |                              | a single field may be the target of multiple edges        |

Both `EdgeOwner` and `EdgeTarget` are treated like `Derived` by `crdt::write_field` /
`crdt::read_field`. Edge list storage is managed exclusively by the `edge_crdt` layer
(`list_append_unique`, `list_remove`, `read_list_as_uuids`).  The `EdgeOwner` variant
replaces the former `ensure_all_owner_lists_for_type` global scan: `mirror_entity_fields`
now iterates each entity type's own field descriptors and calls `ensure_owner_list`
only for `EdgeOwner` fields, making the scan O(fields-per-entity) and self-contained.

## Field-to-CRDT Mapping by Entity

### PanelType

| Field                                                                                        | CrdtFieldType |
| -------------------------------------------------------------------------------------------- | ------------- |
| `prefix`, `panel_kind`                                                                       | `Scalar`      |
| `hidden`, `is_workshop`, `is_break`, `is_cafe`, `is_room_hours`, `is_timeline`, `is_private` | `Scalar`      |
| `color`, `bw`                                                                                | `Scalar`      |
| `panels` (computed)                                                                          | `EdgeTarget`  |

### Panel

| Field                                                                                             | CrdtFieldType                |
| ------------------------------------------------------------------------------------------------- | ---------------------------- |
| `uid`, `name`                                                                                     | `Scalar`                     |
| `description`                                                                                     | `Text`                       |
| `note`, `notes_non_printing`, `workshop_notes`, `power_needs`, `av_notes`                         | `Text`                       |
| `difficulty`, `prereq`, `cost`, `ticket_url`, `simpletix_event`, `simpletix_link`, `alt_panelist` | `Scalar`                     |
| `sewing_machines`, `is_free`, `is_kids`, `is_full`, `have_ticket_image`, `hide_panelist`          | `Scalar`                     |
| `capacity`, `seats_sold`, `pre_reg_max`                                                           | `Scalar`                     |
| `time_slot` (start, duration)                                                                     | `Scalar` (two scalar fields) |
| `presenters` (CRDT owner, `EdgeDescriptor` embedded)                                              | `EdgeOwner`                  |
| `event_rooms` (CRDT owner, `EdgeDescriptor` embedded)                                             | `EdgeOwner`                  |
| `panel_type` (CRDT owner, `EdgeDescriptor` embedded)                                              | `EdgeOwner`                  |
| `credited_presenters`, `uncredited_presenters` (computed, no direct CRDT storage)                 | `Derived`                    |
| `presenters_meta` (per-edge metadata map)                                                         | see Per-Edge Metadata below  |

### Presenter

| Field                                                          | CrdtFieldType |
| -------------------------------------------------------------- | ------------- |
| `name`                                                         | `Scalar`      |
| `bio`                                                          | `Text`        |
| `rank`, `sort_rank`                                            | `Scalar`      |
| `is_explicit_group`, `always_grouped`, `always_shown_in_group` | `Scalar`      |
| `members` (CRDT owner, `EdgeDescriptor` embedded)              | `EdgeOwner`   |
| `groups`, `panels` (computed, non-owner lookup side)           | `EdgeTarget`  |

### EventRoom

| Field                                                 | CrdtFieldType |
| ----------------------------------------------------- | ------------- |
| `room_name`, `long_name`                              | `Scalar`      |
| `sort_key`                                            | `Scalar`      |
| `hotel_rooms` (CRDT owner, `EdgeDescriptor` embedded) | `EdgeOwner`   |
| `panels` (computed, non-owner lookup side)            | `EdgeTarget`  |

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
`CrdtFieldType::EdgeOwner(&EDGE_X)` and the inverse lookup side carries
`CrdtFieldType::EdgeTarget`. The embedded `&EdgeDescriptor` tells `mirror_entity_fields`
exactly which list to pre-create via `ensure_owner_list` at entity-insertion time,
replacing the former `ensure_all_owner_lists_for_type` global scan.

| Relation                    | Owner              | Field on owner   |
| --------------------------- | ------------------ | ---------------- |
| Panel ↔ Presenter           | Panel              | `presenter_ids`  |
| Panel ↔ EventRoom           | Panel              | `event_room_ids` |
| Panel → PanelType           | Panel              | `panel_type_id`  |
| EventRoom ↔ HotelRoom       | EventRoom          | `hotel_room_ids` |
| Presenter → Presenter group | Presenter (member) | `group_ids`      |

Automerge list operations give add-wins resolution for concurrent edge
mutations: an add and a concurrent remove of the same target UUID resolve to
the add. `RawEdgeMap` is a fast bidirectional in-memory index rebuilt from
these owner lists on load and maintained incrementally on every write.

### Per-Edge Metadata

Some edges carry additional scalar data beyond membership. These are stored in a
parallel `{edge_field}_meta` automerge `ObjType::Map` on the owning entity, keyed
by target UUID string. Each value is a nested `ObjType::Map` of per-edge scalars
(LWW semantics). A missing entry means the field is at its declared default.

```text
entities/panel/{uuid}/
  presenters              ObjType::List   ← membership list
  presenters_meta         ObjType::Map    ← per-edge data
    "{presenter_uuid}":   ObjType::Map
      "credited": bool    ← LWW scalar; absent ≡ default (true)
```

Removing a presenter leaves the meta entry as a harmless tombstone. The
`EdgeDescriptor.fields` slot declares each per-edge field and its default,
enabling readers to apply the correct default without scanning the doc.

**API:** `Schedule::edge_get_bool<L,R>(l, r, field)` and
`Schedule::edge_set_bool<L,R>(l, r, field, value)` are the typed access points.
The underlying CRDT helpers are `edge_crdt::read_edge_meta_bool` /
`edge_crdt::write_edge_meta_bool`.

**Currently implemented per-edge fields:**

| Edge              | Field      | Default | Meaning                                          |
| ----------------- | ---------- | ------- | ------------------------------------------------ |
| Panel ↔ Presenter | `credited` | `true`  | Whether the presenter appears in `FIELD_CREDITS` |

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
