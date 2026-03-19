# `panelTypes`

`panelTypes` is a JSON array where each entry defines a category of events.

## Access

Public

## Status

Supported in v4

## Fields

| Field        | Type    | Public | Description                                         |
| ------------ | ------- | ------ | --------------------------------------------------- |
| `uid`        | string  | yes    | Unique identifier in format `"panel-type-{prefix}"` |
| `prefix`     | string  | yes    | Short prefix code, uppercase                        |
| `kind`       | string  | yes    | Human-readable category name                        |
| `color`      | string  | yes    | Hex color code with `#` prefix                      |
| `isBreak`    | boolean | yes    | True for break-type events                          |
| `isCafe`     | boolean | yes    | True for café/social events                         |
| `isWorkshop` | boolean | yes    | True for workshop events                            |
| `isHidden`   | boolean | yes    | True for hidden panel types                         |

## Description

Panel types define categories of events and control how they are displayed and processed. Each panel type has a unique UID that is referenced by events.

### UID Format

The `uid` field follows the format `"panel-type-{prefix}"` where `prefix` is the lowercased version of the `prefix` field. For example:

- `prefix: "GW"` → `uid: "panel-type-gw"`
- `prefix: "GP"` → `uid: "panel-type-gp"`

### Color Codes

Colors are specified as hex codes with a `#` prefix (e.g. `"#FDEEB5"`, `"#B5D8FD"`). These colors are used in the schedule widget to visually distinguish different panel types.

### Hidden Panel Types

Panel types may have `isHidden: true`. Hidden panel types are filtered from the public schedule unless staff mode is enabled. Events with hidden panel types (e.g. staff meals) are excluded from non-staff output.

## Examples

```json
[
  {
    "uid": "panel-type-gp",
    "prefix": "GP",
    "kind": "Guest Panel",
    "color": "#FDEEB5",
    "isBreak": false,
    "isCafe": false,
    "isWorkshop": false,
    "isHidden": false
  },
  {
    "uid": "panel-type-gw",
    "prefix": "GW",
    "kind": "Guest Workshop",
    "color": "#B5D8FD",
    "isBreak": false,
    "isCafe": false,
    "isWorkshop": true,
    "isHidden": false
  },
  {
    "uid": "panel-type-br",
    "prefix": "BR",
    "kind": "Break",
    "color": "#E8E8E8",
    "isBreak": true,
    "isCafe": false,
    "isWorkshop": false,
    "isHidden": true
  }
]
```

## Notes

- The `uid` field is the canonical reference used in `events[].panelType`
- `isWorkshop` indicates events that require registration and have capacity limits
- `isBreak` events are typically unscheduled or have special timing
- `isCafe` events are social gatherings that may have different scheduling rules
