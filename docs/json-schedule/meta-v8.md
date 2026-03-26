# `meta`

`meta` is a JSON object containing metadata about the schedule file itself.

## Access

Public

## Status

Supported in v8

## Fields

| Field             | Type    | Public | Description                                                  |
| ----------------- | ------- | ------ | ------------------------------------------------------------ |
| `title`           | string  | yes    | Display title for the schedule                               |
| `generated`       | string  | yes    | ISO 8601 UTC timestamp when the file was generated           |
| `version`         | integer | yes    | Schema version number (always `8` for this format)           |
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

The `variant` field distinguishes between the two format variants:

- **`"full"`**: Private/internal format with complete hierarchical data, metadata fields, Excel metadata, and optional changeLog
- **`"display"`**: Public format with flattened panels, baked-in breaks, and no private fields

### Version Field

The `version` field indicates the JSON schema version:

- **`8`**: Full format with changeLog support (latest)
- **`7`**: Display format (unchanged from v7)

### Presenter ID Counter

The `nextPresenterId` field tracks the next available integer ID for new presenters. This counter is monotonically increasing and never reused, ensuring stable presenter IDs across edits. Only present in the full format.

### Timestamp Formats

All timestamps use ISO 8601 UTC format: `YYYY-MM-DDTHH:MM:SSZ`

- `generated`: When the JSON file was created
- `startTime`/`endTime`: Overall schedule start and end times
- `modified`: When the Excel source file was last modified

## Examples

### Full Format (v8)

```json
{
  "title": "Cosplay America 2026 Schedule",
  "generated": "2026-06-01T12:00:00Z",
  "version": 8,
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

### Display Format (v7)

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

- The `variant` field is required to distinguish between full and display formats
- Excel metadata fields (`creator`, `lastModifiedBy`) are only included in the full format
- `nextPresenterId` is only present in the full format and must not be decremented
- Full format files use version 8, display format files remain at version 7
- The variant name changed from `"public"` (v6) to `"display"` (v7)

## Version History

### v8 Changes

- Version number updated from 7 to 8 for full format
- Display format remains at version 7
- No other field changes from v7 meta structure

### v7 Changes

- Added `variant` field to distinguish full/display formats
- Added `nextPresenterId` for stable presenter IDs
- Excel metadata fields moved from private to full format only

### v6 Changes

- Added Excel metadata fields (`creator`, `lastModifiedBy`, `modified`)
- Maintained separate private/public variants
