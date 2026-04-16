# Real-Time Peer-to-Peer Sync at Convention Events

## Summary

Design and decide on local-network peer-to-peer sync for on-site use at events.

## Status

Open

## Priority

Low

## Description

The baseline sync mechanism is per-device automerge files in a shared folder
(OneDrive/iCloud Drive/etc.), which works well between sessions. At the
convention itself, internet access may be unreliable, and operators may want
real-time collaboration without waiting for cloud sync.

Automerge provides a built-in sync protocol that efficiently exchanges only
missing changes over any transport.

### Open questions

- **Discovery**: mDNS, QR code join, manual IP, or hub device?
- **Transport**: WebSocket, raw TCP, or higher-level?
- **Platform scope**: Desktop only initially, or also mobile?
- **Conflict with file sync**: Two sync paths → same changes arrive twice
  (automerge handles idempotently, but UX needs thought)
- **Auth / trust**: Pairing/confirmation on shared network?

### Relationship to existing design

- The automerge document and actor identity design is compatible with both
  sync approaches — the CRDT layer is transport-agnostic
- Mobile support would likely be its own project phase
