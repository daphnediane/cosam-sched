# Schedule JSON Format v7 - Public/Widget

This document describes version 7 of the schedule JSON format, public/widget variant. This format is produced by the Rust converter or editor in public export mode and consumed by the schedule widget.

## Key Changes from v6

- **Prefix-based panelType**: Changed from full UIDs (`"panel-type-br"`) to 2-letter prefixes (`"BR"`)
- **Backend implicit breaks**: Implicit breaks are now generated on the backend, not client-side
- **Enhanced PanelType**: Added `isImplicit` and `isOvernight` fields for break categorization
- **BREAK room**: Special room for auto-generated breaks with hidden sort_key

## Top-Level Structure

```json
{
  "meta": { ... },
  "panels": [ ... ],
  "rooms": [ ... ],
  "panelTypes": [ ... ],
  "timeTypes": [ ... ],
  "timeline": [ ... ],
  "presenters": [ ... ]
}
```

## Structures Overview

- [meta](meta-v5.md) - Metadata about the schedule file (shared with v5+)
- [panels](panels-public-v7.md) - Flattened panels array with implicit breaks included
- [rooms](rooms-v4.md) - Physical and virtual event spaces (same as v4)
- [panelTypes](panelTypes-v7.md) - Event category definitions with implicit break support
- [timeTypes](timeTypes-v4.md) - Time category definitions (same as v4)
- [timeline](timeline-v4.md) - Key time markers for layout and navigation (same as v4)
- [presenters](presenters-v4.md) - People and groups that present events (same as v4)

## Structure Details

### [`meta`](meta-v5.md)

`meta` is a JSON object containing metadata about the schedule file itself.

**Access:** Public

**Status:** Supported in v7

**Key Fields:**

| Field       | Type    | Public | Description                                                 |
| ----------- | ------- | ------ | ----------------------------------------------------------- |
| `title`     | string  | yes    | Display title for the schedule                              |
| `generated` | string  | yes    | ISO 8601 UTC timestamp when the file was generated          |
| `version`   | integer | yes    | Schema version number (always `7` for this format)          |
| `variant`   | string  | yes    | Format variant: `"full"` for private, `"public"` for public |
| `generator` | string  | yes    | Identifier of the tool that produced the file               |
| `startTime` | string  | yes    | ISO 8601 UTC timestamp of the schedule start date           |

### [`panels`](panels-public-v7.md)

`panels` is a JSON array where each entry represents one session - the smallest schedulable unit, flattened from the private hierarchical format with automatically generated implicit breaks included.

**Access:** Public

**Status:** Supported in v7 (public format only)

**Key Changes from v6:**
- `panelType` now uses 2-letter prefix format
- Implicit break sessions are automatically included
- BREAK room assignment for generated breaks

### [`panelTypes`](panelTypes-v7.md)

`panelTypes` is a JSON array where each entry defines a category of events with enhanced support for implicit break types.

**Access:** Public

**Status:** Supported in v7

**Key Changes from v6:**
- `uid` and `prefix` are now identical (2-letter codes)
- Added `isImplicit` and `isOvernight` boolean fields
- Special handling for IB (Implicit Break) and NB (Overnight Break) types

## Migration Notes

### From v6 Public

**Automatic Conversion:**
- `panelType` UIDs: `"panel-type-br"` → `"BR"`
- Widget handles prefix-to-CSS conversion automatically
- No client-side implicit break generation needed

**Backend Changes:**
- Implicit breaks now generated during export
- BREAK room auto-created if needed
- Panel types marked as hidden but included for breaks

### Widget Compatibility

The v7 widget supports both formats:
- **v6**: Full UID format with client-side break generation
- **v7**: Prefix format with backend-generated breaks

## Complete Example

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-03-21T05:21:32Z",
    "version": 7,
    "variant": "public",
    "generator": "cosam-editor 0.1.0",
    "startTime": "2026-06-25T21:00:00Z",
    "endTime": "2026-06-27T19:00:00Z"
  },
  "panels": [
    {
      "id": "GP002S1",
      "baseId": "GP002",
      "partNum": null,
      "sessionNum": null,
      "name": "Guest Panel",
      "panelType": "GP",
      "roomIds": [1],
      "startTime": "2026-06-26T10:00:00",
      "endTime": "2026-06-26T11:00:00",
      "duration": 60,
      "isFree": true,
      "credits": ["John Doe"]
    },
    {
      "id": "IB001S1", 
      "baseId": "IB001",
      "partNum": null,
      "sessionNum": null,
      "name": "Break",
      "panelType": "IB",
      "roomIds": [999],
      "startTime": "2026-06-26T11:00:00",
      "endTime": "2026-06-26T15:00:00",
      "duration": 240,
      "description": "Break period",
      "isFree": true,
      "credits": []
    }
  ],
  "panelTypes": {
    "GP": {
      "prefix": "GP",
      "kind": "Guest Panel",
      "color": "#E2F9D7",
      "isBreak": false,
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
    }
  },
  "rooms": [
    {
      "uid": 1,
      "short_name": "Main",
      "long_name": "Main Events",
      "hotel_room": "Salon A",
      "sort_key": 1
    },
    {
      "uid": 999,
      "short_name": "BREAK",
      "long_name": "BREAK",
      "hotel_room": "",
      "sort_key": 999
    }
  ]
}
```
