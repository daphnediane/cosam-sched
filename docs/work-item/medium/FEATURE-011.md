# CRDT Abstraction Layer Design

## Summary

Design the abstraction layer between the entity/field system and the CRDT backend.

## Status

Open

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

- **CrdtDocument** — top-level container wrapping the CRDT state
- **CrdtFieldOp** — a field-level operation (set, delete, increment)
- **CrdtMerge** — merge two documents, producing a combined state
- **CrdtDiff** — compute changes between two document states
- **ActorId** — unique identifier for each editing peer

### Candidate Libraries

- **automerge-rs**: Document-oriented CRDT with map/list model. Good fit for
  entity+field data. Built-in conflict tracking and history.
- **crdts** (rust-crdt): Lower-level primitives (LWW registers, OR-sets).
  More control but more scaffolding needed.

### Evaluation Criteria

- How well does the data model map to entities with typed fields?
- What is the merge granularity (per-field, per-entity, per-document)?
- How are conflicts surfaced to the user?
- Binary size and dependency weight for desktop apps

## Acceptance Criteria

- Written design document with trait definitions
- Proof-of-concept with at least one backend
- Entity read/write works through the abstraction without API changes
