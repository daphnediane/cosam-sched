# PanelType Structure v7

`panelTypes` is a JSON object keyed by **panel type UID** containing category definitions for events with enhanced support for implicit break generation.

**Access:** Public

**Status:** Supported in v7

## Key Changes from v6

- **New Fields**: Added `isImplicit` and `isOvernight` for implicit break categorization
- **UID Format**: Panel type prefix (e.g., "BR", "IB") is now the primary UID
- **CSS Compatibility**: Widget converts prefix to `panel-type-{prefix}` format for styling

## Fields

| Field         | Type    | Public | Description                                           |
| ------------- | ------- | ------ | ----------------------------------------------------- |
| `prefix`      | string  | yes    | 2-letter prefix code, uppercase (also the object key) |
| `kind`        | string  | yes    | Human-readable category name                          |
| `color`       | string  | yes    | Hex color code with `#` prefix                        |
| `isBreak`     | boolean | yes    | True for break-type events                            |
| `isCafe`      | boolean | yes    | True for café/social events                           |
| `isWorkshop`  | boolean | yes    | True for workshop events                              |
| `isHidden`    | boolean | yes    | True for panel types not shown in UI                  |
| `isRoomHours` | boolean | yes    | True for room hours events                            |
| `isImplicit`  | boolean | yes    | True for automatically generated implicit breaks      |
| `isOvernight` | boolean | yes    | True for overnight break types                        |
| `isPrivate`   | boolean | yes    | True for private/internal events                      |
| `bwColor`     | string  | yes    | Black and white color for printing                    |

## Structure

```json
{
  "BR": {
    "prefix": "BR", 
    "kind": "Break",
    "color": "#CCCCCC",
    "isBreak": true,
    "isCafe": false,
    "isWorkshop": false,
    "isHidden": false,
    "isImplicit": false,
    "isOvernight": false,
    "isPrivate": false
  },
  "IB": {
    "prefix": "IB",
    "kind": "Implicit Break", 
    "color": "#CCCCCC",
    "isBreak": true,
    "isCafe": false,
    "isWorkshop": false,
    "isHidden": true,
    "isImplicit": true,
    "isOvernight": false,
    "isPrivate": false
  },
  "NB": {
    "prefix": "NB",
    "kind": "Overnight Break",
    "color": "#CCCCCC", 
    "isBreak": true,
    "isCafe": false,
    "isWorkshop": false,
    "isHidden": true,
    "isImplicit": true,
    "isOvernight": true,
    "isPrivate": false
  }
}
```

## Widget Compatibility

The widget converts v7 prefix UIDs to the legacy format for CSS styling:
- `uid: "BR"` → CSS class `panel-type-br`
- `uid: "IB"` → CSS class `panel-type-ib`
- `uid: "NB"` → CSS class `panel-type-nb`

This maintains backward compatibility with existing CSS while simplifying the JSON format.

## Auto-Generation Rules

Implicit break panel types are automatically created by the backend when:

1. Gaps of 3+ hours exist between events in non-hidden rooms
2. No explicit breaks are scheduled during the gap
3. The gap spans overnight (different days or crosses 4 AM boundary)

The system prefers "IB" and "NB" prefixes, but will use available 2-letter combinations if those are taken.
