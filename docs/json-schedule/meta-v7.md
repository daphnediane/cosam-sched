# `meta`

`meta` is a JSON object containing metadata about the schedule file itself.

## Access

Public

## Status

Supported in v7

## Fields

| Field             | Type    | Public | Description                                                  |
| ----------------- | ------- | ------ | ------------------------------------------------------------ |
| `title`           | string  | yes    | Display title for the schedule                               |
| `generated`       | string  | yes    | ISO 8601 UTC timestamp when the file was generated           |
| `version`         | integer | yes    | Schema version number (always `7` for this format)           |
| `variant`         | string  | yes    | Format variant: `"full"` for private, `"display"` for public |
| `generator`       | string  | yes    | Identifier of the tool that produced the file                |
| `startTime`       | string  | yes    | ISO 8601 UTC timestamp of the schedule start date            |
| `endTime`         | string  | yes    | ISO 8601 UTC timestamp of the schedule end date              |
| `nextPresenterId` | integer | no     | Next available presenter ID counter (full format only)       |
| `creator`         | string  | no     | Excel file creator/author (full format only)                 |
| `lastModifiedBy`  | string  | no     | Excel file last modified by (full format only)               |
| `modified`        | string  | yes    | Excel file last modified timestamp                           |

## Description

The `meta` object provides essential metadata about the schedule file including its title, version, variant, generation information, and Excel file metadata.

### Variant Field

The `variant` field distinguishes between the two v7 formats:

- **`"full"`**: Private/internal format with complete hierarchical data, metadata fields, and Excel metadata
- **`"display"`**: Public format with flattened panels, baked-in breaks, and no private fields

### Presenter ID Counter

The `nextPresenterId` field tracks the next available integer ID for new presenters. This counter is monotonically increasing and never reused, ensuring stable presenter IDs across edits. Only present in the full format.

### Timestamp Formats

All timestamps use ISO 8601 UTC format: `YYYY-MM-DDTHH:MM:SSZ`

- `generated`: When the JSON file was created
- `startTime`/`endTime`: Overall schedule start and end times
- `modified`: When the Excel source file was last modified

## Examples

### Full Format

```json
{
  "title": "Cosplay America 2026 Schedule",
  "generated": "2026-06-01T12:00:00Z",
  "version": 7,
  "variant": "full",
  "generator": "cosam-editor 0.3.0",
  "startTime": "2026-06-26T17:00:00Z",
  "endTime": "2026-06-28T18:00:00Z",
  "nextPresenterId": 42,
  "creator": "Schedule Editor",
  "lastModifiedBy": "Admin User",
  "modified": "2026-05-15T14:30:00Z"
}
```

### Display Format

```json
{
  "title": "Cosplay America 2026 Schedule",
  "generated": "2026-06-01T12:00:00Z",
  "version": 7,
  "variant": "display",
  "generator": "cosam-editor 0.3.0",
  "startTime": "2026-06-26T17:00:00Z",
  "endTime": "2026-06-28T18:00:00Z",
  "modified": "2026-05-15T14:30:00Z"
}
```

## Notes

- The `variant` field is required in v7 to distinguish between full and display formats
- Excel metadata fields (`creator`, `lastModifiedBy`) are only included in the full format
- `nextPresenterId` is only present in the full format and must not be decremented
- The variant name changed from `"public"` (v6) to `"display"` (v7)
