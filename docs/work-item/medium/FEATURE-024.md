# Change Tracking and Merge Operations

## Summary

Implement change tracking, diff computation, and merge for CRDT documents.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-023: CRDT-backed entity storage

## Description

Build on the CRDT storage (FEATURE-023) to provide:

- Change tracking between document states
- Diff computation showing what changed between two versions
- Merge operations for combining concurrent changes from multiple actors
- Conflict surfacing for concurrent scalar edits (LWW with visibility)

## Acceptance Criteria

- Can compute diff between two document versions
- Merge of concurrent non-conflicting changes succeeds
- Scalar conflicts are surfaced to the user
- Unit tests for merge scenarios from docs/crdt-design.md
