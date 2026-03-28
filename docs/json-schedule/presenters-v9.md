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

For the public-facing `DisplayPresenter` structure, see [Display Format Presenters v9](presenters-display-v9.md).

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
