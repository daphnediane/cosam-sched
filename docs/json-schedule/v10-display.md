# Display Format v10

**Access Level**: Public  
**Status**: Current  
**Version**: 10

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

- [meta-v9.md](meta-v9.md) - Metadata structure (unchanged)
- [presenters-display-v9.md](presenters-display-v9.md) - DisplayPresenter with flat sortKey and panelIds (unchanged)
- [panels-display-v7.md](panels-display-v7.md) - Flattened panels with baked-in breaks
- [panelTypes-v7.md](panelTypes-v7.md) - Panel type definitions
- [rooms-v7.md](rooms-v7.md) - Room definitions
- [timeline-v7.md](timeline-v7.md) - Timeline markers

## Key Changes from v9

- **No structural changes**: The display format is unchanged from v9
- **Internal change**: DisplayPresenter fields are now populated from `RelationshipManager` instead of `Presenter` struct methods; output is identical
