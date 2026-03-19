# `PanelSession`

`PanelSession` is an object representing a specific scheduled occurrence of a panel part.

## Access

Private

## Status

Supported in v5 (private format only)

## Fields

| Field                  | Type            | Public | Description                                                                   |
| ---------------------- | --------------- | ------ | ----------------------------------------------------------------------------- |
| `id`                   | string          | yes    | Full Uniq ID for this session (e.g. `"GW097P1"`, `"GP002"`)                   |
| `sessionNum`           | integer \| null | yes    | Session number (e.g. `2` for `S2` suffix); `null` when no session subdivision |
| `description`          | string \| null  | yes    | Additive description for this session                                         |
| `note`                 | string \| null  | yes    | Additive note for this session                                                |
| `prereq`               | string \| null  | yes    | Additive prerequisite text for this session                                   |
| `altPanelist`          | string \| null  | yes    | Override credits text; takes precedence over part and base when set           |
| `roomIds`              | integer[]       | yes    | Room UIDs for this session; empty array if unscheduled                        |
| `startTime`            | string \| null  | yes    | ISO 8601 local datetime; null if unscheduled                                  |
| `endTime`              | string \| null  | yes    | ISO 8601 local datetime                                                       |
| `duration`             | integer         | yes    | Duration in minutes                                                           |
| `isFull`               | boolean         | yes    | True if this session is at capacity                                           |
| `capacity`             | string \| null  | yes    | Per-session seat capacity override; null = use base value                     |
| `seatsSold`            | integer \| null | no     | Number of seats already pre-sold via ticketing                                |
| `preRegMax`            | string \| null  | no     | Per-session pre-reg maximum; null = use base value                            |
| `ticketUrl`            | string \| null  | yes    | Per-session ticket URL override; null = use base value                        |
| `simpleTixEvent`       | string \| null  | no     | Per-session SimpleTix admin portal link; null = use base value                |
| `hidePanelist`         | boolean         | no     | True to suppress presenter credits entirely for this session                  |
| `creditedPresenters`   | string[]        | yes    | Additional credited presenter names for this session                          |
| `uncreditedPresenters` | string[]        | no     | Additional uncredited presenter names for this session                        |
| `notesNonPrinting`     | string \| null  | no     | Internal notes (not shown publicly)                                           |
| `workshopNotes`        | string \| null  | no     | Notes for workshop staff                                                      |
| `powerNeeds`           | string \| null  | no     | Power requirements                                                            |
| `sewingMachines`       | boolean         | no     | True if sewing machines are required                                          |
| `avNotes`              | string \| null  | no     | Audio/visual setup notes                                                      |
| `extras`               | ExtraFields     | no     | Additional non-standard spreadsheet columns                                   |

## Description

Panel sessions represent the smallest schedulable unit - a specific occurrence of a panel part at a particular time and location. Each session has a unique Uniq ID and can have its own scheduling details.

### Session ID Generation

Session IDs are constructed from the base ID, part number, and session number:

- Base only: `GP002` → `id: "GP002"`
- With part: `GW097P1` → `base: "GW097"`, `partNum: 1`
- With session: `GW097P1S2` → `base: "GW097"`, `partNum: 1`, `sessionNum: 2`

### Scheduling Fields

- `roomIds`: Array of room UIDs where this session takes place
- `startTime`/`endTime`: ISO 8601 local times without timezone
- `duration`: Duration in minutes, must match time difference

### Workshop-Specific Fields

These fields are primarily used for workshop-type panels:

- `capacity`: Maximum attendees for this session
- `seatsSold`: Pre-sold seats via ticketing
- `preRegMax`: Pre-registration limit
- `workshopNotes`: Staff instructions
- `powerNeeds`: Electrical requirements
- `sewingMachines`: Equipment needs
- `avNotes`: Audio/visual setup requirements

### Extra Fields

The `extras` field contains arbitrary additional columns from the spreadsheet that don't correspond to named fields. Each entry can be either:

- Plain string value for literal cells
- `FormulaValue` object for formula cells with both formula and computed result

### Optional Fields

All fields whose type includes `null`, `boolean` fields that default to `false`, and array fields may be **omitted entirely** when they have default values.

## Examples

### Simple Panel Session

```json
{
  "id": "GP002",
  "sessionNum": null,
  "description": null,
  "note": null,
  "prereq": null,
  "altPanelist": null,
  "roomIds": [10],
  "startTime": "2026-06-26T14:00:00",
  "endTime": "2026-06-26T15:00:00",
  "duration": 60,
  "isFull": false,
  "capacity": null,
  "seatsSold": null,
  "preRegMax": null,
  "hidePanelist": false,
  "creditedPresenters": [],
  "uncreditedPresenters": []
}
```

### Workshop Session

```json
{
  "id": "GW097P1",
  "sessionNum": null,
  "description": "Hands-on practice session",
  "note": null,
  "prereq": null,
  "altPanelist": null,
  "roomIds": [3],
  "startTime": "2026-06-26T10:00:00",
  "endTime": "2026-06-26T12:00:00",
  "duration": 120,
  "isFull": false,
  "capacity": "15",
  "seatsSold": 3,
  "preRegMax": "12",
  "hidePanelist": false,
  "creditedPresenters": [],
  "uncreditedPresenters": [],
  "notesNonPrinting": "Check projector before session",
  "workshopNotes": "Bring extra foam sheets",
  "powerNeeds": "2 outlets",
  "sewingMachines": false,
  "avNotes": "Need projector and speakers",
  "extras": {
    "Custom Field": "some value",
    "Computed Value": {
      "formula": "=D3*0.8",
      "value": "12"
    }
  }
}
```

## Notes

- Sessions are the only level that contains scheduling information (time, room)
- `hidePanelist` affects public output by suppressing all credits
- Workshop-specific fields are optional but important for event planning
- Extra fields preserve spreadsheet data that doesn't fit the standard schema
