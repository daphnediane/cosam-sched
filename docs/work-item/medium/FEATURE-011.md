# CRDT Abstraction Layer Design

## Summary

Design the abstraction layer between the entity/field system and the CRDT backend.

## Status

In Progress

## Priority

Medium

## Description

Before integrating a specific CRDT library, define the abstraction boundary so
the entity system doesn't depend directly on CRDT internals.

### Design Goals

- Entity reads and writes go through the field system as before
- Underneath, field writes produce CRDT operations instead of direct mutation
- The CRDT layer handles merge, conflict detection, and causal ordering
- The abstraction should support swapping backends (automerge, crdts, etc.)

### Key Abstractions

- **CrdtBackend** — trait wrapping create, apply-op, merge, serialize/deserialize
- **CrdtOp** — field-level operation (set register, add to set, remove from set)
- **ActorId** — unique identifier for each editing peer

### Field Type → CRDT Type Mapping

- Scalar fields (`String`, `Integer`, `Float`, `Boolean`, `DateTime`,
  `Duration`) → LWW-Register
- Relationship fields (`Vec<TypedId>`, e.g., `presenter_ids`, `group_ids`) →
  OR-Set. Concurrent add+remove → add wins. The typed ID of the target entity
  is the element identity within the set. No separate edge UUIDs needed.
- `None` → tombstone / deletion marker

EdgeMaps (bidirectional reverse indexes) are not CRDT-backed — they are rebuilt
from primary CRDT state on load and maintained incrementally via entity hooks.

### Library Decision

**automerge** for all field types. Spike evaluated a two-library split
(`crdts` + `automerge`) but settled on single-library for simpler sync,
one serialisation format, and OR-Set-equivalent semantics from automerge's
List type. See `docs/crdt-design.md` for full rationale.

## Progress

### Spike complete (META-027 Step 2)

Library evaluation done in `crates/crdt-spike` (12 tests passing).
Design findings written to `docs/crdt-design.md`.

**Field type → CRDT type mapping confirmed:**

- Structured scalars (String, Integer, Boolean, UUID, DateTime, Duration) →
  `crdts::LWWReg<V, (u64, ActorId)>`. Marker is `(logical_time, actor_id)`;
  actor ID breaks ties deterministically for concurrent writes.
- Relationship sets (`presenter_ids`, `event_room_ids`, etc.) →
  `crdts::Orswot<Uuid, ActorId>`. Add-wins over unobserved-concurrent-remove.
- Prose fields (`description`, `note`, `notes_non_printing`, `workshop_notes`,
  `av_notes`) → `automerge::Text` (RGA). LWW is insufficient: a concurrent
  global find-replace + independent paragraph edit at a different position
  would silently discard one writer's entire change under LWW; RGA preserves
  both at character granularity.

**Library decision:** two-library approach —
`crdts` for structured/set fields, `automerge` for prose.

**Design decisions settled** (see `docs/crdt-design.md` for full detail):

- **Library**: `automerge` for everything (single-library approach)
- **Entity presence**: no OR-Set needed; soft-delete only, entities are never
  hard-deleted from the document; deleted state derived from field values
- **Scalars**: `put()` LWW — acceptable given soft-delete recoverability and
  rare true concurrency on scheduling fields
- **Relationship sets** (`presenter_ids` etc.): automerge `List` (RGA);
  OR-Set-equivalent add-wins semantics; deduplicate UUIDs on read
- **Prose fields**: automerge `Text` (RGA); character-level concurrent edits
- **Working format**: automerge binary; JSON is export-only (widgets, archive)
- **Actor identity**: per-device persistent UUID; no central server needed;
  stored via `directories` crate at OS-conventional config path
  (`com.CosplayAmerica.cosam-sched`); display name written into document's
  `actors/` map and propagated via normal merge
- **Logical clock**: managed internally by automerge; no manual clock needed
- **Actor priority**: future option via actor ID ordering for role-based LWW
  tiebreaking

**Remaining open questions** (decide during implementation):

1. Document structure: one automerge document per schedule vs per entity
   (single document is almost certainly correct at this scale)
2. Sync wire format: full-state merge to start; op-log streaming later
   (FEATURE-013)
3. `FieldValue::Text(String)` vs reuse `FieldValue::Str` for prose fields

### Next steps

- Decide document structure (one doc per schedule)
- Define `CrdtBackend` trait and `CrdtOp` enum with automerge implementation
- Proof-of-concept: entity read/write through the abstraction

## Acceptance Criteria

- Written design document with trait definitions
- Proof-of-concept with at least one backend
- Entity read/write works through the abstraction without API changes
