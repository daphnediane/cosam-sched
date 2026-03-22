# `rooms`

`rooms` is a JSON array where each entry represents a physical or virtual space where panels can be scheduled.

## Access

Public

## Status

Supported in v7

## Fields

| Field        | Type           | Public | Description                                    |
| ------------ | -------------- | ------ | ---------------------------------------------- |
| `uid`        | integer        | yes    | Unique room identifier from spreadsheet        |
| `short_name` | string         | yes    | Abbreviated room name for compact display      |
| `long_name`  | string         | yes    | Full room name                                 |
| `hotel_room` | string         | yes    | Physical hotel room identifier                 |
| `sort_key`   | integer        | yes    | Display sort order (lower = first, 1-indexed)  |
| `is_break`   | boolean        | yes    | True for virtual break rooms                   |
| `metadata`   | object \| null | no     | Optional key-value metadata (full format only) |

## Description

Rooms represent physical or virtual spaces where panels are scheduled. Each room has a unique identifier assigned from the spreadsheet order.

### Room UIDs

Room UIDs are assigned based on the order they appear in the spreadsheet's Rooms sheet. They are stable identifiers that must be consistent between the `rooms` array and all `roomIds` references in panel sessions.

### Virtual Break Rooms

Rooms with `is_break: true` are virtual rooms used for break panels. In the display variant, break panels are expanded to fill all inactive visible rooms during their time slot. The virtual break room itself may not appear in room filter dropdowns.

### Sort Key and Hidden Rooms

Rooms with `sort_key` values >= 100 are hidden from the public schedule display. The sort key determines the left-to-right order of rooms in grid views.

## Examples

```json
[
  {
    "uid": 1,
    "short_name": "WS 1",
    "long_name": "Workshop Room 1",
    "hotel_room": "Salon A",
    "sort_key": 1,
    "is_break": false
  },
  {
    "uid": 2,
    "short_name": "WS 2",
    "long_name": "Workshop Room 2",
    "hotel_room": "Salon B",
    "sort_key": 2,
    "is_break": false
  },
  {
    "uid": 99,
    "short_name": "BREAK",
    "long_name": "Break",
    "hotel_room": "",
    "sort_key": 100,
    "is_break": true
  }
]
```

## Notes

- UIDs are not necessarily sequential but are stable across edits
- `sort_key` is 1-indexed for display ordering; values >= 100 are hidden
- `hotel_room` contains the actual hotel room designation for setup purposes
- `is_break` is new in v7; break rooms were implicit in earlier versions
- `metadata` is only present in the full format and is stripped in the display variant
