# Full Format v9

Complete internal schedule format with full presenter data and edit history support.

This document is generated from the structured documentation in [json-schedule](json-schedule).

---

## Top-Level Structure

```json
{
  "meta": Meta,
  "conflicts": Array<ScheduleConflict>,
  "panelSets": Object<PanelSet>,
  "panelTypes": Object<PanelType>,
  "rooms": Array<Room>,
  "presenters": Array<Presenter>,
  "timeline": Array<TimelineEntry>,
  "importedSheets": ImportedSheetPresence,
  "changeLog": Array<EditCommand>
}
```

## Structures Overview

- [meta-v9.md](meta-v9.md) - Metadata structure
- [presenters-v9.md](presenters-v9.md) - Presenters with PresenterSortRank
- [PanelSet-v9.md](PanelSet-v9.md) - Flat panel sets
- [Panel-v9.md](Panel-v9.md) - Self-contained panel objects with TimeRange timing
- [panelTypes-v7.md](panelTypes-v7.md) - Panel type definitions
- [rooms-v7.md](rooms-v7.md) - Room definitions
- [timeline-v7.md](timeline-v7.md) - Timeline markers
- [conflicts-v7.md](conflicts-v7.md) - Conflict detection
- [changeLog-v8.md](changeLog-v8.md) - Edit history
- [ImportedSheetPresence-v6.md](ImportedSheetPresence-v6.md) - Sheet tracking

## Structure Details

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

### [`changeLog`](json-schedule/changeLog-v8.md)

`changeLog` is a JSON object containing the edit history for a schedule file, enabling persistent undo/redo functionality across application sessions.

**Access:** Private

**Status:** Introduced in v8

**Key Fields:**

| Field       | Type                         | Public | Description                                                 |
| ----------- | ---------------------------- | ------ | ----------------------------------------------------------- |
| `undoStack` | array of EditCommand objects | no     | Stack of edits that can be undone (newest first)            |
| `redoStack` | array of EditCommand objects | no     | Stack of edits that can be redone (newest first)            |

*See full details in: [`changeLog-v8.md`](json-schedule/changeLog-v8.md)*

---

## Related Documentation

- [JSON Schedule Documentation](json-schedule/) - Complete structured documentation
- [Schedule JSON Format v4](json-format-v4.md) - This document describes version 4 of the schedule JSON format. V4 introduces timeline support and time types while maintaining backward compatibility with earlier versions.
- [Schedule JSON Format v5 - Private/Full](json-private-v5.md) - This document describes version 5 of the schedule JSON format, private/full variant. This format is produced and consumed by the Rust editor and converter for internal data storage and editing.
- [Schedule JSON Format v5 - Public/Widget](json-public-v5.md) - This document describes version 5 of the schedule JSON format, public/widget variant. This format is produced by the Rust converter or editor in public export mode and consumed by the schedule widget.
- [v6-Private](json-private-v6.md) - Private format documentation for JSON schedule format v6.
- [v6-Public](json-public-v6.md) - Public format documentation for JSON schedule format v6.
- [v7-Display](json-v7-display.md) - Display format documentation for JSON schedule format v7. This is the public-facing format consumed by the schedule widget.
- [v7-Full](json-v7-full.md) - Full format documentation for JSON schedule format v7. This is the editable master format used by the editor and converter.
- [v8-Full](json-v8-full.md) - Full format documentation for JSON schedule format v8. This is the editable master format used by the editor and converter, with support for persistent edit history via the optional `changeLog` field.
- [Display Format v9](json-v9-display.md) - Public-facing schedule format with DisplayPresenter objects and filtered presenter list.

*This document is automatically generated. Do not edit directly.*
