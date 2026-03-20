# Schedule JSON Format v4

This document describes version 4 of the schedule JSON format. V4 introduces timeline support and time types while maintaining backward compatibility with earlier versions.

This document is generated from the structured documentation in [json-schedule](json-schedule).

---

## Top-Level Structure

```json
{
  "meta": { ... },
  "timeline": [ ... ],
  "events": [ ... ],
  "rooms": [ ... ],
  "panelTypes": [ ... ],
  "timeTypes": [ ... ],
  "presenters": [ ... ],
  "conflicts": [ ... ]
}
```

## Structures Overview

- [meta](meta-v4.md) - Metadata about the schedule file
- [timeline](timeline-v4.md) - Key time markers for layout and navigation
- [events](events-v4.md) - Array of scheduled events
- [rooms](rooms-v4.md) - Physical and virtual event spaces
- [panelTypes](panelTypes-v4.md) - Event category definitions
- [timeTypes](timeTypes-v4.md) - Time category definitions
- [presenters](presenters-v4.md) - People and groups that present events
- [conflicts](conflicts-v4.md) - Detected scheduling conflicts

## Structure Details

### [`meta`](json-schedule/meta-v4.md)

`meta` is a JSON object containing metadata about the schedule file itself.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

| Field       | Type    | Public | Description                                        |
| ----------- | ------- | ------ | -------------------------------------------------- |
| `title`     | string  | yes    | Display title for the schedule                     |
| `generated` | string  | yes    | ISO 8601 UTC timestamp when the file was generated |
| `version`   | integer | yes    | Schema version number (always `4` for this format) |
| `generator` | string  | yes    | Identifier of the tool that produced the file      |
| `startTime` | string  | yes    | ISO 8601 UTC timestamp of the schedule start date  |

*See full details in: [`meta-v4.md`](json-schedule/meta-v4.md)*

### [`timeline`](json-schedule/timeline-v4.md)

`timeline` is a JSON array of key time markers used for layout, navigation, and formatting.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

| Field         | Type           | Public | Description                                                  |
| ------------- | -------------- | ------ | ------------------------------------------------------------ |
| `id`          | string         | yes    | Unique identifier for the time marker                        |
| `startTime`   | string         | yes    | ISO 8601 UTC timestamp for the marker                        |
| `description` | string         | yes    | Description of the time marker                               |
| `timeType`    | string \| null | yes    | Time type UID, references [timeTypes](timeTypes-v4.md)[].uid |

*See full details in: [`timeline-v4.md`](json-schedule/timeline-v4.md)*

### [`events`](json-schedule/events-v4.md)

`events` is a JSON array where each entry represents a single scheduled item.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

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

*See full details in: [`events-v4.md`](json-schedule/events-v4.md)*

### [`rooms`](json-schedule/rooms-v4.md)

`rooms` is a JSON array where each entry represents a physical or virtual space where events can be scheduled.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

| Field        | Type    | Public | Description                               |
| ------------ | ------- | ------ | ----------------------------------------- |
| `uid`        | integer | yes    | Unique room identifier from spreadsheet   |
| `short_name` | string  | yes    | Abbreviated room name for compact display |
| `long_name`  | string  | yes    | Full room name                            |
| `hotel_room` | string  | yes    | Physical hotel room identifier            |

*See full details in: [`rooms-v4.md`](json-schedule/rooms-v4.md)*

### [`panelTypes`](json-schedule/panelTypes-v4.md)

`panelTypes` is a JSON array where each entry defines a category of events.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

| Field        | Type    | Public | Description                                         |
| ------------ | ------- | ------ | --------------------------------------------------- |
| `uid`        | string  | yes    | Unique identifier in format `"panel-type-{prefix}"` |
| `prefix`     | string  | yes    | Short prefix code, uppercase                        |
| `kind`       | string  | yes    | Human-readable category name                        |
| `color`      | string  | yes    | Hex color code with `#` prefix                      |
| `isBreak`    | boolean | yes    | True for break-type events                          |
| `isCafe`     | boolean | yes    | True for café/social events                         |
| `isWorkshop` | boolean | yes    | True for workshop events                            |

*See full details in: [`panelTypes-v4.md`](json-schedule/panelTypes-v4.md)*

### [`timeTypes`](json-schedule/timeTypes-v4.md)

`timeTypes` is a JSON array where each entry defines a category of time markers used in the timeline.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

| Field    | Type   | Public | Description                                        |
| -------- | ------ | ------ | -------------------------------------------------- |
| `uid`    | string | yes    | Unique identifier in format `"time-type-{prefix}"` |
| `prefix` | string | yes    | Short prefix code, uppercase                       |

*See full details in: [`timeTypes-v4.md`](json-schedule/timeTypes-v4.md)*

### [`presenters`](json-schedule/presenters-v4.md)

`presenters` is a JSON array where each entry represents a person or group that can be assigned to events.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

| Field      | Type     | Public | Description                                                                        |
| ---------- | -------- | ------ | ---------------------------------------------------------------------------------- |
| `name`     | string   | yes    | Display name                                                                       |
| `rank`     | string   | yes    | Role: `"guest"`, `"judge"`, `"staff"`, `"invited_guest"`, or `"fan_panelist"`      |
| `is_group` | boolean  | yes    | True if this entry represents a group rather than an individual                    |
| `members`  | string[] | yes    | For groups: list of individual member names. Empty for individuals                 |
| `groups`   | string[] | yes    | For individuals: list of group names this person belongs to. Empty for non-members |

*See full details in: [`presenters-v4.md`](json-schedule/presenters-v4.md)*

### [`conflicts`](json-schedule/conflicts-v4.md)

`conflicts` is an optional JSON array of detected scheduling conflicts at the top level.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

| Field       | Type           | Public | Description                                     |
| ----------- | -------------- | ------ | ----------------------------------------------- |
| `type`      | string         | yes    | `"room"`, `"presenter"`, or `"group_presenter"` |
| `room`      | string \| null | yes    | Room UID (for room conflicts)                   |
| `presenter` | string \| null | yes    | Presenter name (for presenter/group conflicts)  |
| `event1`    | object         | yes    | `{ "id": "...", "name": "..." }`                |

*See full details in: [`conflicts-v4.md`](json-schedule/conflicts-v4.md)*

## Complete Example

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-01T12:00:00Z",
    "version": 4,
    "generator": "cosam-editor 0.2.0",
    "startTime": "2026-06-26T17:00:00Z",
    "endTime": "2026-06-28T18:00:00Z"
  },
  "timeline": [
    {
      "id": "SPLIT01",
      "startTime": "2026-06-26T17:00:00Z",
      "description": "Thursday Evening",
      "timeType": "time-type-split",
      "note": null
    }
  ],
  "events": [
    {
      "id": "GP002",
      "name": "Cosplay Contest Misconceptions",
      "startTime": "2026-06-26T14:00:00",
      "endTime": "2026-06-26T15:00:00",
      "duration": 60,
      "roomId": 10,
      "panelType": "panel-type-gp",
      "presenters": ["December Wynn", "Pro", "Con"],
      "credits": ["December Wynn", "Pros and Cons Cosplay"],
      "conflicts": [],
      "isFree": true,
      "isFull": false,
      "isKids": false
    }
  ],
  "rooms": [
    {
      "uid": 10,
      "short_name": "GP",
      "long_name": "Main Panel Room",
      "hotel_room": "Salon B/C",
      "sort_key": 1
    }
  ],
  "panelTypes": [
    {
      "uid": "panel-type-gp",
      "prefix": "GP",
      "kind": "Guest Panel",
      "color": "#FDEEB5",
      "isBreak": false,
      "isWorkshop": false,
      "isHidden": false
    }
  ],
  "timeTypes": [
    {
      "uid": "time-type-split",
      "prefix": "SPLIT",
      "kind": "Page split"
    }
  ],
  "presenters": [
    {
      "name": "December Wynn",
      "rank": "guest",
      "is_group": false,
      "members": [],
      "groups": [],
      "always_grouped": false
    }
  ],
  "conflicts": []
}
```

## Migration Notes

---

## Related Documentation

- [JSON Schedule Documentation](json-schedule/) - Complete structured documentation
- [Schedule JSON Format v5 - Private/Full](json-private-v5.md) - This document describes version 5 of the schedule JSON format, private/full variant. This format is produced and consumed by the Rust editor and converter for internal data storage and editing.
- [Schedule JSON Format v5 - Public/Widget](json-public-v5.md) - This document describes version 5 of the schedule JSON format, public/widget variant. This format is produced by the Rust converter or editor in public export mode and consumed by the schedule widget.

*This document is automatically generated. Do not edit directly.*
