# Schedule JSON Format v5 - Private/Full

This document describes version 5 of the schedule JSON format, private/full variant. This format is produced and consumed by the Rust editor and converter for internal data storage and editing.

This document is generated from the structured documentation in [json-schedule](json-schedule).

---

## Top-Level Structure

```json
{
  "meta": { ... },
  "panels": { "GP002": { ... }, "GW097": { ... } },
  "rooms": [ ... ],
  "panelTypes": [ ... ],
  "timeTypes": [ ... ],
  "timeline": [ ... ],
  "presenters": [ ... ],
  "conflicts": [ ... ]
}
```

## Structures Overview

- [meta](meta-v5.md) - Metadata about the schedule file (shared with public format)
- [panels](panels-v5.md) - Hierarchical panels hash with base→part→session nesting
- [PanelPart](PanelPart-v5.md) - Panel part objects (subdivision of base panels)
- [PanelSession](PanelSession-v5.md) - Panel session objects (individual scheduled occurrences)
- [rooms](rooms-v4.md) - Physical and virtual event spaces (same as v4)
- [panelTypes](panelTypes-v4.md) - Event category definitions (same as v4)
- [timeTypes](timeTypes-v4.md) - Time category definitions (same as v4)
- [timeline](timeline-v4.md) - Key time markers for layout and navigation (same as v4)
- [presenters](presenters-v4.md) - People and groups that present events (same as v4)
- [conflicts](conflicts-v4.md) - Detected scheduling conflicts (same as v4)

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

### [`panels`](json-schedule/panels-v5.md)

`panels` is a JSON object keyed by **base ID** containing hierarchical panel data with base→part→session nesting.

**Access:** Private

**Status:** Supported in v5 (private format only)

**Key Fields:**

| Field                  | Type                           | Public | Description                                                      |
| ---------------------- | ------------------------------ | ------ | ---------------------------------------------------------------- |
| `id`                   | string                         | yes    | Base ID (same as hash key, e.g. `"GW097"`)                       |
| `name`                 | string                         | yes    | Display name of the panel                                        |
| `panelType`            | string \| null                 | yes    | Panel type UID (e.g. `"panel-type-gw"`)                          |
| `description`          | string \| null                 | yes    | Base portion of description (see Effective Values)               |
| `note`                 | string \| null                 | yes    | Base note text                                                   |
| `prereq`               | string \| null                 | yes    | Base prerequisite text                                           |
| `altPanelist`          | string \| null                 | yes    | Override text for credits line (see Effective Values)            |
| `cost`                 | string \| null                 | yes    | Cost string (see Cost Values in v4 documentation)                |
| `capacity`             | string \| null                 | yes    | Default seat capacity; sessions may override                     |
| `preRegMax`            | string \| null                 | no     | Default pre-reg maximum; sessions may override                   |
| `difficulty`           | string \| null                 | yes    | Skill level indicator (e.g. `"Beginner"`, `"3"`)                 |
| `ticketUrl`            | string \| null                 | yes    | Default URL for ticket purchase; sessions may override           |
| `isFree`               | boolean                        | yes    | True if no additional cost                                       |
| `isKids`               | boolean                        | yes    | True for kids-only panels                                        |
| `creditedPresenters`   | string[]                       | yes    | Individual presenter names who appear in credits (non-`*` flag)  |
| `uncreditedPresenters` | string[]                       | no     | Individual presenter names attending but suppressed from credits |
| `simpleTixEvent`       | string \| null                 | no     | Default SimpleTix admin portal link; sessions may override       |

*See full details in: [`panels-v5.md`](json-schedule/panels-v5.md)*

### [`PanelPart`](json-schedule/PanelPart-v5.md)

`PanelPart` is an object representing a subdivision of a base panel, containing one or more sessions.

**Access:** Private

**Status:** Supported in v5 (private format only)

**Key Fields:**

| Field                  | Type                                 | Public | Description                                                             |
| ---------------------- | ------------------------------------ | ------ | ----------------------------------------------------------------------- |
| `partNum`              | integer \| null                      | yes    | Part number (e.g. `1` for `P1` suffix); `null` when no part subdivision |
| `description`          | string \| null                       | yes    | Additive description for this part (appended to base description)       |
| `note`                 | string \| null                       | yes    | Additive note for this part                                             |
| `prereq`               | string \| null                       | yes    | Additive prerequisite text for this part                                |
| `altPanelist`          | string \| null                       | yes    | Override credits text; takes precedence over base when set              |
| `creditedPresenters`   | string[]                             | yes    | Additional credited presenter names for this part                       |
| `uncreditedPresenters` | string[]                             | no     | Additional uncredited presenter names for this part                     |

*See full details in: [`PanelPart-v5.md`](json-schedule/PanelPart-v5.md)*

### [`PanelSession`](json-schedule/PanelSession-v5.md)

`PanelSession` is an object representing a specific scheduled occurrence of a panel part.

**Access:** Private

**Status:** Supported in v5 (private format only)

**Key Fields:**

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

*See full details in: [`PanelSession-v5.md`](json-schedule/PanelSession-v5.md)*

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
    "version": 5,
    "variant": "full",
    "generator": "cosam-editor 0.2.0",
    "startTime": "2026-06-26T17:00:00Z",
    "endTime": "2026-06-28T18:00:00Z"
  },
  "panels": {
    "GP002": {
      "id": "GP002",
      "name": "Cosplay Contest Misconceptions",
      "panelType": "panel-type-gp",
      "description": "A deep-dive into competition issues.",
      "isFree": true,
      "isKids": false,
      "creditedPresenters": ["December Wynn", "Pro", "Con"],
      "parts": [
        {
          "partNum": null,
          "sessions": [
            {
              "id": "GP002",
              "roomIds": [10],
              "startTime": "2026-06-26T14:00:00",
              "endTime": "2026-06-26T15:00:00",
              "duration": 60,
              "isFull": false
            }
          ]
        }
      ]
    }
  },
  "rooms": [],
  "panelTypes": [],
  "timeTypes": [],
  "timeline": [],
  "presenters": [],
  "conflicts": []
}
```

## Migration Notes

---

## Related Documentation

- [JSON Schedule Documentation](json-schedule/) - Complete structured documentation
- [Schedule JSON Format v4](json-format-v4.md) - This document describes version 4 of the schedule JSON format. V4 introduces timeline support and time types while maintaining backward compatibility with earlier versions.
- [Schedule JSON Format v5 - Public/Widget](json-public-v5.md) - This document describes version 5 of the schedule JSON format, public/widget variant. This format is produced by the Rust converter or editor in public export mode and consumed by the schedule widget.

*This document is automatically generated. Do not edit directly.*
