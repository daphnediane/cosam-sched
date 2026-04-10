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
- Each field within an entity is a register (last-writer-wins for simple fields)
- Edges/relationships are stored as sets or maps
- The entity registry is a map from UUID to EntityKind

### Per-Field Granularity

- Simple fields (String, Integer, Boolean) use LWW registers
- List fields (e.g., presenter lists) use OR-sets or CRDT sequences
- Optional fields use LWW with a tombstone for None
- Computed fields remain read-only and are not stored in CRDT

### Schedule Document Structure

```text
CrdtDocument
├── metadata: Map
├── panels: Map<UUID, Map<field_name, Register>>
├── presenters: Map<UUID, Map<field_name, Register>>
├── event_rooms: Map<UUID, Map<field_name, Register>>
├── hotel_rooms: Map<UUID, Map<field_name, Register>>
├── panel_types: Map<UUID, Map<field_name, Register>>
├── edges: Map<UUID, Map<field_name, Register>>
└── entity_registry: Map<UUID, EntityKind>
```

## Acceptance Criteria

- Entity CRUD operations work through CRDT storage
- Concurrent edits to different fields merge cleanly
- Concurrent edits to the same field resolve via LWW
- Storage can be serialized and deserialized
- Unit tests for concurrent edit scenarios
