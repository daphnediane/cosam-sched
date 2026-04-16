# CRDT Design

CRDT-backed storage design for offline collaborative editing. The actual
automerge integration is Phase 4 work; this document records the design
decisions that are baked in from Phase 2 onward.

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

## Field-to-CRDT Mapping by Entity

### PanelType

| Field                                                                                        | CrdtFieldType |
| -------------------------------------------------------------------------------------------- | ------------- |
| `prefix`, `panel_kind`                                                                       | `Scalar`      |
| `hidden`, `is_workshop`, `is_break`, `is_cafe`, `is_room_hours`, `is_timeline`, `is_private` | `Scalar`      |
| `color`, `bw`                                                                                | `Scalar`      |
| `panels` (computed)                                                                          | `Derived`     |

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
| `presenters`, `event_rooms`, `panel_type` (computed)                                              | `Derived`                    |

### Presenter

| Field                                                          | CrdtFieldType |
| -------------------------------------------------------------- | ------------- |
| `name`                                                         | `Scalar`      |
| `bio`                                                          | `Text`        |
| `rank`, `sort_rank`                                            | `Scalar`      |
| `is_explicit_group`, `always_grouped`, `always_shown_in_group` | `Scalar`      |
| `groups`, `members`, `panels` (computed)                       | `Derived`     |

### EventRoom

| Field                              | CrdtFieldType |
| ---------------------------------- | ------------- |
| `room_name`, `long_name`           | `Scalar`      |
| `sort_key`                         | `Scalar`      |
| `hotel_rooms`, `panels` (computed) | `Derived`     |

### HotelRoom

| Field                    | CrdtFieldType |
| ------------------------ | ------------- |
| `hotel_room_name`        | `Scalar`      |
| `event_rooms` (computed) | `Derived`     |

## Merge Semantics

### Scalar fields (LWW)

Last write wins, disambiguated by Lamport timestamp. Concurrent edits to the
same scalar field resolve to the write with the higher timestamp. No
application-level merge logic required.

### Text fields (RGA)

automerge's RGA algorithm merges concurrent text edits character-by-character.
Concurrent insertions at the same position are ordered deterministically.
Applications see the merged result without manual intervention.

### Relationships (edge maps)

Edge maps are OR-Set semantics: concurrent adds and removes are merged such
that an add and a remove of the same edge, if concurrent, resolve to the add
(add wins). This matches the expected behavior for presenter/room assignments.

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

Implementation of soft deletes is deferred to Phase 4 alongside the full
automerge integration.

## Phase Plan

- **Phase 2** (current): `CrdtFieldType` annotations on all field descriptors;
  no automerge code yet
- **Phase 3**: CRDT spike / proof of concept with a single entity type
  (see `crates/crdt-spike/` in v10-try3 for prior exploration)
- **Phase 4**: Full automerge integration in `schedule-core`; replace in-memory
  `HashMap` storage with automerge document storage
- **Phase 8** (future): Multi-device sync, conflict UI, causal history browser
