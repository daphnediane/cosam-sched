# Schedule JSON Format — Public / Widget (v5)

This document describes the public JSON format consumed by the
`widget/cosam-calendar.js` schedule widget, version 5. This format is
produced by the Rust converter or editor in public export mode.

For the full private format with all fields, see
[json-private-v5.md](json-private-v5.md).  
For the archived v4 format, see [json-format-v4.md](json-format-v4.md).

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

`conflicts` is omitted from public output. `panels` is a **flat ordered
array** pre-flattened from the private hierarchical format by the exporter —
the widget does not need to understand the base→part→session hierarchy.

---

## `meta`

| Field       | Type    | Required | Description                        |
| ----------- | ------- | -------- | ---------------------------------- |
| `title`     | string  | yes      | Display title                      |
| `generated` | string  | yes      | ISO 8601 UTC timestamp             |
| `version`   | integer | yes      | Always `5` for this format         |
| `variant`   | string  | yes      | Always `"public"` for this format  |
| `generator` | string  | no       | Tool identifier string             |
| `startTime` | string  | no       | Schedule start time (ISO 8601 UTC) |
| `endTime`   | string  | no       | Schedule end time (ISO 8601 UTC)   |

---

## `panels` Array

Each entry represents one **session** — the smallest schedulable unit. For
panels with no part or session subdivisions (the common case), there is
exactly one entry with `partNum: null` and `sessionNum: null`.

Entries are ordered chronologically by `startTime`. Unscheduled sessions
(null `startTime`) appear at the end.

### Panel Entry Fields

| Field         | Type            | Required | Description                                                         |
| ------------- | --------------- | -------- | ------------------------------------------------------------------- |
| `id`          | string          | yes      | Full Uniq ID of this session (e.g. `"GW097P1S2"`, `"GP002"`)        |
| `baseId`      | string          | yes      | Base panel ID (e.g. `"GW097"`, `"GP002"`)                           |
| `partNum`     | integer \| null | yes      | Part number; `null` when no part subdivision                        |
| `sessionNum`  | integer \| null | yes      | Session number; `null` when no session subdivision                  |
| `name`        | string          | yes      | Display name (from base panel)                                      |
| `panelType`   | string \| null  | yes      | Panel type UID (e.g. `"panel-type-gw"`)                             |
| `roomIds`     | integer[]       | yes      | Room UIDs for this session; empty array if unscheduled              |
| `startTime`   | string \| null  | yes      | ISO 8601 local datetime; null if unscheduled                        |
| `endTime`     | string \| null  | yes      | ISO 8601 local datetime                                             |
| `duration`    | integer         | yes      | Duration in minutes                                                 |
| `description` | string \| null  | no       | Effective description (base + part + session concatenated)          |
| `note`        | string \| null  | no       | Effective note                                                      |
| `prereq`      | string \| null  | no       | Effective prerequisite text                                         |
| `cost`        | string \| null  | no       | Cost string from base (see Cost Values in `json-format-v4.md`)      |
| `capacity`    | string \| null  | no       | Effective seat capacity (session override or base default)          |
| `difficulty`  | string \| null  | no       | Skill level indicator (from base)                                   |
| `ticketUrl`   | string \| null  | no       | Effective ticket URL (session override or base default)             |
| `isFree`      | boolean         | yes      | True if no additional cost                                          |
| `isFull`      | boolean         | yes      | True if this session is at capacity                                 |
| `isKids`      | boolean         | yes      | True for kids-only panels                                           |
| `credits`     | string[]        | yes      | Formatted credit strings for public display (see Credits below)     |
| `presenters`  | string[]        | yes      | All individual presenter names for filtering and search (see below) |

### `startTime` and `endTime` Format

ISO 8601 local datetimes **without** a timezone suffix: `YYYY-MM-DDTHH:MM:SS`.
The timezone is the event venue's local time. See `json-format-v4.md` for
details on the cost value format.

### Optional fields and null values

Fields marked `no` in the Required column may be omitted entirely when their
value is null, false, or an empty array. Absent fields are treated as null /
false / `[]` by the widget. The exporter omits these fields to reduce file
size.

---

## Credits

The `credits` array is computed by the exporter from the effective presenter
and `altPanelist` values:

1. If `hidePanelist` is true on the session, `credits` is an empty array.
2. Otherwise, if an effective `altPanelist` value exists (first non-null among
   session → part → base), credits is a single-element array containing that
   string: `["<altPanelist value>"]`.
3. Otherwise, credits are generated by group resolution from the union of all
   `creditedPresenters` across base, part, and session levels, following the
   same rules as v4 (see `json-format-v4.md` § Credits Generation).

---

## `presenters` (per panel entry)

The `presenters` array on each panel entry contains all individual presenter
names (credited and uncredited) from across base, part, and session levels.
This list is used by the widget for presenter filtering and search. It
contains raw individual names, not group names or formatted credits.

---

## `rooms`, `panelTypes`, `timeTypes`, `timeline`

These arrays are identical in structure to v4. See
[json-format-v4.md](json-format-v4.md) for complete field definitions.

---

## `presenters` (top-level array)

The top-level `presenters` array defines all presenters and groups for the
schedule, used to populate the presenter filter dropdown. Structure is
identical to v4.

---

## Example (abbreviated)

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-01T00:00:00Z",
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
      "preRegMax": null,
      "difficulty": null,
      "ticketUrl": null,
      "isFree": true,
      "isFull": false,
      "isKids": false,
      "credits": ["December Wynn", "Pros and Cons Cosplay"],
      "presenters": ["December Wynn", "Pro", "Con"]
    },
    {
      "id": "GW097P1",
      "baseId": "GW097",
      "partNum": 1,
      "sessionNum": null,
      "name": "Advanced Foam Techniques",
      "panelType": "panel-type-gw",
      "roomIds": [3],
      "startTime": "2026-06-26T10:00:00",
      "endTime": "2026-06-26T12:00:00",
      "duration": 120,
      "description": "Common intro Part 1 unique content",
      "note": null,
      "prereq": null,
      "cost": "$35.00",
      "capacity": "15",
      "preRegMax": "12",
      "difficulty": "Intermediate",
      "ticketUrl": "https://simpletix.com/…",
      "isFree": false,
      "isFull": false,
      "isKids": false,
      "credits": ["Sayakat Cosplay"],
      "presenters": ["Sayakat Cosplay"]
    },
    {
      "id": "GW097P2",
      "baseId": "GW097",
      "partNum": 2,
      "sessionNum": null,
      "name": "Advanced Foam Techniques",
      "panelType": "panel-type-gw",
      "roomIds": [3],
      "startTime": "2026-06-26T14:00:00",
      "endTime": "2026-06-26T16:00:00",
      "duration": 120,
      "description": "Common intro Part 2 unique content",
      "note": null,
      "prereq": "GW097P1",
      "cost": "$35.00",
      "capacity": "15",
      "preRegMax": "12",
      "difficulty": "Intermediate",
      "ticketUrl": "https://simpletix.com/…",
      "isFree": false,
      "isFull": false,
      "isKids": false,
      "credits": ["Sayakat Cosplay"],
      "presenters": ["Sayakat Cosplay"]
    }
  ],
  "rooms": [
    { "uid": 3, "short_name": "WS 1", "long_name": "Workshop Room 1", "hotel_room": "Salon A", "sort_key": 1 },
    { "uid": 10, "short_name": "GP", "long_name": "Main Panel Room", "hotel_room": "Salon B/C", "sort_key": 2 }
  ],
  "panelTypes": [
    { "uid": "panel-type-gp", "prefix": "GP", "kind": "Guest Panel", "color": "#FDEEB5", "isBreak": false, "isWorkshop": false },
    { "uid": "panel-type-gw", "prefix": "GW", "kind": "Guest Workshop", "color": "#B5D8FD", "isBreak": false, "isWorkshop": true }
  ],
  "timeTypes": [],
  "timeline": [],
  "presenters": [
    { "name": "December Wynn", "rank": "guest", "is_group": false, "members": [], "groups": [], "always_grouped": false },
    { "name": "Pros and Cons Cosplay", "rank": "guest", "is_group": true, "members": ["Pro", "Con"], "groups": [], "always_grouped": false },
    { "name": "Pro", "rank": "guest", "is_group": false, "members": [], "groups": ["Pros and Cons Cosplay"], "always_grouped": false },
    { "name": "Con", "rank": "guest", "is_group": false, "members": [], "groups": ["Pros and Cons Cosplay"], "always_grouped": false },
    { "name": "Sayakat Cosplay", "rank": "guest", "is_group": false, "members": [], "groups": [], "always_grouped": false }
  ]
}
```
