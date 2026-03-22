# `timeline`

`timeline` is a JSON array of key time markers used for layout, navigation, and formatting.

## Access

Public

## Status

Supported in v7

## Fields

| Field         | Type           | Public | Description                                                           |
| ------------- | -------------- | ------ | --------------------------------------------------------------------- |
| `id`          | string         | yes    | Unique identifier for the time marker                                 |
| `startTime`   | string         | yes    | ISO 8601 UTC timestamp for the marker                                 |
| `description` | string         | yes    | Description of the time marker                                        |
| `panelType`   | string \| null | yes    | Panel type prefix, references [panelTypes](panelTypes-v7.md) hash key |
| `note`        | string \| null | yes    | Additional notes for the marker                                       |
| `metadata`    | object \| null | no     | Optional key-value metadata (full format only)                        |

## Description

Timeline entries represent key time markers that divide the schedule into logical sections. These are used for layout, navigation, and formatting in the schedule widget.

### Panel Type References

The `panelType` field references entries in the [panelTypes](panelTypes-v7.md) hashmap by prefix key. In v4–v6, this field was named `timeType` and used the format `"time-type-{prefix}"`. In v7, it references the prefix directly since timeline types are merged into panelTypes with `isTimeline: true`.

### Generation

When converting from spreadsheet, this array is populated with panels whose panel type has `isTimeline: true` (prefix starts with `"SP"` or `"SPLIT"`, or has `Is TimeLine`/`Is Split` set). When exporting to spreadsheet, these use a duration of 30 minutes.

### Common Timeline Markers

- **Day splits**: Mark the beginning of each day
- **Page splits**: Indicate where printed schedule pages should divide
- **Section breaks**: Mark major transitions in the schedule

## Examples

```json
[
  {
    "id": "SPLIT01",
    "startTime": "2026-06-26T17:00:00Z",
    "description": "Thursday Evening",
    "panelType": "SPLIT",
    "note": "Opening ceremonies"
  },
  {
    "id": "SPLIT02",
    "startTime": "2026-06-27T08:00:00Z",
    "description": "Friday Morning",
    "panelType": "SPLIT",
    "note": null
  },
  {
    "id": "SPLITDAY01",
    "startTime": "2026-06-28T00:00:00Z",
    "description": "Saturday Start",
    "panelType": "SPLITDAY",
    "note": "Full day programming"
  }
]
```

## Notes

- Timeline markers are not schedulable panels but structural divisions
- The `panelType` field now references panelTypes hash keys directly (was `timeType` in v4–v6)
- The separate `timeTypes` array from v4–v6 is removed; timeline types are in panelTypes with `isTimeline: true`
- Duration is fixed at 30 minutes when exporting to spreadsheet format
- `metadata` is only present in the full format and is stripped in the display variant
