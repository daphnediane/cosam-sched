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

### Candidate Libraries

- **automerge-rs**: Document-oriented CRDT with map/list model. Good fit for
  entity+field data. Built-in conflict tracking and history.
- **crdts** (rust-crdt): Lower-level primitives (LWW registers, OR-sets).
  More control but more scaffolding needed.
- **Custom**: Hand-rolled for perfect fit and maximum learning value; more work.

### Evaluation Criteria

- How well does the data model map to entities with typed fields?
- What is the merge granularity (per-field, per-entity, per-document)?
- How are conflicts surfaced to the user?
- Binary size and dependency weight for desktop apps

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

**Open questions to resolve before trait design:**

1. Actor identity scheme (per-device UUID vs per-user)
2. Logical clock management (`(u64, ActorId)` vs hybrid logical clock)
3. Sync wire format (full state vs op log)
4. `FieldValue::Text` variant vs opaque CRDT handle for prose
5. Whether `MVReg` (multi-value register) is preferable to LWW for
   high-stakes fields like `start_time` (surface conflicts to user)
6. `crdts::Map` vs flat `HashMap` for entity field storage

### Next steps

- Resolve open questions above
- Define `CrdtBackend` trait and `CrdtOp` enum
- Proof-of-concept: route field writes through the trait into `crdts` backend

## Acceptance Criteria

- Written design document with trait definitions
- Proof-of-concept with at least one backend
- Entity read/write works through the abstraction without API changes
