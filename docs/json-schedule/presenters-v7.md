# `presenters`

`presenters` is a JSON array where each entry represents a person or group that can be assigned to panels.

## Access

Public

## Status

Supported in v7

## Fields

| Field            | Type           | Public | Description                                                                        |
| ---------------- | -------------- | ------ | ---------------------------------------------------------------------------------- |
| `id`             | integer        | yes    | Stable unique integer identifier (never reused)                                    |
| `name`           | string         | yes    | Display name                                                                       |
| `rank`           | string         | yes    | Role: `"guest"`, `"judge"`, `"staff"`, `"invited_guest"`, or `"fan_panelist"`      |
| `is_group`       | boolean        | yes    | True if this entry represents a group rather than an individual                    |
| `members`        | string[]       | yes    | For groups: list of individual member names. Empty for individuals                 |
| `groups`         | string[]       | yes    | For individuals: list of group names this person belongs to. Empty for non-members |
| `always_grouped` | boolean        | yes    | If true, this member always appears under their group name in credits              |
| `always_shown`   | boolean        | yes    | If true (on groups), the group name is shown even when not all members present     |
| `metadata`       | object \| null | no     | Optional key-value metadata (full format only)                                     |

## Description

Presenters can be individuals or groups. The array defines all presenters and groups for the schedule, used to populate the presenter filter dropdown and for credits generation.

### Stable Integer IDs

Each presenter has a stable integer `id` assigned from a monotonically increasing counter tracked in `meta.nextPresenterId`. IDs are never reused — if a presenter is removed, their ID is not recycled. This ensures stable references across edits.

### Group/Member Relationships

Group membership is bidirectional:

- A **group** entry (e.g. `"Pros and Cons Cosplay"`) has `is_group: true` and `members: ["Pro", "Con"]`
- Each **member** entry (e.g. `"Pro"`) has `groups: ["Pros and Cons Cosplay"]`

Groups of groups are supported: a group's `members` list may include names of other groups.

### Presenter Ranks

The `rank` field categorizes presenters by their role:

- `"guest"`: Guest presenters
- `"judge"`: Contest judges
- `"staff"`: Convention staff members
- `"invited_guest"`: Specially invited guests
- `"fan_panelist"`: Fan panelists

### Always Grouped vs Always Shown

These two flags control credit display and are set via spreadsheet header syntax:

| Flag             | Applies to  | Spreadsheet syntax | Effect                                                      |
| ---------------- | ----------- | ------------------ | ----------------------------------------------------------- |
| `always_grouped` | Individuals | `G:<Name=Group`    | Member always appears under group name, never individually  |
| `always_shown`   | Groups      | `G:Name==Group`    | Group name shown in credits even if not all members present |

#### Credit Display Logic

1. For each credited member, check their groups
2. If group has `always_shown` → show group name (even if not all members present)
3. If all group members are hosting → show group name
4. Otherwise show individual name
5. If group is `always_shown` and only some members present:
   - Members with `always_grouped` → show just group name
   - One member without `always_grouped` → show "Group (Member)"

## Examples

```json
[
  {
    "id": 1,
    "name": "December Wynn",
    "rank": "guest",
    "is_group": false,
    "members": [],
    "groups": [],
    "always_grouped": false,
    "always_shown": false
  },
  {
    "id": 2,
    "name": "Pros and Cons Cosplay",
    "rank": "guest",
    "is_group": true,
    "members": ["Pro", "Con"],
    "groups": [],
    "always_grouped": false,
    "always_shown": true
  },
  {
    "id": 3,
    "name": "Pro",
    "rank": "guest",
    "is_group": false,
    "members": [],
    "groups": ["Pros and Cons Cosplay"],
    "always_grouped": true,
    "always_shown": false
  }
]
```

## Notes

- The `id` field is new in v7 and must be unique across all presenters
- `always_shown` is new in v7; in v4–v6 only `always_grouped` existed
- In v4–v6, `==Group` incorrectly set `always_grouped` on the member; in v7 it correctly sets `always_shown` on the group
- `metadata` is only present in the full format and is stripped in the display variant
- Group membership must be defined in both directions (group lists members, members list groups)
