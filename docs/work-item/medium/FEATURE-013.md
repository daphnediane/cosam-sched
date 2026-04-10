# Change Tracking and Merge Operations

## Summary

Implement change tracking, diff computation, and merge for CRDT documents.

## Status

Open

## Priority

Medium

## Description

Build on the CRDT storage (FEATURE-012) to provide:

### Change Tracking

- Track which fields changed since last save/sync
- Per-entity dirty flags
- Change timestamps with causal ordering (vector clocks or Lamport timestamps)

### Diff Computation

- Compute the set of changes between two document states
- Express diffs as a list of field-level operations
- Support diffing for UI display ("what changed since last sync")

### Merge Operations

- Merge two divergent document states into a consistent result
- Surface conflicts where the same field was edited by different peers
- Automatic resolution for non-conflicting concurrent edits
- Conflict metadata available for UI display (FEATURE-024)

### Integration With Edit Commands

- Each `EditCommand` (FEATURE-010) generates corresponding CRDT operations
- Remote CRDT operations can be replayed as synthetic edit commands
- Undo/redo interacts correctly with merged state

## Acceptance Criteria

- Changes are tracked at field granularity
- Diffs between two states are accurate and complete
- Non-conflicting merges produce correct results
- Conflicting merges are detected and surfaced
- Unit tests for merge scenarios (concurrent add, concurrent edit, delete+edit)
