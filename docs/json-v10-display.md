# Display Format v10

Public-facing schedule format with DisplayPresenter objects and filtered presenter list.

This document is generated from the structured documentation in [json-schedule](json-schedule).

---

## Top-Level Structure

```json
{
  "meta": Meta,
  "panels": Array<DisplayPanel>,
  "rooms": Array<Room>,
  "panelTypes": Object<PanelType>,
  "timeline": Array<TimelineEntry>,
  "presenters": Array<DisplayPresenter>
}
```

## Structures Overview

- [meta-v9.md](meta-v9.md) - Metadata structure (unchanged)
- [presenters-display-v9.md](presenters-display-v9.md) - DisplayPresenter with flat sortKey and panelIds (unchanged)
- [panels-display-v7.md](panels-display-v7.md) - Flattened panels with baked-in breaks
- [panelTypes-v7.md](panelTypes-v7.md) - Panel type definitions
- [rooms-v7.md](rooms-v7.md) - Room definitions
- [timeline-v7.md](timeline-v7.md) - Timeline markers

## Structure Details

### [`panels`](json-schedule/panels-display-v7.md)

`panels` is a JSON array where each entry represents one **session** - the smallest schedulable unit, flattened from the full hierarchical format. In v7, the display variant also includes baked-in implicit break panels.

**Access:** Public

**Status:** Supported in v7 (display format only)

**Key Fields:**

| Field         | Type            | Public | Description                                                     |
| ------------- | --------------- | ------ | --------------------------------------------------------------- |
| `id`          | string          | yes    | Full Uniq ID of this session (e.g. `"GW097P1S2"`, `"GP002"`)    |
| `baseId`      | string          | yes    | Base panel ID (e.g. `"GW097"`, `"GP002"`)                       |
| `partNum`     | integer \| null | yes    | Part number; `null` when no part subdivision                    |
| `sessionNum`  | integer \| null | yes    | Session number; `null` when no session subdivision              |
| `name`        | string          | yes    | Display name (from base panel)                                  |
| `panelType`   | string \| null  | yes    | Panel type prefix (e.g. `"GW"`), references panelTypes hash key |
| `roomIds`     | integer[]       | yes    | Room UIDs for this session; empty array if unscheduled          |
| `startTime`   | string \| null  | yes    | ISO 8601 local datetime; null if unscheduled                    |
| `endTime`     | string \| null  | yes    | ISO 8601 local datetime                                         |
| `duration`    | integer         | yes    | Duration in minutes                                             |
| `description` | string \| null  | yes    | Effective description (base + part + session concatenated)      |
| `note`        | string \| null  | yes    | Effective note                                                  |
| `prereq`      | string \| null  | yes    | Effective prerequisite text                                     |
| `cost`        | string \| null  | yes    | Cost string from base (see Cost Values in v4 documentation)     |
| `capacity`    | string \| null  | yes    | Effective seat capacity (session override or base default)      |
| `difficulty`  | string \| null  | yes    | Skill level indicator (from base)                               |
| `ticketUrl`   | string \| null  | yes    | Effective ticket URL (session override or base default)         |
| `isFree`      | boolean         | yes    | True if no additional cost                                      |
| `isFull`      | boolean         | yes    | True if this session is at capacity                             |
| `isKids`      | boolean         | yes    | True for kids-only panels                                       |
| `credits`     | string[]        | yes    | Formatted credit strings for public display                     |

*See full details in: [`panels-display-v7.md`](json-schedule/panels-display-v7.md)*

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

---

## Related Documentation

- [JSON Schedule Documentation](json-schedule/) - Complete structured documentation
- [Full Format v10](json-v10-full.md) - Complete internal schedule format with flat presenter relationship fields and edit history support.
- [Display Format v9](json-v9-display.md) - Public-facing schedule format with DisplayPresenter objects and filtered presenter list.
- [Full Format v9](json-v9-full.md) - Complete internal schedule format with full presenter data and edit history support.

*This document is automatically generated. Do not edit directly.*
