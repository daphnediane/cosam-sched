# `rooms`

`rooms` is a JSON array where each entry represents a physical or virtual space where events can be scheduled.

## Access

Public

## Status

Supported in v4

## Fields

| Field        | Type    | Public | Description                                   |
| ------------ | ------- | ------ | --------------------------------------------- |
| `uid`        | integer | yes    | Unique room identifier from spreadsheet       |
| `short_name` | string  | yes    | Abbreviated room name for compact display     |
| `long_name`  | string  | yes    | Full room name                                |
| `hotel_room` | string  | yes    | Physical hotel room identifier                |
| `sort_key`   | integer | yes    | Display sort order (lower = first, 1-indexed) |

## Description

Rooms represent physical or virtual spaces where events are scheduled. Each room has a unique identifier assigned from the spreadsheet order.

### Room UIDs

Room UIDs are assigned based on the order they appear in the spreadsheet's Rooms sheet. They are stable identifiers that must be consistent between the `rooms` array and all `roomId` references in events.

### Virtual Rooms

Virtual rooms may exist for break events, day separators, or staff meals. These virtual rooms may not appear in the `rooms` array but can still be referenced by `roomId` in events.

## Examples

```json
[
  {
    "uid": 1,
    "short_name": "WS 1",
    "long_name": "Workshop Room 1",
    "hotel_room": "Salon A",
    "sort_key": 1
  },
  {
    "uid": 2,
    "short_name": "WS 2",
    "long_name": "Workshop Room 2", 
    "hotel_room": "Salon B",
    "sort_key": 2
  },
  {
    "uid": 10,
    "short_name": "GP",
    "long_name": "Main Panel Room",
    "hotel_room": "Salon B/C",
    "sort_key": 3
  }
]
```

## Notes

- UIDs are not necessarily sequential but are stable
- `sort_key` is 1-indexed for display ordering
- `hotel_room` contains the actual hotel room designation for setup purposes
