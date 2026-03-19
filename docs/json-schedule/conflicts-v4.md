# `conflicts`

`conflicts` is an optional JSON array of detected scheduling conflicts at the top level.

## Access

Public

## Status

Supported in v4

## Fields

| Field       | Type           | Public | Description                                     |
| ----------- | -------------- | ------ | ----------------------------------------------- |
| `type`      | string         | yes    | `"room"`, `"presenter"`, or `"group_presenter"` |
| `room`      | string \| null | yes    | Room UID (for room conflicts)                   |
| `presenter` | string \| null | yes    | Presenter name (for presenter/group conflicts)  |
| `event1`    | object         | yes    | `{ "id": "...", "name": "..." }`                |
| `event2`    | object         | yes    | `{ "id": "...", "name": "..." }`                |

## Description

The top-level conflicts array contains detected scheduling conflicts across the entire schedule. This is separate from the per-event conflicts arrays contained within each event.

### Conflict Types

- **`room`**: Two events scheduled in the same room at overlapping times
- **`presenter`**: Individual presenter scheduled for two events at overlapping times
- **`group_presenter`**: Group presenter scheduled for two events at overlapping times

### Event Objects

Each `event1` and `event2` entry contains:

| Field  | Type   | Description        |
| ------ | ------ | ------------------ |
| `id`   | string | Event ID           |
| `name` | string | Event display name |

### Detection Logic

Conflicts are detected by comparing:

1. **Room conflicts**: Events with the same `roomId` and overlapping time ranges
2. **Presenter conflicts**: Events sharing individual presenter names with overlapping times
3. **Group conflicts**: Events where all members of a group are scheduled for overlapping events

## Examples

```json
[
  {
    "type": "room",
    "room": "3",
    "presenter": null,
    "event1": {
      "id": "GW097",
      "name": "Advanced Foam Techniques"
    },
    "event2": {
      "id": "GW098", 
      "name": "Foam Sculpting Basics"
    }
  },
  {
    "type": "presenter",
    "room": null,
    "presenter": "December Wynn",
    "event1": {
      "id": "GP002",
      "name": "Cosplay Contest Misconceptions"
    },
    "event2": {
      "id": "GP005",
      "name": "Judging Workshop"
    }
  },
  {
    "type": "group_presenter",
    "room": null,
    "presenter": "Pros and Cons Cosplay",
    "event1": {
      "id": "GP010",
      "name": "Debate Panel"
    },
    "event2": {
      "id": "GP015",
      "name": "Q&A Session"
    }
  }
]
```

## Notes

- The conflicts array may be omitted entirely when no conflicts exist
- This is separate from per-event conflicts which are contained within each event
- Conflict detection considers exact time overlaps, not near misses
- The conflicts array is useful for schedule validation and debugging
