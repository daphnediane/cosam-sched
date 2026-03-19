# `timeTypes`

`timeTypes` is a JSON array where each entry defines a category of time markers used in the timeline.

## Access

Public

## Status

Supported in v4

## Fields

| Field    | Type   | Public | Description                                        |
| -------- | ------ | ------ | -------------------------------------------------- |
| `uid`    | string | yes    | Unique identifier in format `"time-type-{prefix}"` |
| `prefix` | string | yes    | Short prefix code, uppercase                       |
| `kind`   | string | yes    | Human-readable category name                       |

## Description

Time types define categories for timeline markers. These are used to classify different types of time-based divisions in the schedule.

### UID Format

The `uid` field follows the format `"time-type-{prefix}"` where `prefix` is the lowercased version of the `prefix` field. For example:

- `prefix: "SPLIT"` → `uid: "time-type-split"`
- `prefix: "SPLITDAY"` → `uid: "time-type-splitday"`

### Timeline Integration

Time types are referenced by [timeline](timeline-v4.md) entries via the `timeType` field. This allows timeline markers to be categorized and styled consistently.

### Generation

When converting from spreadsheet, this array is populated from panel types that have `Is Split` set to any truthy value, or begin with `"SP"` or `"SPLIT"`. When exporting to spreadsheet, time types will be stored with other prefixes in panel types.

## Examples

```json
[
  {
    "uid": "time-type-split",
    "prefix": "SPLIT",
    "kind": "Page split"
  },
  {
    "uid": "time-type-splitday",
    "prefix": "SPLITDAY", 
    "kind": "Split day"
  }
]
```

## Notes

- Time types are primarily used for timeline organization and display
- The `uid` field is the canonical reference used in `timeline[].timeType`
- Time types help categorize different types of schedule divisions (days, page breaks, etc.)
