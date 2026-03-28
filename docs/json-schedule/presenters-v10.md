# Presenters Structure v10

**Access Level**: Private  
**Status**: Current  
**Version**: v10-full

Internal presenter structure with flat relationship fields and PresenterSortRank.

## Fields

| Field         | Type              | Public | Description                                           |
| ------------- | ----------------- | ------ | ----------------------------------------------------- |
| name          | String            | ✓      | Presenter or group name                               |
| rank          | String            | ✓      | Presenter rank (guest, staff, fan_panelist, etc.)     |
| isGroup       | Boolean           | ✓      | True if this presenter is a group                     |
| members       | Array\<String>    | ✓      | Group member names (empty for individuals)            |
| groups        | Array\<String>    | ✓      | Groups this presenter belongs to (empty for groups)   |
| alwaysGrouped | Boolean           | ✓      | Always display this member under its group name       |
| alwaysShown   | Boolean           | ✓      | Always show group even with partial member attendance |
| sortRank      | PresenterSortRank | ✗      | Internal sorting data (private format only)           |
| metadata      | Object            | ✗      | Arbitrary key-value metadata (optional, private only) |

### PresenterSortRank Structure

| Field       | Type    | Description                                      |
| ----------- | ------- | ------------------------------------------------ |
| columnIndex | Integer | Column index from spreadsheet (0 = People table) |
| rowIndex    | Integer | Row index within column                          |
| memberIndex | Integer | 0 = group/standalone, 1 = individual member      |

## Key Changes from v9

- **Flat relationship fields**: `isGroup`, `members`, `groups`, `alwaysGrouped`, `alwaysShown` replace the enum-based `isMember`/`isGrouped` fields
- **RelationshipManager**: Relationships are managed internally as edges; flat fields are populated from `RelationshipManager` on save
- **Backward-compatible deserialization**: Loading still accepts the old v9 enum format (`isMember`/`isGrouped`) for files saved by earlier versions

## Relationship Semantics

- **Groups**: A presenter with `isGroup: true` represents a named group (e.g. "Pros and Cons Cosplay")
- **Members**: Individual presenters list their groups in `groups`; groups list their members in `members`
- **alwaysGrouped**: When true, this member's name always appears with its group (e.g. "Pro of Pros and Cons Cosplay")
- **alwaysShown**: When true, the group name always appears in credits even when only some members are present

## JSON Example

### Individual Presenter (member of a group)

```json
{
  "name": "Pro",
  "rank": "guest",
  "isGroup": false,
  "members": [],
  "groups": ["Pros and Cons Cosplay"],
  "alwaysGrouped": true,
  "alwaysShown": false,
  "sortRank": {
    "columnIndex": 5,
    "rowIndex": 0,
    "memberIndex": 1
  }
}
```

### Group Presenter

```json
{
  "name": "Pros and Cons Cosplay",
  "rank": "guest",
  "isGroup": true,
  "members": ["Pro", "Con"],
  "groups": [],
  "alwaysGrouped": false,
  "alwaysShown": true,
  "sortRank": {
    "columnIndex": 5,
    "rowIndex": 0,
    "memberIndex": 0
  }
}
```

### Standalone Presenter

```json
{
  "name": "MC Host",
  "rank": "guest",
  "isGroup": false,
  "members": [],
  "groups": [],
  "alwaysGrouped": false,
  "alwaysShown": false,
  "sortRank": {
    "columnIndex": 0,
    "rowIndex": 0
  }
}
```
