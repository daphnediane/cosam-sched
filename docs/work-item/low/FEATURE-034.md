# Peer-to-Peer Schedule Sync Protocol

## Summary

Define and implement the protocol for synchronizing schedule data between peers.

## Status

Open

## Priority

Low

## Blocked By

- FEATURE-024: Change tracking and merge operations

## Description

Enable multiple users to edit the schedule concurrently and sync their changes
without a central server. Uses automerge's built-in sync protocol.

## Acceptance Criteria

- Two peers can exchange changes and converge
- Sync works over shared folder (baseline) and optionally local network
- Unit tests for sync scenarios
