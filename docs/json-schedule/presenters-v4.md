# `presenters`

`presenters` is a JSON array where each entry represents a person or group that can be assigned to events.

## Access

Public

## Status

Supported in v4

## Fields

| Field            | Type     | Public | Description                                                                        |
| ---------------- | -------- | ------ | ---------------------------------------------------------------------------------- |
| `name`           | string   | yes    | Display name                                                                       |
| `rank`           | string   | yes    | Role: `"guest"`, `"judge"`, `"staff"`, `"invited_guest"`, or `"fan_panelist"`      |
| `is_group`       | boolean  | yes    | True if this entry represents a group rather than an individual                    |
| `members`        | string[] | yes    | For groups: list of individual member names. Empty for individuals                 |
| `groups`         | string[] | yes    | For individuals: list of group names this person belongs to. Empty for non-members |
| `always_grouped` | boolean  | yes    | If true, this presenter always appears under their group name in credits           |

## Description

Presenters can be individuals or groups. The array defines all presenters and groups for the schedule, used to populate the presenter filter dropdown and for credits generation.

### Group/Member Relationships

Presenters can be individuals or groups. Group membership is bidirectional:

- A **group** entry (e.g. `"Pros and Cons Cosplay"`) has `is_group: true` and `members: ["Pro", "Con"]`
- Each **member** entry (e.g. `"Pro"`) has `groups: ["Pros and Cons Cosplay"]`

This relationship is used during credits generation to determine whether to show the group name or individual names in the schedule.

### Presenter Ranks

The `rank` field categorizes presenters by their role:

- `"guest"`: Guest presenters
- `"judge"`: Contest judges
- `"staff"`: Convention staff members
- `"invited_guest"`: Specially invited guests
- `"fan_panelist"`: Fan panelists

### Always Grouped

When `always_grouped` is true, this presenter always appears under their group name in credits, regardless of whether all group members are present in the event.

## Examples

```json
[
  {
    "name": "December Wynn",
    "rank": "guest",
    "is_group": false,
    "members": [],
    "groups": [],
    "always_grouped": false
  },
  {
    "name": "Pros and Cons Cosplay",
    "rank": "guest",
    "is_group": true,
    "members": ["Pro", "Con"],
    "groups": [],
    "always_grouped": false
  },
  {
    "name": "Pro",
    "rank": "guest", 
    "is_group": false,
    "members": [],
    "groups": ["Pros and Cons Cosplay"],
    "always_grouped": false
  },
  {
    "name": "Con",
    "rank": "guest",
    "is_group": false,
    "members": [],
    "groups": ["Pros and Cons Cosplay"],
    "always_grouped": false
  },
  {
    "name": "Staff Member",
    "rank": "staff",
    "is_group": false,
    "members": [],
    "groups": [],
    "always_grouped": false
  }
]
```

## Notes

- The presenter array is used for filter dropdowns and credits generation
- Group membership must be defined in both directions (group lists members, members list groups)
- Individual presenter names are used in `events[].presenters` for scheduling
- Group names may appear in `events[].credits` based on membership resolution
