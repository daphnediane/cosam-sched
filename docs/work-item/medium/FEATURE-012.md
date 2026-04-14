# CRDT-backed Entity Storage

## Summary

Replace direct `HashMap` entity storage with CRDT-backed storage.

## Status

Open

## Priority

Medium

## Description

Implement the CRDT abstraction layer (FEATURE-011) with a concrete backend,
replacing the in-memory `HashMap<NonNilUuid, Data>` collections with
CRDT-backed equivalents.

### Implementation Approach

Using the selected CRDT library (likely automerge-rs):

- Each entity is a map in the CRDT document, keyed by UUID
- Each scalar field within an entity is a register (LWW for simple fields)
- Relationship fields (`Vec<TypedId>`) use OR-Sets — no separate edge entities
- The entity registry is a map from UUID to EntityKind
- EdgeMaps (bidirectional reverse indexes) are rebuilt from CRDT state on load
  and maintained incrementally via entity hooks; they are not stored in CRDT

### Per-Field Granularity

- Simple fields (String, Integer, Boolean) use LWW registers
- Relationship fields (e.g., `presenter_ids`, `group_ids`) use OR-sets — typed
  IDs are the element identities; concurrent add+remove → add wins
- Optional fields use LWW with a tombstone for None
- Computed fields remain read-only and are not stored in CRDT

### Write / Read Paths

- **Write**: field mutation → CRDT op → apply to document → update materialized
  view (HashMap + EdgeMap)
- **Read**: read from materialized HashMap (fast, no CRDT traversal)
- `EditCommand.apply()` generates CRDT ops for replication

### Schedule Document Structure

```text
CrdtDocument
├── metadata: Map
├── panels: Map<UUID, Map<field_name, Register|OR-Set>>
├── presenters: Map<UUID, Map<field_name, Register|OR-Set>>
├── event_rooms: Map<UUID, Map<field_name, Register|OR-Set>>
├── hotel_rooms: Map<UUID, Map<field_name, Register>>
├── panel_types: Map<UUID, Map<field_name, Register>>
└── entity_registry: Map<UUID, EntityKind>
```

## Acceptance Criteria

- Entity CRUD operations work through CRDT storage
- Concurrent edits to different fields merge cleanly
- Concurrent edits to the same scalar field resolve via LWW
- Concurrent add/remove on relationship fields resolve via OR-Set
- Storage can be serialized and deserialized
- Unit tests for concurrent edit scenarios
