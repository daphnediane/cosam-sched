# Merge Conflict Resolution UI

## Summary

Provide UI for reviewing and resolving merge conflicts after sync.

## Status

Open

## Priority

Low

## Blocked By

- FEATURE-034: Peer-to-peer schedule sync protocol

## Description

When two peers edit the same field concurrently, the CRDT automatically picks
a winner (LWW), but the user should be able to review these decisions and
override them.

## Acceptance Criteria

- User can see which fields had concurrent edits
- User can override CRDT's automatic resolution
- Resolution is recorded as a new edit in the history
