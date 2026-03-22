# `conflicts`

`conflicts` is an optional JSON array of detected scheduling conflicts at the top level.

## Access

Public

## Status

Supported in v7 (unchanged from v4)

## Fields

| Field       | Type           | Public | Description                                     |
| ----------- | -------------- | ------ | ----------------------------------------------- |
| `type`      | string         | yes    | `"room"`, `"presenter"`, or `"group_presenter"` |
| `room`      | string \| null | yes    | Room UID (for room conflicts)                   |
| `presenter` | string \| null | yes    | Presenter name (for presenter/group conflicts)  |
| `panel1`    | object         | yes    | `{ "id": "...", "name": "..." }`                |
| `panel2`    | object         | yes    | `{ "id": "...", "name": "..." }`                |

## Description

The top-level conflicts array contains detected scheduling conflicts across the entire schedule. This is separate from the per-session conflicts arrays contained within each panel session.

### Conflict Types

- **`room`**: Two panels scheduled in the same room at overlapping times
- **`presenter`**: Individual presenter scheduled for two panels at overlapping times
- **`group_presenter`**: Group presenter scheduled for two panels at overlapping times

### Panel Objects

Each `panel1` and `panel2` entry contains:

| Field  | Type   | Description        |
| ------ | ------ | ------------------ |
| `id`   | string | Panel session ID   |
| `name` | string | Panel display name |

### Detection Logic

Conflicts are detected by comparing:

1. **Room conflicts**: Panels with the same `roomId` and overlapping time ranges (excluding room-hours panels overlapping with subpanels)
2. **Presenter conflicts**: Panels sharing individual presenter names with overlapping times
3. **Group conflicts**: Panels where all members of a group are scheduled for overlapping panels

## Examples

```json
[
  {
    "type": "room",
    "room": "3",
    "presenter": null,
    "panel1": {
      "id": "GW097",
      "name": "Advanced Foam Techniques"
    },
    "panel2": {
      "id": "GW098",
      "name": "Foam Sculpting Basics"
    }
  },
  {
    "type": "presenter",
    "room": null,
    "presenter": "December Wynn",
    "panel1": {
      "id": "GP002",
      "name": "Cosplay Contest Misconceptions"
    },
    "panel2": {
      "id": "GP005",
      "name": "Judging Workshop"
    }
  }
]
```

## Notes

- The conflicts array may be omitted entirely when no conflicts exist
- This is separate from per-session conflicts which are contained within each panel session
- Conflict detection considers exact time overlaps, not near misses
- Room-hours panels (`isRoomHours: true`) do not conflict with non-room-hours panels in the same room
- Field names changed from `event1`/`event2` (v4) to `panel1`/`panel2` (v7) for consistency with panel terminology
