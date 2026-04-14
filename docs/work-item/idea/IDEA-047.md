# IDEA-047: Real-time peer-to-peer sync at convention events

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

Automerge provides a built-in sync protocol (`sync::SyncState`,
`generate_sync_message`, `receive_sync_message`) that efficiently exchanges
only missing changes over any transport. This opens the door to local-network
peer-to-peer sync, but several design questions need answering first.

### Open questions

- **Discovery**: How do devices find each other on the local network? mDNS
  (Bonjour/Avahi), a QR code join flow, a manually entered IP address, or a
  dedicated hub device that others connect to?
- **Transport**: WebSocket, raw TCP, or something higher-level? The hub model
  (one device is the "server", others are clients) is simplest to implement.
- **Platform scope**: Desktop (macOS + Windows) only initially, or also iPad,
  iPhone, Android? Mobile support implies either a web-based UI or native
  apps — a much larger scope than the desktop editor.
- **Conflict with file sync**: When both local-network sync and cloud file sync
  are running, the same changes could arrive via two paths. Automerge's
  idempotent merge handles this correctly, but the UX around "sync status"
  needs thought.
- **Auth / trust**: On a shared convention network, should the app require
  pairing/confirmation before accepting changes from an unknown device?

### Relationship to existing design

- The automerge document and actor identity design (FEATURE-011) is compatible
  with both sync approaches — the CRDT layer is transport-agnostic.
- Implementing the automerge sync protocol is a natural follow-on to
  FEATURE-013 (change tracking and merge operations).
- Mobile support would likely be its own project phase.
