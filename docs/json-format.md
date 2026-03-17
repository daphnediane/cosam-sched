# Schedule JSON Format

This document describes the JSON file format used by the schedule editor and
the calendar widget. The file is produced by either the Perl converter
(`converter/schedule_to_json`) or the Rust editor (`editor/`).

## Top-Level Structure

```json
{
  "meta": { ... },
  "events": [ ... ],
  "rooms": [ ... ],
  "panelTypes": [ ... ],
  "presenters": [ ... ],
  "conflicts": [ ... ]
}
```

All top-level keys are required except `conflicts`, which may be omitted when
there are no scheduling conflicts.

---

## `meta`

Metadata about the schedule file itself.

| Field       | Type    | Required | Description                                                                                       |
| ----------- | ------- | -------- | ------------------------------------------------------------------------------------------------- |
| `title`     | string  | yes      | Display title (e.g. "Cosplay America 2026 Schedule")                                              |
| `generated` | string  | yes      | ISO 8601 UTC timestamp of when the file was generated (e.g. `"2026-06-26T14:00:00Z"`)             |
| `version`   | integer | no       | Schema version number. Current version is `3`. Absent in older files (implies v1)                 |
| `generator` | string  | no       | Identifier of the tool that produced the file (e.g. `"cosam-editor 0.1.0"`, `"schedule_to_json"`) |

---

## `events`

Array of event objects. Each event represents a single scheduled item.

| Field         | Type            | Required | Description                                                                                                                                                                                                                           |
| ------------- | --------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `id`          | string          | yes      | Unique event ID, typically prefix + number (e.g. `"GP002"`, `"GW006P1"`)                                                                                                                                                              |
| `name`        | string          | yes      | Display name of the event                                                                                                                                                                                                             |
| `description` | string \| null  | no       | Long description text                                                                                                                                                                                                                 |
| `startTime`   | string          | yes      | ISO 8601 local time without timezone (e.g. `"2026-06-26T14:00:00"`)                                                                                                                                                                   |
| `endTime`     | string          | yes      | ISO 8601 local time without timezone                                                                                                                                                                                                  |
| `duration`    | integer         | yes      | Duration in minutes                                                                                                                                                                                                                   |
| `roomId`      | integer \| null | no       | References `rooms[].uid`. Null for events without a room (breaks, staff meals)                                                                                                                                                        |
| `panelType`   | string \| null  | no       | Panel type UID in format `"panel-type-{prefix}"` where prefix is lowercased (e.g. `"panel-type-gw"`). References `panelTypes[].uid`                                                                                                   |
| `kind`        | string \| null  | no       | Human-readable event type (e.g. `"Guest Workshop"`, `"Main Event"`). Derived from the panel type's `kind` field                                                                                                                       |
| `cost`        | string \| null  | no       | Cost as formatted string including currency symbol (e.g. `"$5.00"`, `"$120.00"`, `"TBD"`, `"model"`). Null means free                                                                                                                 |
| `capacity`    | string \| null  | no       | Maximum attendees as string (e.g. `"12"`)                                                                                                                                                                                             |
| `difficulty`  | string \| null  | no       | Skill level or difficulty rating                                                                                                                                                                                                      |
| `note`        | string \| null  | no       | Additional notes for the event                                                                                                                                                                                                        |
| `prereq`      | string \| null  | no       | Prerequisites text                                                                                                                                                                                                                    |
| `ticketUrl`   | string \| null  | no       | URL for ticket purchase                                                                                                                                                                                                               |
| `presenters`  | string[]        | yes      | List of presenter names associated with this event. Includes all presenters (credited and uncredited). Individual member names are used here, not group names (e.g. `["Pro", "Con"]` not `["Pros and Cons Cosplay"]`)                 |
| `credits`     | string[]        | yes      | Public-facing attribution list. Uses group names when all group members are present (e.g. `["Pros and Cons Cosplay"]` instead of `["Pro", "Con"]`). Omits uncredited/hidden presenters. See [Credits Generation](#credits-generation) |
| `conflicts`   | object[]        | yes      | List of scheduling conflicts for this event. Empty array when no conflicts. See [Per-Event Conflicts](#per-event-conflicts)                                                                                                           |
| `isFree`      | boolean         | yes      | True if the event has no cost                                                                                                                                                                                                         |
| `isFull`      | boolean         | yes      | True if the event is at capacity                                                                                                                                                                                                      |
| `isKids`      | boolean         | yes      | True for kids-only events                                                                                                                                                                                                             |

### `startTime` and `endTime` Format

Times are ISO 8601 local datetimes **without** a timezone suffix:
`YYYY-MM-DDTHH:MM:SS`. The timezone is assumed to be the event venue's local
time. The `duration` field must be consistent with the difference between
`startTime` and `endTime`.

### Cost Values

| Spreadsheet Value             | JSON `cost`                    | `isFree`                   |
| ----------------------------- | ------------------------------ | -------------------------- |
| empty / `*` / free / n/a / $0 | `null`                         | `true`                     |
| `kids`                        | `null`                         | `true` (`isKids` = `true`) |
| `TBD` / `T.B.D.`              | `"TBD"`                        | `false`                    |
| `model`                       | `"model"`                      | `false`                    |
| numeric or `$X.XX`            | `"$X.XX"` (currency formatted) | `false`                    |

### Credits Generation

The `credits` array is derived from `presenters` using group resolution:

1. **Always-grouped presenters** are added first using their group name.
2. For each remaining presenter:
   - If the presenter belongs to a **group** and **all members** of that group
     are in the event's `presenters`, the **group name** is used (e.g.
     `"Pros and Cons Cosplay"` instead of individual `"Pro"` and `"Con"`).
   - If only **some members** are present, each is shown as
     `"{member} of {group}"` (e.g. `"Con of Pros and Cons Cosplay"`).
   - Otherwise the individual name is used.
3. Duplicate names are suppressed.

Credits are the names shown publicly in the schedule display. The `presenters`
list retains the raw individual names for scheduling and conflict detection.

### Per-Event Conflicts

Each entry in the per-event `conflicts` array:

| Field               | Type           | Description                         |
| ------------------- | -------------- | ----------------------------------- |
| `type`              | string         | `"room"` or `"presenter"`           |
| `conflict_event_id` | string \| null | ID of the conflicting event         |
| `details`           | string \| null | Human-readable conflict description |

---

## `rooms`

Array of room objects. Rooms represent physical or virtual spaces.

| Field        | Type    | Required | Description                                                                       |
| ------------ | ------- | -------- | --------------------------------------------------------------------------------- |
| `uid`        | integer | yes      | Unique room identifier. Assigned from the spreadsheet; not necessarily sequential |
| `short_name` | string  | yes      | Abbreviated room name for compact display                                         |
| `long_name`  | string  | yes      | Full room name                                                                    |
| `hotel_room` | string  | yes      | Physical hotel room identifier (e.g. `"Salon F/G"`)                               |
| `sort_key`   | integer | yes      | Display sort order (lower = first). 1-indexed                                     |

Room UIDs are assigned based on the order they appear in the spreadsheet's
Rooms sheet. They are stable identifiers that must be consistent between the
`rooms` array and all `roomId` references in events.

Virtual rooms may exist for break events, day separators, or staff meals.
These virtual rooms may not appear in the `rooms` array but can still be
referenced by `roomId` in events.

---

## `panelTypes`

Array of panel type objects defining event categories.

| Field        | Type    | Required | Description                                                                                             |
| ------------ | ------- | -------- | ------------------------------------------------------------------------------------------------------- |
| `uid`        | string  | yes      | Unique identifier in format `"panel-type-{prefix}"` where prefix is lowercased (e.g. `"panel-type-gw"`) |
| `prefix`     | string  | yes      | Short prefix code, uppercase (e.g. `"GW"`, `"ME"`)                                                      |
| `kind`       | string  | no       | Human-readable category name (e.g. `"Guest Workshop"`)                                                  |
| `color`      | string  | yes      | Hex color code with `#` prefix (e.g. `"#FDEEB5"`)                                                       |
| `isBreak`    | boolean | yes      | True for break-type events                                                                              |
| `isCafe`     | boolean | no       | True for café/social events                                                                             |
| `isWorkshop` | boolean | yes      | True for workshop events                                                                                |
| `isHidden`   | boolean | no       | True for hidden panel types                                                                             |
| `isSplit`    | boolean | no       | True for reference split time                                                                           |

The `uid` field is the canonical reference used in `events[].panelType`.

### Hidden Panel Types

Panel types may have an `isHidden` boolean (not included in output). Hidden
panel types are filtered from the public schedule unless staff mode is enabled.
Events with hidden panel types (e.g. staff meals) are excluded from non-staff
output.

---

## `presenters`

Array of presenter objects.

| Field            | Type     | Required | Description                                                                                      |
| ---------------- | -------- | -------- | ------------------------------------------------------------------------------------------------ |
| `name`           | string   | yes      | Display name                                                                                     |
| `rank`           | string   | yes      | Role: `"guest"`, `"judge"`, `"staff"`, `"invited_guest"`, or `"fan_panelist"`                    |
| `is_group`       | boolean  | yes      | True if this entry represents a group (e.g. `"Pros and Cons Cosplay"`) rather than an individual |
| `members`        | string[] | yes      | For groups: list of individual member names. Empty for individuals                               |
| `groups`         | string[] | yes      | For individuals: list of group names this person belongs to. Empty for non-members               |
| `always_grouped` | boolean  | yes      | If true, this presenter always appears under their group name in credits                         |

### Group/Member Relationships

Presenters can be individuals or groups. Group membership is bidirectional:

- A **group** entry (e.g. `"Pros and Cons Cosplay"`) has `is_group: true` and
  `members: ["Pro", "Con"]`.
- Each **member** entry (e.g. `"Pro"`) has `groups: ["Pros and Cons Cosplay"]`.

This relationship is used during [credits generation](#credits-generation) to
determine whether to show the group name or individual names in the schedule.

### Presenter Column Parsing

In the spreadsheet, presenter columns use a naming convention to indicate rank
and grouping:

| Header Pattern     | Meaning                                        |
| ------------------ | ---------------------------------------------- |
| `Guest:Name`       | Named guest presenter                          |
| `Guest:Other`      | Column for listing guest names in cell values  |
| `Judge:Name`       | Named judge                                    |
| `Staff:Name`       | Named staff member                             |
| `Guest:Name=Group` | Named guest who is a member of the named group |
| `g1`, `g2`, etc.   | Legacy: positional guest columns               |
| `p1`, `p2`, etc.   | Legacy: positional panelist columns            |

---

## `conflicts` (Top-Level)

Optional array of detected scheduling conflicts.

| Field       | Type           | Description                                      |
| ----------- | -------------- | ------------------------------------------------ |
| `type`      | string         | `"room"` or `"presenter"` or `"group_presenter"` |
| `room`      | string \| null | Room UID (for room conflicts)                    |
| `presenter` | string \| null | Presenter name (for presenter/group conflicts)   |
| `event1`    | object         | `{ "id": "...", "name": "..." }`                 |
| `event2`    | object         | `{ "id": "...", "name": "..." }`                 |

---

## Differences: Editor vs. Converter

The editor is intended to replace the Perl converter. Status of alignment:

| Feature            | Converter                    | Editor                         | Status                                            |
| ------------------ | ---------------------------- | ------------------------------ | ------------------------------------------------- |
| `credits` field    | Present on all events        | Empty array (populated later)  | Partial: struct present, generation logic pending |
| `conflicts`        | Top-level + per-event        | Empty arrays (populated later) | Partial: structs present, detection logic pending |
| `panelType` format | `"panel-type-gw"` (uid)      | `"panel-type-gw"` (uid)        | **Done**                                          |
| `panelTypes[].uid` | Present                      | Present                        | **Done**                                          |
| `cost` format      | `"$5.00"` (currency)         | `"$5.00"` (currency)           | **Done**                                          |
| `duration`         | Correct (from spreadsheet)   | Sometimes wrong                | Bug: duration parsing                             |
| `endTime`          | Correct                      | Sometimes wrong                | Related to duration bug                           |
| Room UIDs          | From spreadsheet order       | From spreadsheet order         | **Done**                                          |
| Presenter model    | Full (groups, members)       | Full (groups, members)         | **Done**                                          |
| SPLIT events       | Generated for day separators | Not generated                  | Needs implementation                              |
| `color` on events  | Not present                  | Not present                    | **Done**                                          |
| `meta.version`     | `2`                          | `2`                            | **Done**                                          |
| `meta.generator`   | `"schedule_to_json"`         | `"cosam-editor {ver}"`         | **Done**                                          |

---

## Version History

- **v1** (implicit): Original format produced by the Perl converter. No
  `version` or `generator` fields in `meta`.
- **v2**: Adds `meta.version` and `meta.generator` fields. Both the Perl
  converter and the Rust editor now produce v2 output. The schema is otherwise
  backward-compatible with v1 (new fields have defaults, so v1 files still
  parse correctly). The editor adds `panelTypes[].isHidden`,
  `panelTypes[].isRoomHours`, `panelTypes[].bwColor`, `events[].hidePanelist`,
  and `events[].altPanelist` for spreadsheet round-tripping; these fields are
  omitted when not set.
- **v3**: Adds `panelTypes[].isSplit` field to indicate split events. Removes
  `events[].isWorkshop` and `events[].isBreak` fields (these are now only available
  in `panelTypes[]`). SPLIT events are always visible (never hidden) regardless of
  the Hidden field in the spreadsheet. Widget checks `panelTypes[].isSplit` and
  `panelTypes[].isBreak` first (V3) with fallback to inline event fields for V2
  compatibility.
