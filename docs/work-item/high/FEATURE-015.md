# TimeRange + Panel Entity

## Summary

Port `TimeRange` and implement the Panel entity with stored and computed time fields.

## Status

Open

## Priority

High

## Blocked By

- FEATURE-013: FieldSet registry

## Description

Panel is the most complex entity with ~30 stored fields plus computed time
projections from `TimeRange`.

### TimeRange

Port from v10-try1:

- `Unspecified` — no time info
- `UnspecifiedWithDuration(Duration)` — duration only
- `Scheduled { start: NaiveDateTime, duration: Duration }` — fully scheduled

### PanelData

Hand-written data struct with:

- `entity_id`, `uid`, `name`, `description` (Text for CRDT)
- `panel_type_id: Option<EntityId<PanelTypeEntityType>>`
- `time_range: TimeRange`
- Boolean flags: `hidden`, `needs_av`, `is_18_plus`, etc.
- `presenter_ids: Vec<EntityId<PresenterEntityType>>` (relationship backing)
- `event_room_id: Option<EntityId<EventRoomEntityType>>` (relationship backing)

### Computed fields

- `start_time`, `end_time`, `duration` — projections from `time_range`
- `presenters`, `add_presenters`, `remove_presenters` — relationship fields
  (read/write stubs; full implementation deferred to FEATURE-018)
- `room`, `set_room` — relationship fields (stubs)

## Acceptance Criteria

- TimeRange serialization round-trips correctly
- PanelData compiles with all stored fields
- Time projection computed fields read correctly
- Unit tests for TimeRange operations and field read/write
- Serialization round-trip test for PanelData
