# Widget v5 support

## Summary

Update `widget/cosam-calendar.js` to consume the v5 public JSON format.

## Status

Completed

## Priority

High

## Description

The v5 public format (see `docs/json-public-v5.md`) changes the top-level
key from `events` to `panels` and replaces the scalar `roomId` with a
`roomIds` array. The widget must be updated to handle these changes.

## Implementation Details

### `panels` array (replaces `events`)

- Read `data.panels` instead of `data.events`.
- All existing filtering, rendering, and display logic operates on the same
  flat list of entries; only the key name changes.

### `roomIds` array (replaces `roomId`)

Room filtering currently checks `e.roomId`. Update to check whether any
element of `e.roomIds` is in the selected room set:

```js
events = events.filter(e =>
  this._isBreakEvent(e) ||
  (e.roomIds && e.roomIds.some(id => this.filters.rooms.has(id)))
);
```

Room display in card/modal: show all rooms for the session, not just one.

### Grid view

The grid layout assigns events to columns by `roomId`. Update to handle
multi-room sessions: place the session in the column of its first room,
spanning to the last room's column (or display in each room column
separately — TBD based on layout preference).

### `baseId` and part/session numbers

These new fields may be used in future features (e.g. grouping related
sessions). No immediate display use required beyond availability.

### No backward compatibility required

All canonical data is in spreadsheets. Old JSON files do not need to load.

## Acceptance Criteria

- Widget loads and displays a v5 public JSON file correctly.
- Room filter works correctly with `roomIds`.
- Panels with multiple rooms show all room names in the event card.
- Presenter filter and search work unchanged.
- Grid view places multi-room sessions correctly.

## References

- [json-public-v5.md](../json-public-v5.md)
- [FEATURE-016.md](FEATURE-016.md) — v5 public export (prerequisite)
