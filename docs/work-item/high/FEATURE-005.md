# Core Entity Definitions

## Summary

Define the core domain entity structs using the `EntityFields` derive macro.

## Status

Open

## Priority

High

## Description

Implement entity definitions for the schedule domain model:

- **Panel** — A scheduled event/session with name, description, timing, flags,
  and computed fields for presenters, room, and panel type
- **Presenter** — A person or group that presents at events
- **EventRoom** — A physical or virtual space where events occur
- **HotelRoom** — A hotel room that may host an event room
- **PanelType** — A category/type classification for panels (e.g., "Gaming",
  "Workshop", "Panel")
- **PresenterRank** — Rank/tier for presenters (Guest, Staff, etc.)

Each entity uses `#[derive(EntityFields)]` with appropriate field annotations
for display names, aliases, required fields, and indexable fields.

### Computed Fields

Panel should have computed fields for:

- `presenters` — derived from panel-to-presenter edges
- `event_room` — derived from panel-to-event-room edge
- `panel_type` — derived from panel-to-panel-type edge

## Acceptance Criteria

- All entities compile with the `EntityFields` macro
- Each entity has a corresponding `EntityType` and typed ID wrapper
- Computed fields read correctly from schedule context
- Unit tests for entity creation and field access
