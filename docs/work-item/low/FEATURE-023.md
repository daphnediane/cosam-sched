# Peer-to-Peer Schedule Sync Protocol

## Summary

Define and implement the protocol for synchronizing schedule data between peers.

## Status

Open

## Priority

Low

## Description

Enable multiple users to edit the schedule concurrently and sync their changes
without a central server.

### Sync Model

- Each peer has a full copy of the CRDT document
- Sync is pull-based: a peer requests changes from another peer
- Changes are expressed as CRDT operations (not full document transfers)
- Causal ordering ensures operations are applied consistently

### Transport Options

- **File-based**: Exchange `.schedule` files via shared folder, email, or USB
- **Network**: Direct peer-to-peer over local network (mDNS discovery)
- **Cloud relay**: Optional cloud service for NAT traversal (future)

### Sync Protocol

1. Peer A sends its vector clock / sync state to Peer B
2. Peer B computes the diff and sends missing operations
3. Peer A applies operations and updates its state
4. Roles can be reversed for bidirectional sync

### Conflict Handling

- Non-conflicting concurrent edits merge automatically
- Conflicting edits (same field, different values) are tracked
- Conflict metadata is available for UI resolution (FEATURE-024)

## Acceptance Criteria

- Two peers can sync via file exchange
- Non-conflicting edits merge correctly
- Conflicting edits are detected and preserved
- Sync is idempotent (repeated sync produces same result)
- Integration tests for multi-peer scenarios
