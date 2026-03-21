# `meta`

`meta` is a JSON object containing metadata about the schedule file itself.

## Access

Public

## Status

Supported in v6

## Fields

| Field            | Type    | Public | Description                                                 |
| ---------------- | ------- | ------ | ----------------------------------------------------------- |
| `title`          | string  | yes    | Display title for the schedule                              |
| `generated`      | string  | yes    | ISO 8601 UTC timestamp when the file was generated          |
| `version`        | integer | yes    | Schema version number (always `6` for this format)          |
| `variant`        | string  | yes    | Format variant: `"full"` for private, `"public"` for public |
| `generator`      | string  | yes    | Identifier of the tool that produced the file               |
| `startTime`      | string  | yes    | ISO 8601 UTC timestamp of the schedule start date           |
| `endTime`        | string  | yes    | ISO 8601 UTC timestamp of the schedule end date             |
| `creator`        | string  | no     | Excel file creator/author (private format only)             |
| `lastModifiedBy` | string  | no     | Excel file last modified by (private format only)           |
| `modified`       | string  | yes    | Excel file last modified timestamp (private format only)    |

## Description

The `meta` object provides essential metadata about the schedule file including its title, version, variant, generation information, and Excel file metadata.

### Version Information

- **v6**: Enhanced version with Excel file metadata integration

### Variant Field

The `variant` field distinguishes between the two v6 formats:

- **`"full"`**: Private/internal format with complete hierarchical data and Excel metadata
- **`"public"`**: Public format with flattened panels, filtered fields, and no Excel metadata

### Timestamp Formats

All timestamps use ISO 8601 UTC format: `YYYY-MM-DDTHH:MM:SSZ`

- `generated`: When the JSON file was created
- `startTime`/`endTime`: Overall schedule start and end times
- `modified`: When the Excel source file was last modified

### Excel Metadata Fields

The following fields are extracted from the Excel file's document properties:

- **`creator`**: The author/creator stored in the Excel file
- **`lastModifiedBy`**: The last person who modified the Excel file
- **`modified`**: The last modification timestamp from the Excel file

These fields are only included in the private format to protect authorship information.

## Examples

### Private Format

```json
{
  "title": "Cosplay America 2026 Schedule",
  "generated": "2026-06-01T12:00:00Z",
  "version": 6,
  "variant": "full",
  "generator": "cosam-editor 0.2.0",
  "startTime": "2026-06-26T17:00:00Z",
  "endTime": "2026-06-28T18:00:00Z",
  "creator": "Schedule Editor",
  "lastModifiedBy": "Admin User",
  "modified": "2026-05-15T14:30:00Z"
}
```

### Public Format

```json
{
  "title": "Cosplay America 2026 Schedule",
  "generated": "2026-06-01T12:00:00Z",
  "version": 6,
  "variant": "public",
  "generator": "cosam-editor 0.2.0",
  "startTime": "2026-06-26T17:00:00Z",
  "endTime": "2026-06-28T18:00:00Z",
  "modified": "2026-05-15T14:30:00Z"
}
```

## Notes

- The `variant` field is required in v6 to distinguish between private and public formats
- Excel metadata fields (`creator`, `lastModifiedBy`) are only included in the private format
- The `modified` field is included in both formats to show when the source Excel file was last changed
- The `generator` field helps identify which tool created the file and can be useful for debugging
- `startTime` and `endTime` are used for schedule boundary calculations and validation
- Excel metadata is extracted from the source file's document properties using umya_spreadsheet
