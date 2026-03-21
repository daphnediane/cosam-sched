# v6-Private

Private format documentation for JSON schedule format v6.

## Top-Level Structure

```json
{
  "meta": { ... },
  "panelTypes": { ... },
  "rooms": { ... },
  "panels": { ... },
  "conflicts": [ ... ]
}
```

## Structures

- [meta-v6.md](meta-v6.md) - Metadata structure with Excel file integration
- [panels-v5.md](panels-v5.md) - Hierarchical panels hash (private) (unchanged from v5)
- [PanelPart-v5.md](PanelPart-v5.md) - Panel part objects (private) (unchanged from v5)
- [PanelSession-v5.md](PanelSession-v5.md) - Panel session objects (private) (unchanged from v5)
- [panelTypes-v5.md](panelTypes-v5.md) - Panel type categories (unchanged from v5)
- [rooms-v5.md](rooms-v5.md) - Room definitions (unchanged from v5)
- [conflicts-v5.md](conflicts-v5.md) - Conflict detection structures (unchanged from v5)

## Key Changes from v5

### Excel Metadata Integration

The `meta` structure now includes Excel file metadata:

- **Added**: `creator` field (private only) - Excel file author
- **Added**: `lastModifiedBy` field (private only) - Last editor of Excel file
- **Added**: `modified` field (private only) - Excel file modification timestamp
- **Changed**: Version number increased to `6`

### Field Privacy

Excel metadata fields are excluded from public format to protect authorship information.

## Migration Notes

### v5-private → v6-private

1. Update `meta.version` from `5` to `6`
2. Add Excel metadata fields to `meta` (extracted from source Excel file):
   - `creator`: Excel file author
   - `lastModifiedBy`: Excel file last modified by
   - `modified`: Excel file modification timestamp
3. All other structures remain unchanged

### v6-private → v6-public

1. Set `meta.variant` to `"public"`
2. Flatten hierarchical `panels` structure to array
3. Filter out private fields including Excel metadata
4. Convert internal presenter references to public credits

## Complete Example

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-20T21:00:00Z",
    "version": 6,
    "variant": "full",
    "generator": "cosam-editor 0.2.0",
    "startTime": "2026-06-26T17:00:00Z",
    "endTime": "2026-06-28T18:00:00Z",
    "creator": "Schedule Editor",
    "lastModifiedBy": "Admin User",
    "modified": "2026-05-15T14:30:00Z"
  },
  "panelTypes": {
    "panel-type-events": {
      "uid": "panel-type-events",
      "name": "Events",
      "description": "Main convention events",
      "color": "#FF6B6B",
      "isHidden": false,
      "isRoomHours": false,
      "bwColor": "#CCCCCC"
    }
  },
  "rooms": {
    "room-main": {
      "uid": "room-main",
      "name": "Main Hall",
      "capacity": 500,
      "location": "Building A, Floor 1"
    }
  },
  "panels": {
    "panel-001": {
      "uid": "panel-001",
      "title": "Opening Ceremony",
      "description": "Kick off the convention",
      "panelType": "panel-type-events",
      "room": "room-main",
      "startTime": "2026-06-26T17:00:00Z",
      "duration": 60,
      "parts": [
        {
          "uid": "part-001-1",
          "title": "Opening Ceremony",
          "sessions": [
            {
              "uid": "session-001-1-1",
              "presenters": ["presenter-host"],
              "credits": ["Host Name"]
            }
          ]
        }
      ],
      "presenters": ["presenter-host"],
      "credits": ["Host Name"],
      "conflicts": []
    }
  },
  "conflicts": []
}
```
