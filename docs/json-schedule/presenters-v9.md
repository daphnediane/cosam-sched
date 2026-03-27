# Presenters Structure v9

**Access Level**: Private  
**Status**: Supported  
**Version**: v9-full

Internal presenter structure with PresenterSortRank for full format editing.

## Fields

| Field       | Type              | Public | Description                                       |
| ----------- | ----------------- | ------ | ------------------------------------------------- |
| id          | Integer           | ✗      | Stable presenter ID (private format only)         |
| name        | String            | ✓      | Presenter or group name                           |
| rank        | String            | ✓      | Presenter rank (guest, staff, fan_panelist, etc.) |
| sortRank    | PresenterSortRank | ✗      | Internal sorting data (private format only)       |
| isMember    | PresenterMember   | ✗      | Group membership data (private format only)       |
| isGrouped   | PresenterGroup    | ✗      | Group data and members (private format only)      |
| changeState | String            | ✗      | Change tracking (private format only)             |

### PresenterSortRank Structure

| Field       | Type    | Description                                      |
| ----------- | ------- | ------------------------------------------------ |
| columnIndex | Integer | Column index from spreadsheet (0 = People table) |
| rowIndex    | Integer | Row index within column                          |
| memberIndex | Integer | 0 = group, 1 = individual member                 |

## Display Format Presenters

The display format uses `DisplayPresenter` with simplified structure for public consumption.

### Fields

| Field         | Type          | Public | Description                                         |
| ------------- | ------------- | ------ | --------------------------------------------------- |
| name          | String        | ✓      | Presenter or group name                             |
| rank          | String        | ✓      | Presenter rank (guest, staff, fan_panelist, etc.)   |
| sortKey       | Integer       | ✓      | Sequential ordering key (0-based)                   |
| isGroup       | Boolean       | ✓      | True if this is a group                             |
| members       | Array<String> | ✓      | Group member names (empty for individuals)          |
| groups        | Array<String> | ✓      | Groups this presenter belongs to (empty for groups) |
| alwaysGrouped | Boolean       | ✓      | Always display under group name                     |
| alwaysShown   | Boolean       | ✓      | Always show group even with partial membership      |
| panelIds      | Array<String> | ✓      | Panel IDs where this presenter/group should appear  |

## Key Changes from v8

- **PresenterSortRank**: Replaces separate `columnRank` and `indexRank` fields
- **memberIndex**: Eliminates index-doubling hack (0 = group, 1 = member)

## JSON Example

### Full Format Presenter

```json
{
  "id": 42,
  "name": "John Doe",
  "rank": "guest",
  "sortRank": {
    "columnIndex": 5,
    "rowIndex": 0,
    "memberIndex": 1
  },
  "isMember": {
    "NotMember": {}
  },
  "isGrouped": {
    "NotGroup": {}
  }
}
```
