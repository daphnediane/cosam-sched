# `events`

`events` is a JSON array where each entry represents a single scheduled item.

## Access

Public

## Status

Supported in v4

## Fields

| Field         | Type            | Public | Description                                                     |
| ------------- | --------------- | ------ | --------------------------------------------------------------- |
| `id`          | string          | yes    | Unique event ID, typically prefix + number                      |
| `name`        | string          | yes    | Display name of the event                                       |
| `description` | string \| null  | yes    | Long description text                                           |
| `startTime`   | string          | yes    | ISO 8601 local time without timezone                            |
| `endTime`     | string          | yes    | ISO 8601 local time without timezone                            |
| `duration`    | integer         | yes    | Duration in minutes                                             |
| `roomId`      | integer \| null | yes    | References [rooms](rooms-v4.md)[].uid                           |
| `panelType`   | string \| null  | yes    | Panel type UID, references [panelTypes](panelTypes-v4.md)[].uid |
| `kind`        | string \| null  | yes    | Human-readable event type                                       |
| `cost`        | string \| null  | yes    | Cost as formatted currency string                               |
| `capacity`    | string \| null  | yes    | Maximum attendees as string                                     |
| `difficulty`  | string \| null  | yes    | Skill level or difficulty rating                                |
| `note`        | string \| null  | yes    | Additional notes for the event                                  |
| `prereq`      | string \| null  | yes    | Prerequisites text                                              |
| `ticketUrl`   | string \| null  | yes    | URL for ticket purchase                                         |
| `presenters`  | string[]        | yes    | All presenter names (credited and uncredited)                   |
| `credits`     | string[]        | yes    | Public-facing attribution list                                  |
| `conflicts`   | object[]        | yes    | List of scheduling conflicts for this event                     |
| `isFree`      | boolean         | yes    | True if the event has no cost                                   |
| `isFull`      | boolean         | yes    | True if the event is at capacity                                |
| `isKids`      | boolean         | yes    | True for kids-only events                                       |

## Description

Each event represents a single scheduled item in the convention program. Events can be panels, workshops, breaks, or other scheduled activities.

### Time Format

Times are ISO 8601 local datetimes **without** a timezone suffix: `YYYY-MM-DDTHH:MM:SS`. The timezone is assumed to be the event venue's local time. The `duration` field must be consistent with the difference between `startTime` and `endTime`.

### Cost Values

| Spreadsheet Value             | JSON `cost`                    | `isFree`                   |
| ----------------------------- | ------------------------------ | -------------------------- |
| empty / `*` / free / n/a / $0 | `null`                         | `true`                     |
| `kids`                        | `null`                         | `true` (`isKids` = `true`) |
| `TBD` / `T.B.D.`              | `"TBD"`                        | `false`                    |
| `model`                       | `"model"`                      | `false`                    |
| numeric or `$X.XX`            | `"$X.XX"` (currency formatted) | `false`                    |

### Credits Generation

The `credits` array is derived from `presenters` using group resolution:

1. **Always-grouped presenters** are added first using their group name
2. For each remaining presenter:
   - If the presenter belongs to a **group** and **all members** of that group are in the event's `presenters`, the **group name** is used
   - If only **some members** are present, each is shown as `"{member} of {group}"`
   - Otherwise the individual name is used
3. Duplicate names are suppressed

Credits are the names shown publicly in the schedule display. The `presenters` list retains the raw individual names for scheduling and conflict detection.

### Per-Event Conflicts

Each entry in the per-event `conflicts` array:

| Field               | Type           | Description                         |
| ------------------- | -------------- | ----------------------------------- |
| `type`              | string         | `"room"` or `"presenter"`           |
| `conflict_event_id` | string \| null | ID of the conflicting event         |
| `details`           | string \| null | Human-readable conflict description |

## Examples

```json
[
  {
    "id": "GP002",
    "name": "Cosplay Contest Misconceptions",
    "description": "A deep-dive into competition issues.",
    "startTime": "2026-06-26T14:00:00",
    "endTime": "2026-06-26T15:00:00",
    "duration": 60,
    "roomId": 10,
    "panelType": "panel-type-gp",
    "kind": "Guest Panel",
    "cost": null,
    "capacity": null,
    "difficulty": null,
    "note": null,
    "prereq": null,
    "ticketUrl": null,
    "presenters": ["December Wynn", "Pro", "Con"],
    "credits": ["December Wynn", "Pros and Cons Cosplay"],
    "conflicts": [],
    "isFree": true,
    "isFull": false,
    "isKids": false
  },
  {
    "id": "GW097",
    "name": "Advanced Foam Techniques",
    "description": "Learn advanced foam crafting techniques.",
    "startTime": "2026-06-26T10:00:00",
    "endTime": "2026-06-26T12:00:00",
    "duration": 120,
    "roomId": 3,
    "panelType": "panel-type-gw",
    "kind": "Guest Workshop",
    "cost": "$35.00",
    "capacity": "15",
    "difficulty": "Intermediate",
    "note": "Bring reference photos",
    "prereq": null,
    "ticketUrl": "https://simpletix.com/e/advanced-foam",
    "presenters": ["Sayakat Cosplay"],
    "credits": ["Sayakat Cosplay"],
    "conflicts": [],
    "isFree": false,
    "isFull": false,
    "isKids": false
  }
]
```

## Notes

- `roomId` can be null for events without a physical room (breaks, staff meals)
- Virtual rooms may exist that don't appear in the `rooms` array but can still be referenced
- The `kind` field is derived from the panel type's `kind` field in v4
- `conflicts` array is empty when no conflicts exist
