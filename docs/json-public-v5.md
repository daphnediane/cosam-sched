# Schedule JSON Format v5 - Public/Widget

This document describes version 5 of the schedule JSON format, public/widget variant. This format is produced by the Rust converter or editor in public export mode and consumed by the schedule widget.

This document is generated from the structured documentation in [json-schedule](json-schedule).

---

## Top-Level Structure

```json
{
  "meta": { ... },
  "panels": [ ... ],
  "rooms": [ ... ],
  "panelTypes": [ ... ],
  "timeTypes": [ ... ],
  "timeline": [ ... ],
  "presenters": [ ... ]
}
```

## Structures Overview

- [meta](meta-v5.md) - Metadata about the schedule file (shared with private format)
- [panels](panels-public-v5.md) - Flattened panels array with pre-computed effective values
- [rooms](rooms-v4.md) - Physical and virtual event spaces (same as v4)
- [panelTypes](panelTypes-v4.md) - Event category definitions (same as v4)
- [timeTypes](timeTypes-v4.md) - Time category definitions (same as v4)
- [timeline](timeline-v4.md) - Key time markers for layout and navigation (same as v4)
- [presenters](presenters-v4.md) - People and groups that present events (same as v4)

## Structure Details

### [`meta`](json-schedule/meta-v5.md)

`meta` is a JSON object containing metadata about the schedule file itself.

**Access:** Public

**Status:** Supported in v5

**Key Fields:**

| Field       | Type    | Public | Description                                                 |
| ----------- | ------- | ------ | ----------------------------------------------------------- |
| `title`     | string  | yes    | Display title for the schedule                              |
| `generated` | string  | yes    | ISO 8601 UTC timestamp when the file was generated          |
| `version`   | integer | yes    | Schema version number (always `5` for this format)          |
| `variant`   | string  | yes    | Format variant: `"full"` for private, `"public"` for public |
| `generator` | string  | yes    | Identifier of the tool that produced the file               |
| `startTime` | string  | yes    | ISO 8601 UTC timestamp of the schedule start date           |

*See full details in: [`meta-v5.md`](json-schedule/meta-v5.md)*

### [`panels`](json-schedule/panels-public-v5.md)

`panels` is a JSON array where each entry represents one **session** - the smallest schedulable unit, flattened from the private hierarchical format.

**Access:** Public

**Status:** Supported in v5 (public format only)

**Key Fields:**

| Field         | Type            | Public | Description                                                  |
| ------------- | --------------- | ------ | ------------------------------------------------------------ |
| `id`          | string          | yes    | Full Uniq ID of this session (e.g. `"GW097P1S2"`, `"GP002"`) |
| `baseId`      | string          | yes    | Base panel ID (e.g. `"GW097"`, `"GP002"`)                    |
| `partNum`     | integer \| null | yes    | Part number; `null` when no part subdivision                 |
| `sessionNum`  | integer \| null | yes    | Session number; `null` when no session subdivision           |
| `name`        | string          | yes    | Display name (from base panel)                               |
| `panelType`   | string \| null  | yes    | Panel type UID (e.g. `"panel-type-gw"`)                      |
| `roomIds`     | integer[]       | yes    | Room UIDs for this session; empty array if unscheduled       |
| `startTime`   | string \| null  | yes    | ISO 8601 local datetime; null if unscheduled                 |
| `endTime`     | string \| null  | yes    | ISO 8601 local datetime                                      |
| `duration`    | integer         | yes    | Duration in minutes                                          |
| `description` | string \| null  | yes    | Effective description (base + part + session concatenated)   |
| `note`        | string \| null  | yes    | Effective note                                               |
| `prereq`      | string \| null  | yes    | Effective prerequisite text                                  |
| `cost`        | string \| null  | yes    | Cost string from base (see Cost Values in v4 documentation)  |
| `capacity`    | string \| null  | yes    | Effective seat capacity (session override or base default)   |
| `difficulty`  | string \| null  | yes    | Skill level indicator (from base)                            |
| `ticketUrl`   | string \| null  | yes    | Effective ticket URL (session override or base default)      |
| `isFree`      | boolean         | yes    | True if no additional cost                                   |
| `isFull`      | boolean         | yes    | True if this session is at capacity                          |
| `isKids`      | boolean         | yes    | True for kids-only panels                                    |
| `credits`     | string[]        | yes    | Formatted credit strings for public display                  |

*See full details in: [`panels-public-v5.md`](json-schedule/panels-public-v5.md)*

### [`rooms`](json-schedule/rooms-v4.md)

`rooms` is a JSON array where each entry represents a physical or virtual space where events can be scheduled.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

| Field        | Type    | Public | Description                                   |
| ------------ | ------- | ------ | --------------------------------------------- |
| `uid`        | integer | yes    | Unique room identifier from spreadsheet       |
| `short_name` | string  | yes    | Abbreviated room name for compact display     |
| `long_name`  | string  | yes    | Full room name                                |
| `hotel_room` | string  | yes    | Physical hotel room identifier                |

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

### [`presenters`](json-schedule/presenters-v4.md)

`presenters` is a JSON array where each entry represents a person or group that can be assigned to events.

**Access:** Public

**Status:** Supported in v4

**Key Fields:**

| Field            | Type     | Public | Description                                                                        |
| ---------------- | -------- | ------ | ---------------------------------------------------------------------------------- |
| `name`           | string   | yes    | Display name                                                                       |
| `rank`           | string   | yes    | Role: `"guest"`, `"judge"`, `"staff"`, `"invited_guest"`, or `"fan_panelist"`      |
| `is_group`       | boolean  | yes    | True if this entry represents a group rather than an individual                    |
| `members`        | string[] | yes    | For groups: list of individual member names. Empty for individuals                 |
| `groups`         | string[] | yes    | For individuals: list of group names this person belongs to. Empty for non-members |

*See full details in: [`presenters-v4.md`](json-schedule/presenters-v4.md)*

## Complete Example

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-01T12:00:00Z",
    "version": 5,
    "variant": "public",
    "generator": "cosam-editor 0.2.0",
    "startTime": "2026-06-26T17:00:00Z",
    "endTime": "2026-06-28T18:00:00Z"
  },
  "panels": [
    {
      "id": "GP002",
      "baseId": "GP002",
      "partNum": null,
      "sessionNum": null,
      "name": "Cosplay Contest Misconceptions",
      "panelType": "panel-type-gp",
      "roomIds": [10],
      "startTime": "2026-06-26T14:00:00",
      "endTime": "2026-06-26T15:00:00",
      "duration": 60,
      "description": "A deep-dive into competition issues.",
      "note": null,
      "prereq": null,
      "cost": null,
      "capacity": null,
      "difficulty": null,
      "ticketUrl": null,
      "isFree": true,
      "isFull": false,
      "isKids": false,
      "credits": ["December Wynn", "Pros and Cons Cosplay"],
      "presenters": ["December Wynn", "Pro", "Con"]
    }
  ],
  "rooms": [],
  "panelTypes": [],
  "timeTypes": [],
  "timeline": [],
  "presenters": []
}
```

## Migration Notes

---

## Related Documentation

- [JSON Schedule Documentation](json-schedule/) - Complete structured documentation
- [Display Format v10](json-v10-display.md) - Public-facing schedule format with DisplayPresenter objects and filtered presenter list.
- [Full Format v10](json-v10-full.md) - Complete internal schedule format with flat presenter relationship fields and edit history support.
- [Schedule JSON Format v4](json-format-v4.md) - This document describes version 4 of the schedule JSON format. V4 introduces timeline support and time types while maintaining backward compatibility with earlier versions.
- [Schedule JSON Format v5 - Private/Full](json-private-v5.md) - This document describes version 5 of the schedule JSON format, private/full variant. This format is produced and consumed by the Rust editor and converter for internal data storage and editing.
- [v6-Private](json-private-v6.md) - Private format documentation for JSON schedule format v6.
- [v6-Public](json-public-v6.md) - Public format documentation for JSON schedule format v6.
- [v7-Display](json-v7-display.md) - Display format documentation for JSON schedule format v7. This is the public-facing format consumed by the schedule widget.
- [v7-Full](json-v7-full.md) - Full format documentation for JSON schedule format v7. This is the editable master format used by the editor and converter.
- [v8-Full](json-v8-full.md) - Full format documentation for JSON schedule format v8. This is the editable master format used by the editor and converter, with support for persistent edit history via the optional `changeLog` field.
- [Display Format v9](json-v9-display.md) - Public-facing schedule format with DisplayPresenter objects and filtered presenter list.
- [Full Format v9](json-v9-full.md) - Complete internal schedule format with full presenter data and edit history support.

*This document is automatically generated. Do not edit directly.*
