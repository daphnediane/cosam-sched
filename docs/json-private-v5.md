# Schedule JSON Format — Private / Full (v5)

This document describes the private (internal) JSON format for schedule data,
version 5. This format is produced and consumed by the Rust editor
(`apps/cosam-editor`) and Rust converter (`apps/cosam-convert`).

For the public-facing widget format, see [json-public-v5.md](json-public-v5.md).  
For the archived v4 format, see [json-format-v4.md](json-format-v4.md).

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

All top-level keys are required except `conflicts`, which may be omitted when
there are no scheduling conflicts.

---

## `meta`

| Field       | Type    | Required | Description                        |
| ----------- | ------- | -------- | ---------------------------------- |
| `title`     | string  | yes      | Display title                      |
| `generated` | string  | yes      | ISO 8601 UTC timestamp             |
| `version`   | integer | yes      | Always `5` for this format         |
| `variant`   | string  | yes      | Always `"full"` for this format    |
| `generator` | string  | no       | Tool identifier string             |
| `startTime` | string  | no       | Schedule start time (ISO 8601 UTC) |
| `endTime`   | string  | no       | Schedule end time (ISO 8601 UTC)   |

---

## `panels` Hash

`panels` is a JSON object keyed by **base ID**. The base ID is the panel type
prefix plus number portion of the Uniq ID, with no part or session suffix.

| Uniq ID     | Base ID |
| ----------- | ------- |
| `GP002`     | `GP002` |
| `GW097P1`   | `GW097` |
| `GW097P2S3` | `GW097` |
| `ME001`     | `ME001` |

Panels with part or session suffixes all nest under the same base key.

---

### Panel Object (Base Level)

| Field                  | Type           | Public | Description                                                      |
| ---------------------- | -------------- | ------ | ---------------------------------------------------------------- |
| `id`                   | string         | yes    | Base ID (same as hash key, e.g. `"GW097"`)                       |
| `name`                 | string         | yes    | Display name of the panel                                        |
| `panelType`            | string \| null | yes    | Panel type UID (e.g. `"panel-type-gw"`)                          |
| `description`          | string \| null | yes    | Base portion of description (see Effective Values)               |
| `note`                 | string \| null | yes    | Base note text                                                   |
| `prereq`               | string \| null | yes    | Base prerequisite text                                           |
| `altPanelist`          | string \| null | yes    | Override text for credits line (see Effective Values)            |
| `cost`                 | string \| null | yes    | Cost string (see Cost Values in `json-format-v4.md`)             |
| `capacity`             | string \| null | yes    | Default seat capacity; sessions may override                     |
| `preRegMax`            | string \| null | no     | Default pre-reg maximum; sessions may override                   |
| `difficulty`           | string \| null | yes    | Skill level indicator (e.g. `"Beginner"`, `"3"`)                 |
| `ticketUrl`            | string \| null | yes    | Default URL for ticket purchase; sessions may override           |
| `isFree`               | boolean        | yes    | True if no additional cost                                       |
| `isKids`               | boolean        | yes    | True for kids-only panels                                        |
| `creditedPresenters`   | string[]       | yes    | Individual presenter names who appear in credits (non-`*` flag)  |
| `uncreditedPresenters` | string[]       | no     | Individual presenter names attending but suppressed from credits |
| `simpleTixEvent`       | string \| null | no     | Default SimpleTix admin portal link; sessions may override       |
| `parts`                | PanelPart[]    | yes    | Parts list; always at least one entry (see below)                |

---

### PanelPart Object

| Field                  | Type            | Public | Description                                                             |
| ---------------------- | --------------- | ------ | ----------------------------------------------------------------------- |
| `partNum`              | integer \| null | yes    | Part number (e.g. `1` for `P1` suffix); `null` when no part subdivision |
| `description`          | string \| null  | yes    | Additive description for this part (appended to base description)       |
| `note`                 | string \| null  | yes    | Additive note for this part                                             |
| `prereq`               | string \| null  | yes    | Additive prerequisite text for this part                                |
| `altPanelist`          | string \| null  | yes    | Override credits text; takes precedence over base when set              |
| `creditedPresenters`   | string[]        | yes    | Additional credited presenter names for this part                       |
| `uncreditedPresenters` | string[]        | no     | Additional uncredited presenter names for this part                     |
| `sessions`             | PanelSession[]  | yes    | Sessions list; always at least one entry (see below)                    |

---

### PanelSession Object

| Field                  | Type            | Public | Description                                                                   |
| ---------------------- | --------------- | ------ | ----------------------------------------------------------------------------- |
| `id`                   | string          | yes    | Full Uniq ID for this session (e.g. `"GW097P1"`, `"GP002"`)                   |
| `sessionNum`           | integer \| null | yes    | Session number (e.g. `2` for `S2` suffix); `null` when no session subdivision |
| `description`          | string \| null  | yes    | Additive description for this session                                         |
| `note`                 | string \| null  | yes    | Additive note for this session                                                |
| `prereq`               | string \| null  | yes    | Additive prerequisite text for this session                                   |
| `altPanelist`          | string \| null  | yes    | Override credits text; takes precedence over part and base when set           |
| `roomIds`              | integer[]       | yes    | Room UIDs for this session; empty array if unscheduled                        |
| `startTime`            | string \| null  | yes    | ISO 8601 local datetime (e.g. `"2026-06-26T14:00:00"`); null if unscheduled   |
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
| `extras`               | ExtraFields     | no     | Additional non-standard spreadsheet columns (see Extra Fields below)          |

The **Public** column indicates whether the field appears in the public JSON
export. Fields marked `no` are private/internal only.

### Extra Fields

`extras` is a JSON object mapping arbitrary string keys to either a plain
string value or a `FormulaValue` object. It holds non-standard spreadsheet
columns that do not correspond to any named field above.

```json
{
  "My Custom Column": "some text",
  "Computed Seats": { "formula": "=D3*0.8", "value": "12" }
}
```

**`FormulaValue` object:**

| Field     | Type   | Description                              |
| --------- | ------ | ---------------------------------------- |
| `formula` | string | The raw spreadsheet formula string       |
| `value`   | string | The computed value at time of conversion |

If the spreadsheet cell contains a literal value (not a formula), the entry
is stored as a plain string. If the cell contains a formula, it is stored as
a `FormulaValue` with both the formula text and its evaluated result.

### Optional fields and null values

All fields whose type includes `null`, `boolean` fields that default to
`false`, and array fields (`string[]`, `PanelPart[]`, etc.) may be **omitted
entirely** from the JSON file. Absent fields are treated identically to their
default value (`null`, `false`, or `[]`). The serializer omits these fields
when writing to reduce file size. Parsers must apply defaults for any absent
field.

---

## Effective Values

Several fields accumulate across hierarchy levels:

### Concatenated fields

`description`, `note`, and `prereq` are concatenated across levels. The
effective value for a session is:

```text
[base.field, part.field, session.field]
```

joined with a single space, skipping any null or empty-string levels. If all
three are null or empty, the effective value is null.

### Override fields

The following fields use **first-wins override** semantics (not concatenation).
The effective value is the first non-null value found scanning from the most
specific level upward:

| Field            | Override chain                 |
| ---------------- | ------------------------------ |
| `altPanelist`    | session → part → base          |
| `ticketUrl`      | session → base (no part level) |
| `simpleTixEvent` | session → base (no part level) |

If all levels are null, there is no effective value for that field.

---

## Effective Presenter Lists

For a given session, the effective credited presenter list is the ordered union
of:

1. `base.creditedPresenters`
2. `part.creditedPresenters`
3. `session.creditedPresenters`

The effective uncredited presenter list is the ordered union of all three
`uncreditedPresenters` lists in the same order.

Credits generation (group resolution, always-grouped presenters) follows the
same rules as v4. See `json-format-v4.md` § Credits Generation.

If an effective `altPanelist` string exists, it **replaces** the computed
credits entirely in public output. If `hidePanelist` is true on the session,
credits are empty in public output regardless of `altPanelist`.

---

## Description Common-Prefix Algorithm

When importing from a spreadsheet, the converter uses a common-prefix algorithm
to factor shared description text to the highest applicable level:

1. When the first row for a base ID is seen, its full description is stored at
   the base level. Part and session descriptions start empty.
2. When a second row with the same base ID is added, the converter computes the
   longest common prefix between the existing base description and the new
   row's description. The prefix must end on a whitespace boundary.
3. If a common prefix exists:
   - It is stored as the base description.
   - All existing sibling descriptions are updated to contain only their
     unique suffix (remainder after the common prefix).
   - The new entry's description is its unique suffix.
4. The same process repeats at the part level when a second session is added
   within an existing part.

**Example:**

- `GW097P1S1` arrives with description `"ABC DEF GH"` → stored as base
  description `"ABC DEF GH"`.
- `GW097P2S1` arrives with description `"ABC LMO IJK"` → common prefix `"ABC"`:
  base becomes `"ABC"`, part 1 description becomes `"DEF GH"`, part 2
  description becomes `"LMO IJK"`.
- `GW097P1S2` arrives with description `"ABC DEF GHI"` → part 1 effective is
  `"ABC DEF GH"`, new common prefix with `"ABC DEF GHI"` is `"ABC DEF"`: part 1
  description becomes `"DEF"`, session 1 description becomes `"GH"`, session 2
  description becomes `"GHI"`.

---

## `rooms`, `panelTypes`, `timeTypes`, `timeline`, `presenters`, `conflicts`

These arrays are identical in structure to v4. See
[json-format-v4.md](json-format-v4.md) for complete field definitions.

---

## Example (abbreviated)

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-01T00:00:00Z",
    "version": 5,
    "variant": "full",
    "generator": "cosam-editor 0.2.0"
  },
  "panels": {
    "GP002": {
      "id": "GP002",
      "name": "Cosplay Contest Misconceptions",
      "panelType": "panel-type-gp",
      "description": "A deep-dive into competition issues.",
      "note": null,
      "prereq": null,
      "altPanelist": null,
      "cost": null,
      "capacity": null,
      "preRegMax": null,
      "difficulty": null,
      "ticketUrl": null,
      "isFree": true,
      "isKids": false,
      "creditedPresenters": ["December Wynn", "Pro", "Con"],
      "uncreditedPresenters": [],
      "simpleTixEvent": null,
      "parts": [
        {
          "partNum": null,
          "description": null,
          "note": null,
          "prereq": null,
          "altPanelist": null,
          "creditedPresenters": [],
          "uncreditedPresenters": [],
          "sessions": [
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
              "uncreditedPresenters": [],
              "notesNonPrinting": null,
              "workshopNotes": null,
              "powerNeeds": null,
              "sewingMachines": false,
              "avNotes": null
            }
          ]
        }
      ]
    },
    "GW097": {
      "id": "GW097",
      "name": "Advanced Foam Techniques",
      "panelType": "panel-type-gw",
      "description": "Common intro",
      "note": null,
      "prereq": null,
      "altPanelist": null,
      "cost": "$35.00",
      "capacity": "15",
      "preRegMax": "12",
      "difficulty": "Intermediate",
      "ticketUrl": "https://simpletix.com/…",
      "isFree": false,
      "isKids": false,
      "creditedPresenters": ["Sayakat Cosplay"],
      "uncreditedPresenters": [],
      "simpleTixEvent": "https://admin.simpletix.com/…",
      "parts": [
        {
          "partNum": 1,
          "description": "Part 1 unique content",
          "note": null,
          "prereq": null,
          "altPanelist": null,
          "creditedPresenters": [],
          "uncreditedPresenters": [],
          "sessions": [
            {
              "id": "GW097P1",
              "sessionNum": null,
              "description": null,
              "note": null,
              "prereq": null,
              "altPanelist": null,
              "roomIds": [3],
              "startTime": "2026-06-26T10:00:00",
              "endTime": "2026-06-26T12:00:00",
              "duration": 120,
              "isFull": false,
              "capacity": null,
              "seatsSold": 3,
              "preRegMax": null,
              "hidePanelist": false,
              "creditedPresenters": [],
              "uncreditedPresenters": [],
              "notesNonPrinting": "Check projector before session",
              "workshopNotes": "Bring extra foam sheets",
              "powerNeeds": "2 outlets",
              "sewingMachines": false,
              "avNotes": null
            }
          ]
        },
        {
          "partNum": 2,
          "description": "Part 2 unique content",
          "note": null,
          "prereq": "GW097P1",
          "altPanelist": null,
          "creditedPresenters": [],
          "uncreditedPresenters": [],
          "sessions": [
            {
              "id": "GW097P2",
              "sessionNum": null,
              "description": null,
              "note": null,
              "prereq": null,
              "altPanelist": null,
              "roomIds": [3],
              "startTime": "2026-06-26T14:00:00",
              "endTime": "2026-06-26T16:00:00",
              "duration": 120,
              "isFull": false,
              "capacity": null,
              "seatsSold": 5,
              "preRegMax": null,
              "hidePanelist": false,
              "creditedPresenters": [],
              "uncreditedPresenters": [],
              "notesNonPrinting": null,
              "workshopNotes": null,
              "powerNeeds": "2 outlets",
              "sewingMachines": false,
              "avNotes": null
            }
          ]
        }
      ]
    }
  },
  "rooms": [ { "uid": 3, "short_name": "WS 1", "long_name": "Workshop Room 1", "hotel_room": "Salon A", "sort_key": 1 } ],
  "panelTypes": [],
  "timeTypes": [],
  "timeline": [],
  "presenters": [],
  "conflicts": []
}
```
