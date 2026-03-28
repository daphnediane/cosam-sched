# Display Format Presenters v9

**Access Level**: Public  
**Status**: Supported  
**Version**: v9-display

Public-facing presenter structure with flat sortKey and panel IDs for widget consumption.

## Fields

| Field         | Type            | Public | Description                                         |
| ------------- | --------------- | ------ | --------------------------------------------------- |
| name          | String          | ✓      | Presenter or group name                             |
| rank          | String          | ✓      | Presenter rank (guest, staff, fan_panelist, etc.)   |
| sortKey       | Integer         | ✓      | Sequential ordering key (0-based)                   |
| isGroup       | Boolean         | ✓      | True if this is a group                             |
| members       | `Array<String>` | ✓      | Group member names (empty for individuals)          |
| groups        | `Array<String>` | ✓      | Groups this presenter belongs to (empty for groups) |
| alwaysGrouped | Boolean         | ✓      | Always display under group name                     |
| alwaysShown   | Boolean         | ✓      | Always show group even with partial membership      |
| panelIds      | `Array<String>` | ✓      | Panel IDs where this presenter/group should appear  |

## Key Changes from v8 Display Format

- **DisplayPresenter**: New public-facing structure with flat `sortKey`
- **sortKey**: Sequential ordering computed from internal `PresenterSortRank`
- **panelIds**: Panel IDs where presenter/group should appear (bidirectional membership)
- **Filtered presenters**: Only includes presenters referenced by panels
- **Bidirectional membership**: Groups of individuals and members of groups included

## Bidirectional Group Membership Logic

The display format includes presenters through bidirectional group traversal:

1. **Individual → Group**: When panels list individuals, their groups (and groups of groups) are included
2. **Group → Individual**: When panels list groups, their direct members are included  
3. **Transitive groups only**: Individual → group → group → ... (stops at group level)
4. **Direct members only**: Group → individual (doesn't recurse into individual's groups)

This ensures:

- If "Pro" of "Pros and Cons Cosplay" presents solo, both "Pro" AND "Pros and Cons Cosplay" appear
- If "Imperial Storm Troopers" presents, both "105th" and "501st" appear (groups of groups)
- "Birthday Party Princesses" doesn't appear just because a 105th member belongs to it

## JSON Examples

### Display Format Presenter

```json
{
  "name": "John Doe",
  "rank": "guest",
  "sortKey": 15,
  "isGroup": false,
  "members": [],
  "groups": ["Guest Panel"],
  "alwaysGrouped": false,
  "alwaysShown": false,
  "panelIds": ["panel-001", "panel-045"]
}
```

### Display Format Group

```json
{
  "name": "Guest Panel",
  "rank": "guest",
  "sortKey": 32,
  "isGroup": true,
  "members": ["John Doe", "Jane Smith"],
  "groups": [],
  "alwaysGrouped": false,
  "alwaysShown": true,
  "panelIds": ["panel-001", "panel-023", "panel-045"]
}
```
