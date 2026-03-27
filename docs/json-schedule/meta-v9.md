# Meta Structure v9

**Access Level**: Public/Private  
**Status**: Supported  
**Versions**: v9-full, v9-display

Metadata structure common to both full and display format variants.

## Fields

| Field | Type | Public | Description |
| ----- | ---- | ------ | ----------- |
| title | String | ✓ | Schedule title (e.g., "Event Schedule 2026") |
| version | Integer | ✓ | Format version (9) |
| variant | String | ✓ | Format variant: "full" or "display" |
| generator | String | ✓ | Generator software and version |
| generated | String | ✓ | ISO 8601 timestamp when file was generated |
| creator | String | ✗ | Excel file creator (private format only) |
| lastModifiedBy | String | ✗ | Excel last modifier (private format only) |
| modified | String | ✓ | Excel last modified timestamp |
| startTime | String | ✓ | Schedule start time (ISO 8601) |
| endTime | String | ✓ | Schedule end time (ISO 8601) |
| nextPresenterId | Integer | ✗ | Next presenter ID to assign (private format only) |

## Key Changes from v8

- **No field changes** - v9 maintains the same metadata structure as v8
- Version number bumped to 9 to reflect PresenterSortRank changes

## JSON Example

```json
{
  "title": "Event Schedule 2026",
  "version": 9,
  "variant": "display",
  "generator": "cosam-editor 0.1.0",
  "generated": "2026-03-26T22:00:00Z",
  "modified": "2026-03-26T21:45:00Z",
  "startTime": "2026-05-29T17:00:00Z",
  "endTime": "2026-06-01T15:00:00Z"
}
```
