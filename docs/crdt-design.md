# CRDT Design for Offline Collaborative Editing

**Status:** Design settled; ready for FEATURE-011 implementation
**Spike crate:** `crates/crdt-spike`
**Related work items:** META-027, FEATURE-011, FEATURE-012, FEATURE-013

---

## Problem Statement

The scheduling application is used by multiple operators, sometimes offline
(e.g., at the convention itself where network access is unreliable). When two
operators edit the same entity and later sync, changes must merge without
silent data loss.

---

## Settled Decisions

### Single library: automerge

Use `automerge` for all field types. The initial spike evaluated a two-library
split (`crdts` for structured fields, `automerge` for prose), but the
single-library approach is preferred for:

- One document model — one `fork()`/`merge()` call syncs everything
- One serialisation format to maintain
- automerge's List type gives OR-Set-equivalent add-wins semantics for
  relationship sets without needing `crdts::Orswot`
- The `MVReg` concern (surfacing scalar conflicts) is resolved by soft-delete
  semantics and the existing scheduling conflict display (see below)

### Field type → CRDT type mapping

| Field category | automerge type | Notes |
|---|---|---|
| Structured scalars (`name`, `rank`, booleans, timestamps, duration) | Scalar via `put()` | LWW; automerge manages clock internally |
| UUID references (`event_room_id`, `panel_type_id`) | Scalar via `put()` | LWW |
| Relationship sets (`presenter_ids`, `event_room_ids`) | `ObjType::List` | RGA list; OR-Set-equivalent add-wins semantics; deduplicate UUIDs on read |
| Prose fields (`description`, `note`, `notes_non_printing`, `workshop_notes`, `av_notes`) | `ObjType::Text` | Character-level RGA; concurrent edits at different positions both survive |

### Soft-delete semantics eliminate the OR-Set entity presence concern

There are no hard deletes. An entity UUID key, once created in the automerge
document, is never removed. "Deleted" state is derived from field values:

- **Panel**: soft-deleted when `name`, `start_time`, and `event_room_id` are
  all null/None
- **Presenter**, **EventRoom**, **PanelType**: soft-deleted when no panel
  references them (derived from EdgeMap reverse index — not stored in the CRDT)

Consequences:

- No OR-Set for entity presence is needed. The entity map is a grow-only
  structure; presence = key exists in the document.
- The worst-case LWW outcome is a field becoming null, which makes an entity
  appear soft-deleted. This is detectable (panel shows as "incomplete" in the
  UI), visible, and reversible — no data is permanently lost.
- The concurrent "A deletes entity, B edits fields" hazard from the `crdts`
  spike does not apply.

### JSON is for export only

The working document format is the automerge binary (`save()`/`load()`). JSON
is produced as a static export for:

- Widget/HTML output (`cosam-convert`)
- Spreadsheet export
- Archival snapshots of finalised schedules

`AutoSerde` or a custom document walk produces the current field values as
plain JSON. This export does not carry CRDT metadata and cannot be used to
restore a replica — it is a one-way snapshot.

### Room double-booking is a display concern, not a CRDT concern

Two panels assigned to the same room at the same time is a valid in-progress
state (operators deciding between options). The existing scheduling conflict
detection and display handles this. The CRDT faithfully records both
assignments; conflict resolution is a UI concern.

### LWW for scalars is acceptable

Silent LWW resolution on scalar fields (e.g., two operators set `start_time`
to different values concurrently) is acceptable because:

- Scheduling edits to timing/room fields are typically made by one designated
  operator; true concurrent conflicts are rare
- When they do occur, operators likely make the same edit (the conflict
  resolves to the intended value)
- The soft-delete property means no data is permanently lost; a LWW loss on
  a critical field (e.g., `start_time` → null) makes the panel visibly
  incomplete rather than silently wrong
- Actor priority (see below) provides a future escalation path

### Actor identity: per-device persistent UUID

Each device generates a UUID on first launch and persists it in the
OS-conventional application config directory. No central server is needed for
ID assignment.

Config path via the `directories` crate
(`ProjectDirs::from("com", "CosplayAmerica", "cosam-sched")`):

| Platform | Config directory |
|---|---|
| macOS | `~/Library/Application Support/com.CosplayAmerica.cosam-sched/` |
| Windows | `C:\Users\<user>\AppData\Roaming\CosplayAmerica\cosam-sched\` |
| Linux | `~/.config/cosam-sched/` |

Stored as a TOML file (e.g., `device.toml`):

```toml
# Generated on first launch. Do not edit manually.
actor_id = "550e8400-e29b-41d4-a716-446655440000"
display_name = "Daphne"
```

`display_name` is user-provided (set in app preferences) and written into
the document's `actors/` map on first merge so all replicas can attribute
changes to a human name.

If a device is lost or reinstalled, the new installation gets a new actor ID.
Old changes retain their original actor ID in history; no data is lost.

### User identity in the document

The automerge document root contains an `actors/` map:

```
document root
├── panels/       { uuid → panel fields }
├── presenters/   { uuid → presenter fields }
├── event_rooms/  { uuid → room fields }
├── panel_types/  { uuid → type fields }
└── actors/       { actor_id → { display_name } }
```

Multiple actor IDs per human (across devices) all map to the same display
name. The display layer resolves actor ID → human name for change attribution.

### Actor priority for future use

automerge uses actor ID as the LWW tiebreaker for concurrent writes at the
same logical time. If role-based priority is needed, assigning higher actor IDs
to more authoritative roles (e.g., the lead scheduler) makes their concurrent
writes win ties on all fields. This is coarse-grained (per-actor, not
per-field) but sufficient for scheduling use cases.

---

## Open Questions

### Document structure

One automerge document per schedule (all entity types in one document) vs one
document per entity. For a schedule with hundreds of entities, a single
document is almost certainly fine and far simpler. Confirm during FEATURE-011
proof-of-concept.

### Sync wire format

Full-state merge (`save()` / `load()` / `merge()`) is the simple starting
point. Op-log streaming (automerge's sync protocol) is available for future
optimisation. Decide in FEATURE-013 when the sync transport is designed.

### Prose field read path in the field system

`FieldValue` currently has no `Text` variant. Options:

- Add `FieldValue::Text(String)` — prose fields surface as plain `String` to
  the field system; the CRDT layer handles Text object identity transparently
- Keep `FieldValue::Str(String)` and treat Text as a specialised Str — simpler
  but loses the distinction between LWW string and RGA text at the type level

Decide during FEATURE-011 trait design.
