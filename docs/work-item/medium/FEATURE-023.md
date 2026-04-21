# CRDT-backed Edges via Relationship Lists

## Summary

Store relationships as automerge list fields on a canonical owner entity;
`RawEdgeMap` becomes a derived index rebuilt from these lists.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-022: Automerge-backed Schedule storage

## Description

Move relationship data into the CRDT document by adding a relationship-list
field (`CrdtFieldType::List`) to the canonical owner entity for each
relation. Ownership follows a **panels-outward** rule: panels own outgoing
edges, and entities further from panels own edges that do not point back
toward a panel.

Canonical owners:

| Relation                     | Owner          | Field on owner    |
| ---------------------------- | -------------- | ----------------- |
| Panel ↔ Presenter            | Panel          | `presenter_ids`   |
| Panel ↔ EventRoom            | Panel          | `event_room_ids`  |
| Panel → PanelType            | Panel          | `panel_type_id`   |
| EventRoom ↔ HotelRoom        | EventRoom      | `hotel_room_ids`  |
| Presenter → Presenter group  | Presenter (member) | `group_ids`   |

The public edge API on `Schedule` (`edge_add`, `edge_remove`, `edge_set`,
`edges_from`, `edges_to`) keeps its signature but dispatches to the
canonical owner's relationship list. `RawEdgeMap` stays as a fast in-memory
bidirectional index, rebuilt on `Schedule::load` by scanning all owners'
relationship lists, and maintained incrementally on every edge mutation.

Automerge list semantics give add-wins resolution for concurrent
add/remove on the same relationship, matching `docs/crdt-design.md`.

## Acceptance Criteria

- Each relation above is stored as a `CrdtFieldType::List` field on its
  canonical owner entity.
- `RawEdgeMap` is rebuilt on load from the owner lists; subsequent
  `edge_add` / `edge_remove` calls keep doc and index in sync.
- Existing edge tests pass unchanged.
- Merge test: two docs each add a different presenter UUID to the same
  panel's `presenter_ids`; after merge, both presenters are edges of
  the panel.
- Concurrent add/remove of the same edge resolves add-wins.
