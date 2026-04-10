# Internal Schedule File Format

## Summary

Define and implement the native save/load format for schedule documents.

## Status

Open

## Priority

Medium

## Description

The internal format is used for saving and loading schedule state, including
CRDT history for sync support.

### Requirements

- Round-trip all entity data, edges, metadata, and CRDT state
- Support incremental saves (only changed data)
- Versioned format with forward-compatibility strategy
- Human-inspectable where practical (JSON or similar for the data layer,
  binary for CRDT state)

### File Structure Options

- **Single file**: JSON + embedded binary CRDT blob
- **Directory**: Separate files for data, CRDT state, and metadata
- **Hybrid**: JSON manifest with CRDT state as a companion file

### Considerations

- Must support multi-year archives (FEATURE-015)
- Must support CRDT merge on load (loading a file from another peer)
- File size: schedules are small (hundreds of entities), so compactness
  is not critical

## Acceptance Criteria

- Save and load round-trip preserves all data
- Format is versioned with a migration path
- CRDT state is preserved for sync
- Loading a peer's file produces correct merged state
- Unit tests for save/load round-trip
