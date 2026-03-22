# `panelTypes`

`panelTypes` is a JSON object (hashmap) keyed by uppercase prefix, where each value defines a category of panels.

## Access

Public

## Status

Supported in v7

## Fields

| Field         | Type           | Public | Description                                                   |
| ------------- | -------------- | ------ | ------------------------------------------------------------- |
| `kind`        | string         | yes    | Human-readable category name                                  |
| `colors`      | object         | yes    | Named color sets (see Color Sets below)                       |
| `isBreak`     | boolean        | yes    | True for break-type panels                                    |
| `isCafe`      | boolean        | yes    | True for café/social panels                                   |
| `isWorkshop`  | boolean        | yes    | True for workshop panels                                      |
| `isHidden`    | boolean        | yes    | True for hidden panel types (staff-only)                      |
| `isRoomHours` | boolean        | yes    | True for room-hours panels (e.g. Market Expo operating hours) |
| `isTimeline`  | boolean        | yes    | True for timeline/split panel types (merged from timeTypes)   |
| `isPrivate`   | boolean        | yes    | True for private panel types (e.g. Staff Meal)                |
| `metadata`    | object \| null | no     | Optional key-value metadata (full format only)                |

## Description

Panel types define categories of panels and control how they are displayed and processed. In v7, the panel types structure changed from an array to a hashmap keyed by the uppercase prefix string. The `uid` and `prefix` fields are removed — the hashmap key **is** the prefix.

### Key Format

Keys are uppercase prefix strings:

- **Spreadsheet-sourced**: 2-letter uppercase prefixes (e.g. `"GP"`, `"GW"`, `"BR"`)
- **Synthetic**: Non-alphabetic prefixes for system-generated types (e.g. `"%IB"` for Implicit Break, `"%NB"` for Overnight Break)

Panels reference their panel type by prefix directly (e.g. `"panelType": "GP"` not `"panel-type-gp"`).

### Color Sets

The `colors` field is an object with named color set entries. Each key is a color set name and the value is a CSS hex color string:

```json
{
  "color": "#E2F9D7",
  "bw": "#CCCCCC"
}
```

Standard color set names:

- `"color"`: Primary display color
- `"bw"`: Monochrome/black-and-white alternative

Additional keys may be added for per-theme overrides without schema changes.

### Timeline Types (merged from timeTypes)

In v4–v6, timeline panel types were stored in a separate `timeTypes` array. In v7, they are merged into `panelTypes` with `isTimeline: true`. Timeline types are panel types whose prefix starts with `"SP"` or `"SPLIT"`, or whose spreadsheet row has `Is TimeLine` / `Is Split` set to a truthy value.

### Hidden and Private Panel Types

- **`isHidden`**: Panel type is filtered from the public schedule unless staff mode is enabled
- **`isPrivate`**: Panel type is for internal use (e.g. Staff Meal, ZZ-prefix panels). Private panels are excluded from the display variant entirely.

### CSS Class Generation

For widget styling, the CSS class is derived from the prefix: `panel-type-{prefix.toLowerCase()}`. For example, prefix `"GP"` → class `panel-type-gp`.

## Examples

```json
{
  "GP": {
    "kind": "Guest Panel",
    "colors": { "color": "#E2F9D7", "bw": "#CCCCCC" },
    "isBreak": false,
    "isCafe": false,
    "isWorkshop": false,
    "isHidden": false,
    "isRoomHours": false,
    "isTimeline": false,
    "isPrivate": false
  },
  "GW": {
    "kind": "Guest Workshop",
    "colors": { "color": "#FDEEB5", "bw": "#E0E0E0" },
    "isBreak": false,
    "isCafe": false,
    "isWorkshop": true,
    "isHidden": false,
    "isRoomHours": false,
    "isTimeline": false,
    "isPrivate": false
  },
  "BR": {
    "kind": "Break",
    "colors": { "color": "#E8E8E8", "bw": "#F0F0F0" },
    "isBreak": true,
    "isCafe": false,
    "isWorkshop": false,
    "isHidden": true,
    "isRoomHours": false,
    "isTimeline": false,
    "isPrivate": false
  },
  "SPLIT": {
    "kind": "Page Split",
    "colors": {},
    "isBreak": false,
    "isCafe": false,
    "isWorkshop": false,
    "isHidden": true,
    "isRoomHours": false,
    "isTimeline": true,
    "isPrivate": false
  },
  "%IB": {
    "kind": "Implicit Break",
    "colors": { "color": "#F5F5F5" },
    "isBreak": true,
    "isCafe": false,
    "isWorkshop": false,
    "isHidden": false,
    "isRoomHours": false,
    "isTimeline": false,
    "isPrivate": false
  }
}
```

## Notes

- The hashmap key is the canonical identifier; no separate `uid` or `prefix` field exists
- `isWorkshop` indicates panels that require registration and have capacity limits
- `isBreak` panels are typically stretched across inactive rooms in the display variant
- `isCafe` panels are social gatherings that may have different scheduling rules
- `isRoomHours` panels represent room operating hours and do not conflict with subpanels
- `isTimeline` replaces the separate `timeTypes` array from v4–v6
- `metadata` is only present in the full format and is stripped in the display variant
- Boolean fields that are `false` may be omitted from the JSON
