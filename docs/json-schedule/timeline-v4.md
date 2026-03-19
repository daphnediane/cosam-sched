# `timeline`

`timeline` is a JSON array of key time markers used for layout, navigation, and formatting.

## Access

Public

## Status

Supported in v4

## Fields

| Field         | Type           | Public | Description                                                  |
| ------------- | -------------- | ------ | ------------------------------------------------------------ |
| `id`          | string         | yes    | Unique identifier for the time marker                        |
| `startTime`   | string         | yes    | ISO 8601 UTC timestamp for the marker                        |
| `description` | string         | yes    | Description of the time marker                               |
| `timeType`    | string \| null | yes    | Time type UID, references [timeTypes](timeTypes-v4.md)[].uid |
| `note`        | string \| null | yes    | Additional notes for the event                               |

## Description

Timeline entries represent key time markers that divide the schedule into logical sections. These are used for layout, navigation, and formatting in the schedule widget.

### Generation

When converting from spreadsheet, this array is populated with events that have panel types with `Is Split` set to any truthy value, or begin with `"SP"` or `"SPLIT"`. When exporting to spreadsheet, these use a duration of 30 minutes, and the endTime will be calculated as startTime + 30 minutes.

### Time Type References

The `timeType` field references entries in the [timeTypes](timeTypes-v4.md) array using the UID format `"time-type-{prefix}"`. This allows timeline markers to be categorized and styled consistently.

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
    "timeType": "time-type-split",
    "note": "Opening ceremonies"
  },
  {
    "id": "SPLIT02", 
    "startTime": "2026-06-27T08:00:00Z",
    "description": "Friday Morning",
    "timeType": "time-type-split",
    "note": null
  },
  {
    "id": "SPLITDAY01",
    "startTime": "2026-06-28T00:00:00Z",
    "description": "Saturday Start",
    "timeType": "time-type-splitday",
    "note": "Full day programming"
  }
]
```

## Notes

- Timeline markers are not schedulable events but structural divisions
- The `id` field is used for internal reference and should be unique
- Timeline entries are typically displayed as headers or section dividers in the schedule widget
- Duration is fixed at 30 minutes when exporting to spreadsheet format
