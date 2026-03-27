# Display Format v9

**Access Level**: Public  
**Status**: Supported  
**Version**: 9

Public-facing schedule format with DisplayPresenter objects and filtered presenter list.

## Top-Level Structure

```json
{
  "meta": Meta,
  "panels": Array<DisplayPanel>,
  "rooms": Array<Room>,
  "panelTypes": Object<PanelType>,
  "timeline": Array<TimelineEntry>,
  "presenters": Array<DisplayPresenter>
}
```

## Structures

- [meta-v9.md](meta-v9.md) - Metadata structure
- [presenters-display-v9.md](presenters-display-v9.md) - DisplayPresenter with flat sortKey and panelIds
- [panels-display-v7.md](panels-display-v7.md) - Flattened panels with baked-in breaks
- [PanelType-v7.md](PanelType-v7.md) - Panel type definitions
- [rooms-v7.md](rooms-v7.md) - Room definitions
- [timeline-v7.md](timeline-v7.md) - Timeline markers

## Key Changes from v8

- **DisplayPresenter**: New public presenter structure replaces raw Presenter objects
- **sortKey**: Flat sequential ordering key (0-based) computed from PresenterSortRank
- **Filtered presenters**: Only includes presenters referenced by at least one visible panel
- **isGroup/members/groups**: Public-facing group relationship fields for widget consumption
- **Version bump**: v9 to reflect PresenterSortRank internal changes

## DisplayPresenter Details

The `DisplayPresenter` structure provides exactly what the widget needs:

- **sortKey**: Simple integer for ordering presenters
- **isGroup**: Boolean to separate individuals from groups
- **members**: Array of member names for groups (empty for individuals)
- **groups**: Array of group names for individuals (empty for groups)
- **alwaysGrouped/alwaysShown**: Group display behavior flags

## Complete Example

```json
{
  "meta": {
    "title": "Event Schedule 2026",
    "version": 9,
    "variant": "display",
    "generator": "cosam-editor 0.1.0",
    "generated": "2026-03-26T22:00:00Z",
    "modified": "2026-03-26T21:45:00Z",
    "startTime": "2026-05-29T17:00:00Z",
    "endTime": "2026-06-01T15:00:00Z"
  },
  "panels": [
    {
      "id": "panel-001",
      "baseId": "panel-001",
      "name": "Opening Ceremony",
      "panelType": "panel-type-ceremony",
      "roomIds": [1],
      "startTime": "2026-05-29T17:00:00Z",
      "endTime": "2026-05-29T18:00:00Z",
      "duration": 60,
      "credits": ["MC Host"],
      "presenters": ["MC Host"]
    }
  ],
  "rooms": [
    {
      "uid": 1,
      "name": "Main Hall",
      "building": "Convention Center",
      "isBreak": false
    }
  ],
  "panelTypes": {
    "panel-type-ceremony": {
      "prefix": "ceremony",
      "kind": "Ceremony",
      "colors": {
        "color": "#FF6B6B"
      },
      "isBreak": false,
      "isCafe": false,
      "isWorkshop": false,
      "isHidden": false,
      "isRoomHours": false,
      "isTimeline": false,
      "isPrivate": false
    }
  },
  "timeline": [],
  "presenters": [
    {
      "name": "MC Host",
      "rank": "guest",
      "sortKey": 0,
      "isGroup": false,
      "members": [],
      "groups": [],
      "alwaysGrouped": false,
      "alwaysShown": false
    }
  ]
}
```
