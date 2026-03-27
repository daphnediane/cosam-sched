# Panel Structure v9

**Access Level**: Private  
**Status**: Supported  
**Version**: v9-full

A fully self-contained panel entry in the flat model. Each panel belongs to a PanelSet identified by `baseId`.

## Fields

| Field                 | Type                   | Public | Description                                              |
| --------------------- | ---------------------- | ------ | -------------------------------------------------------- |
| id                    | String                 | ✓      | Unique identifier (e.g. `"GP002P1S2"`)                   |
| baseId                | String                 | ✗      | Base ID of containing PanelSet (e.g. `"GP002"`)          |
| partNum               | Integer \| null        | ✗      | Part number (omitted if null)                            |
| sessionNum            | Integer \| null        | ✗      | Session number (omitted if null)                         |
| name                  | String                 | ✓      | Panel display name                                       |
| panelType             | String \| null         | ✓      | Panel type UID (e.g. `"panel-type-GP"`)                  |
| description           | String \| null         | ✓      | Panel description                                        |
| note                  | String \| null         | ✓      | Additional notes                                         |
| prereq                | String \| null         | ✓      | Prerequisites                                            |
| altPanelist           | String \| null         | ✗      | Alternative panelist label for display                   |
| cost                  | String \| null         | ✓      | Cost (formatted as currency, e.g. `"$5.00"`)            |
| capacity              | String \| null         | ✓      | Room capacity                                            |
| preRegMax             | String \| null         | ✗      | Pre-registration maximum                                 |
| difficulty            | String \| null         | ✓      | Difficulty level                                         |
| ticketUrl             | String \| null         | ✓      | Ticket purchase URL                                      |
| simpleTixEvent        | String \| null         | ✗      | SimpleTix event identifier                               |
| haveTicketImage       | Boolean \| null        | ✗      | Whether a ticket image exists                            |
| isFree                | Boolean                | ✓      | Free admission (default false, omitted when false)       |
| isKids                | Boolean                | ✓      | Kids-friendly (default false, omitted when false)        |
| isFull                | Boolean                | ✓      | Event is full (default false, omitted when false)        |
| hidePanelist          | Boolean                | ✗      | Hide panelist names in display (default false)           |
| sewingMachines        | Boolean                | ✗      | Requires sewing machines (default false)                 |
| roomIds               | Array\<Integer>        | ✓      | Room UIDs (empty array omitted)                          |
| timing                | TimeRange              | ✓      | Timing information (see below)                           |
| seatsSold             | Integer \| null        | ✗      | Number of seats sold                                     |
| creditedPresenters    | Array\<String>         | ✓      | Presenter names for credits (empty array omitted)        |
| uncreditedPresenters  | Array\<String>         | ✗      | Hidden presenter names (empty array omitted)             |
| notesNonPrinting      | String \| null         | ✗      | Internal notes (not for display)                         |
| workshopNotes         | String \| null         | ✗      | Workshop-specific notes                                  |
| powerNeeds            | String \| null         | ✗      | Power requirements                                       |
| avNotes               | String \| null         | ✗      | Audio/visual notes                                       |
| conflicts             | Array\<EventConflict>  | ✗      | Detected conflicts (empty array omitted)                 |
| metadata              | Object                 | ✗      | Extra key-value pairs from non-standard columns (omitted when empty) |

## TimeRange Serialization

The `timing` field uses a tagged enum serialization with four variants:

### Unspecified (no timing)

```json
"timing": "Unspecified"
```

### UnspecifiedWithDuration (duration only, no start time)

```json
"timing": {
  "UnspecifiedWithDuration": 60
}
```

Value is duration in minutes.

### UnspecifiedWithStart (start time only, no duration)

```json
"timing": {
  "UnspecifiedWithStart": "2026-06-26T14:00:00"
}
```

Value is ISO 8601 datetime string (no timezone).

### Scheduled (start time + duration)

```json
"timing": {
  "Scheduled": {
    "start_time": "2026-06-26T14:00:00",
    "duration": 60
  }
}
```

- `start_time`: ISO 8601 datetime string (no timezone)
- `duration`: Duration in minutes

Note: End time is not stored; it is computed as `start_time + duration`.

## Key Changes from v8

- **Flat model**: Panels are now self-contained with all fields inline. The v8 hierarchical model split data across base panel, PanelPart, and PanelSession levels.
- **`id` field**: Each panel has a unique ID (e.g. `"GP002P1S2"`) instead of being addressed by `(base_id, part_index, session_index)`.
- **`timing` field**: Replaces separate `startTime`/`endTime`/`duration` fields with a `TimeRange` tagged enum that captures all scheduling states.
- **`panelType` UID format**: Uses `"panel-type-{prefix}"` format instead of raw prefix.

## JSON Example

```json
{
  "id": "GP002",
  "baseId": "GP002",
  "name": "Cosplay Contest Misconceptions",
  "panelType": "panel-type-GP",
  "description": "A deep-dive into competition issues.",
  "isFree": true,
  "roomIds": [10],
  "timing": {
    "Scheduled": {
      "start_time": "2026-06-26T14:00:00",
      "duration": 60
    }
  },
  "creditedPresenters": ["December Wynn"],
  "metadata": {
    "custom_field": "custom value"
  }
}
```

## Notes

- Fields with default values (`false` for booleans, empty arrays, `null` for optionals) are omitted from serialization
- The `source` and `changeState` fields are runtime-only and never serialized
- `metadata` uses the alias `"extras"` for backward compatibility during deserialization
- Panel IDs follow the pattern `{baseId}[P{partNum}][S{sessionNum}]`
