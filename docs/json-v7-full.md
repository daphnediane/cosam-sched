# v7-Full

Full format documentation for JSON schedule format v7. This is the editable master format used by the editor and converter.

This document is generated from the structured documentation in [json-schedule](json-schedule).

---

## Top-Level Structure

```json
{
  "meta": { ... },
  "panelTypes": { ... },
  "rooms": [ ... ],
  "presenters": [ ... ],
  "panels": { ... },
  "timeline": [ ... ],
  "conflicts": [ ... ]
}
```

## Structures Overview

- [meta-v7.md](meta-v7.md) - Metadata with `nextPresenterId` and variant `"full"`
- [panelTypes-v7.md](panelTypes-v7.md) - Panel types hashmap keyed by prefix, with named color sets
- [rooms-v7.md](rooms-v7.md) - Room definitions with `is_break` flag
- [presenters-v7.md](presenters-v7.md) - Presenters with stable integer `id`, `always_shown`, and `always_grouped`
- [panels-v7.md](panels-v7.md) - Hierarchical panels hash (full format)
- [PanelPart-v5.md](PanelPart-v5.md) - Panel part objects (unchanged from v5)
- [PanelSession-v7.md](PanelSession-v7.md) - Panel session objects (`extras` renamed to `metadata`)
- [timeline-v7.md](timeline-v7.md) - Timeline markers referencing panelType prefix
- [conflicts-v7.md](conflicts-v7.md) - Conflict detection structures

## Structure Details

### [`meta`](json-schedule/meta-v7.md)

`meta` is a JSON object containing metadata about the schedule file itself.

**Access:** Public

**Status:** Supported in v7

**Key Fields:**

| Field             | Type    | Public | Description                                                  |
| ----------------- | ------- | ------ | ------------------------------------------------------------ |
| `title`           | string  | yes    | Display title for the schedule                               |
| `generated`       | string  | yes    | ISO 8601 UTC timestamp when the file was generated           |
| `version`         | integer | yes    | Schema version number (always `7` for this format)           |
| `variant`         | string  | yes    | Format variant: `"full"` for private, `"display"` for public |
| `generator`       | string  | yes    | Identifier of the tool that produced the file                |
| `startTime`       | string  | yes    | ISO 8601 UTC timestamp of the schedule start date            |
| `endTime`         | string  | yes    | ISO 8601 UTC timestamp of the schedule end date              |
| `nextPresenterId` | integer | no     | Next available presenter ID counter (full format only)       |
| `creator`         | string  | no     | Excel file creator/author (full format only)                 |
| `lastModifiedBy`  | string  | no     | Excel file last modified by (full format only)               |

*See full details in: [`meta-v7.md`](json-schedule/meta-v7.md)*

### [`panelTypes`](json-schedule/panelTypes-v7.md)

`panelTypes` is a JSON object (hashmap) keyed by uppercase prefix, where each value defines a category of panels.

**Access:** Public

**Status:** Supported in v7

**Key Fields:**

| Field         | Type           | Public | Description                                                   |
| ------------- | -------------- | ------ | ------------------------------------------------------------- |
| `kind`        | string         | yes    | Human-readable category name                                  |
| `colors`      | object         | yes    | Named color sets (see Color Sets below)                       |
| `isBreak`     | boolean        | yes    | True for break-type panels                                    |
| `isCafe`      | boolean        | yes    | True for café/social panels                                   |
| `isWorkshop`  | boolean        | yes    | True for workshop panels                                      |
| `isHidden`    | boolean        | yes    | True for hidden panel types (staff-only)                      |
| `isRoomHours` | boolean        | yes    | True for room-hours panels (e.g. Market Expo operating hours) |
| `isTimeline`  | boolean        | yes    | True for timeline/split panel types (merged from timeTypes)   |
| `isPrivate`   | boolean        | yes    | True for private panel types (e.g. Staff Meal)                |

*See full details in: [`panelTypes-v7.md`](json-schedule/panelTypes-v7.md)*

### [`rooms`](json-schedule/rooms-v7.md)

`rooms` is a JSON array where each entry represents a physical or virtual space where panels can be scheduled.

**Access:** Public

**Status:** Supported in v7

**Key Fields:**

| Field        | Type           | Public | Description                                    |
| ------------ | -------------- | ------ | ---------------------------------------------- |
| `uid`        | integer        | yes    | Unique room identifier from spreadsheet        |
| `short_name` | string         | yes    | Abbreviated room name for compact display      |
| `long_name`  | string         | yes    | Full room name                                 |
| `hotel_room` | string         | yes    | Physical hotel room identifier                 |
| `sort_key`   | integer        | yes    | Display sort order (lower = first, 1-indexed)  |
| `is_break`   | boolean        | yes    | True for virtual break rooms                   |

*See full details in: [`rooms-v7.md`](json-schedule/rooms-v7.md)*

### [`presenters`](json-schedule/presenters-v7.md)

`presenters` is a JSON array where each entry represents a person or group that can be assigned to panels.

**Access:** Public

**Status:** Supported in v7

**Key Fields:**

| Field            | Type           | Public | Description                                                                        |
| ---------------- | -------------- | ------ | ---------------------------------------------------------------------------------- |
| `name`           | string         | yes    | Display name                                                                       |
| `rank`           | string         | yes    | Role: `"guest"`, `"judge"`, `"staff"`, `"invited_guest"`, or `"fan_panelist"`      |
| `is_group`       | boolean        | yes    | True if this entry represents a group rather than an individual                    |
| `members`        | string[]       | yes    | For groups: list of individual member names. Empty for individuals                 |
| `groups`         | string[]       | yes    | For individuals: list of group names this person belongs to. Empty for non-members |
| `always_grouped` | boolean        | yes    | If true, this member always appears under their group name in credits              |
| `always_shown`   | boolean        | yes    | If true (on groups), the group name is shown even when not all members present     |

*See full details in: [`presenters-v7.md`](json-schedule/presenters-v7.md)*

### [`panels`](json-schedule/panels-v7.md)

`panels` is a JSON object keyed by **base ID** containing hierarchical panel data with base→part→session nesting.

**Access:** Private

**Status:** Supported in v7 (full format only)

**Key Fields:**

| Field                  | Type                           | Public | Description                                                      |
| ---------------------- | ------------------------------ | ------ | ---------------------------------------------------------------- |
| `id`                   | string                         | yes    | Base ID (same as hash key, e.g. `"GW097"`)                       |
| `name`                 | string                         | yes    | Display name of the panel                                        |
| `panelType`            | string \| null                 | yes    | Panel type prefix (e.g. `"GW"`), references panelTypes hash key  |
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
| `creditedPresenters`   | string[]                       | yes    | Individual presenter names who appear in credits                 |
| `uncreditedPresenters` | string[]                       | no     | Individual presenter names attending but suppressed from credits |
| `simpleTixEvent`       | string \| null                 | no     | Default SimpleTix admin portal link; sessions may override       |
| `parts`                | [PanelPart](PanelPart-v5.md)[] | yes    | Parts list; always at least one entry                            |

*See full details in: [`panels-v7.md`](json-schedule/panels-v7.md)*

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

### [`PanelSession`](json-schedule/PanelSession-v7.md)

`PanelSession` is an object representing a specific scheduled occurrence of a panel part.

**Access:** Private

**Status:** Supported in v7 (full format only)

**Key Fields:**

| Field                  | Type            | Public | Description                                                    |
| ---------------------- | --------------- | ------ | -------------------------------------------------------------- |
| `id`                   | string          | yes    | Full Uniq ID for this session (e.g. `"GW097P1"`, `"GP002"`)    |
| `sessionNum`           | integer \| null | yes    | Session number (e.g. `2` for `S2` suffix); `null` when none    |
| `description`          | string \| null  | yes    | Additive description for this session                          |
| `note`                 | string \| null  | yes    | Additive note for this session                                 |
| `prereq`               | string \| null  | yes    | Additive prerequisite text for this session                    |
| `altPanelist`          | string \| null  | yes    | Override credits text; takes precedence over part and base     |
| `roomIds`              | integer[]       | yes    | Room UIDs for this session; empty array if unscheduled         |
| `startTime`            | string \| null  | yes    | ISO 8601 local datetime; null if unscheduled                   |
| `endTime`              | string \| null  | yes    | ISO 8601 local datetime                                        |
| `duration`             | integer         | yes    | Duration in minutes                                            |
| `isFull`               | boolean         | yes    | True if this session is at capacity                            |
| `capacity`             | string \| null  | yes    | Per-session seat capacity override; null = use base value      |
| `seatsSold`            | integer \| null | no     | Number of seats already pre-sold via ticketing                 |
| `preRegMax`            | string \| null  | no     | Per-session pre-reg maximum; null = use base value             |
| `ticketUrl`            | string \| null  | yes    | Per-session ticket URL override; null = use base value         |
| `simpleTixEvent`       | string \| null  | no     | Per-session SimpleTix admin portal link; null = use base value |
| `hidePanelist`         | boolean         | no     | True to suppress presenter credits entirely for this session   |
| `creditedPresenters`   | string[]        | yes    | Additional credited presenter names for this session           |
| `uncreditedPresenters` | string[]        | no     | Additional uncredited presenter names for this session         |
| `notesNonPrinting`     | string \| null  | no     | Internal notes (not shown publicly)                            |
| `workshopNotes`        | string \| null  | no     | Notes for workshop staff                                       |
| `powerNeeds`           | string \| null  | no     | Power requirements                                             |
| `sewingMachines`       | boolean         | no     | True if sewing machines are required                           |
| `avNotes`              | string \| null  | no     | Audio/visual setup notes                                       |

*See full details in: [`PanelSession-v7.md`](json-schedule/PanelSession-v7.md)*

### [`timeline`](json-schedule/timeline-v7.md)

`timeline` is a JSON array of key time markers used for layout, navigation, and formatting.

**Access:** Public

**Status:** Supported in v7

**Key Fields:**

| Field         | Type           | Public | Description                                                           |
| ------------- | -------------- | ------ | --------------------------------------------------------------------- |
| `id`          | string         | yes    | Unique identifier for the time marker                                 |
| `startTime`   | string         | yes    | ISO 8601 UTC timestamp for the marker                                 |
| `description` | string         | yes    | Description of the time marker                                        |
| `panelType`   | string \| null | yes    | Panel type prefix, references [panelTypes](panelTypes-v7.md) hash key |
| `note`        | string \| null | yes    | Additional notes for the marker                                       |

*See full details in: [`timeline-v7.md`](json-schedule/timeline-v7.md)*

### [`conflicts`](json-schedule/conflicts-v7.md)

`conflicts` is an optional JSON array of detected scheduling conflicts at the top level.

**Access:** Public

**Status:** Supported in v7 (unchanged from v4)

**Key Fields:**

| Field       | Type           | Public | Description                                     |
| ----------- | -------------- | ------ | ----------------------------------------------- |
| `type`      | string         | yes    | `"room"`, `"presenter"`, or `"group_presenter"` |
| `room`      | string \| null | yes    | Room UID (for room conflicts)                   |
| `presenter` | string \| null | yes    | Presenter name (for presenter/group conflicts)  |
| `panel1`    | object         | yes    | `{ "id": "...", "name": "..." }`                |

*See full details in: [`conflicts-v7.md`](json-schedule/conflicts-v7.md)*

---

## Related Documentation

- [JSON Schedule Documentation](json-schedule/) - Complete structured documentation
- [Schedule JSON Format v4](json-format-v4.md) - This document describes version 4 of the schedule JSON format. V4 introduces timeline support and time types while maintaining backward compatibility with earlier versions.
- [Schedule JSON Format v5 - Private/Full](json-private-v5.md) - This document describes version 5 of the schedule JSON format, private/full variant. This format is produced and consumed by the Rust editor and converter for internal data storage and editing.
- [Schedule JSON Format v5 - Public/Widget](json-public-v5.md) - This document describes version 5 of the schedule JSON format, public/widget variant. This format is produced by the Rust converter or editor in public export mode and consumed by the schedule widget.
- [v6-Private](json-private-v6.md) - Private format documentation for JSON schedule format v6.
- [v6-Public](json-public-v6.md) - Public format documentation for JSON schedule format v6.
- [v7-Display](json-v7-display.md) - Display format documentation for JSON schedule format v7. This is the public-facing format consumed by the schedule widget.
- [v8-Full](json-v8-full.md) - Full format documentation for JSON schedule format v8. This is the editable master format used by the editor and converter, with support for persistent edit history via the optional `changeLog` field.
- [Display Format v9](json-v9-display.md) - **Access Level**: Public
**Status**: Supported
**Version**: 9

Public-facing schedule format with DisplayPresenter objects and filtered presenter list.
- [Full Format v9](json-v9-full.md) - **Access Level**: Private
**Status**: Supported
**Version**: 9

Complete internal schedule format with full presenter data and edit history support.

*This document is automatically generated. Do not edit directly.*
