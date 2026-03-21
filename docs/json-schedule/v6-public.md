# v6-Public

Public format documentation for JSON schedule format v6.

## Top-Level Structure

```json
{
  "meta": { ... },
  "panels": [ ... ]
}
```

## Structures

- [meta-v6.md](meta-v6.md) - Metadata structure (Excel metadata partially included)
- [panels-public-v5.md](panels-public-v5.md) - Flattened panels array (public) (unchanged from v5)

## Key Changes from v5

### Version Update

- **Changed**: Version number increased to `6`
- **Added**: `modified` field from Excel file metadata
- **Unchanged**: All other public format structures and fields remain the same as v5

### Excel Metadata Exclusion

Excel metadata fields from the private format are not included:

- **Excluded**: `creator` field (private only)
- **Excluded**: `lastModifiedBy` field (private only)
- **Included**: `modified` field (now public)

## Migration Notes

### v5-public → v6-public

1. Update `meta.version` from `5` to `6`
2. All other structures and fields remain unchanged

### v6-private → v6-public

1. Set `meta.variant` to `"public"`
2. Flatten hierarchical `panels` structure to array
3. Filter out private fields (`creator`, `lastModifiedBy`) but keep `modified`
4. Convert internal presenter references to public credits

## Notes

- This format is only used in the public (`"public"`) variant of v6
- Private format uses the hierarchical [panels hash](panels-v5.md) instead
- All effective values are pre-computed by the exporter for simple rendering
- The widget does not need to understand the base→part→session hierarchy
- `credits` contains formatted display strings, while `presenters` contains raw names for filtering
- Structure unchanged from v5

## Complete Example

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-20T21:00:00Z",
    "version": 6,
    "variant": "public",
    "generator": "cosam-editor 0.2.0",
    "startTime": "2026-06-26T17:00:00Z",
    "endTime": "2026-06-28T18:00:00Z",
    "modified": "2026-05-15T14:30:00Z"
  },
  "panels": [
    {
      "uid": "panel-001",
      "title": "Opening Ceremony",
      "description": "Kick off the convention",
      "panelType": "panel-type-events",
      "room": "room-main",
      "startTime": "2026-06-26T17:00:00Z",
      "duration": 60,
      "credits": ["Host Name"]
    }
  ]
}
```
