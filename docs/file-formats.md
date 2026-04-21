# File Formats

Overview of all file formats used in the project: internal schedule format, XLSX spreadsheet format, JSON widget format, and multi-year archive format.

## Internal Schedule Format (`.cosam`)

The native schedule file format is a versioned binary envelope wrapping an
[automerge](https://automerge.org/) document. It is produced by
`Schedule::save_to_file` and consumed by `Schedule::load_from_file`.

### Binary Layout (Format Version 1)

| Offset | Width | Type       | Description                              |
|--------|-------|------------|------------------------------------------|
| 0      | 6     | bytes      | Magic: `COSAM\x00` (ASCII + NUL)         |
| 6      | 2     | `u16` LE   | Format version (currently `1`)           |
| 8      | 4     | `u32` LE   | Metadata JSON byte length (`N`)          |
| 12     | N     | UTF-8 JSON | [`ScheduleMetadata`](#schedulemetadata)  |
| 12+N   | …     | bytes      | Automerge binary document                |

All multi-byte integers are little-endian.

### `ScheduleMetadata`

The metadata section is a JSON object with the following fields:

| Field         | JSON type | Description                                           |
|---------------|-----------|-------------------------------------------------------|
| `schedule_id` | string    | UUID v7 uniquely identifying this schedule document   |
| `created_at`  | string    | ISO 8601 UTC timestamp of original creation           |
| `generator`   | string    | Tool that created the file (e.g. `"cosam-convert 0.1"`) |
| `version`     | number    | Monotonically increasing edit counter (`u32`)         |

### Automerge Payload

The automerge document stores all entity field data and CRDT history.
The entity layout within the document is:

```text
ROOT
└── entities (Map)
    └── {type_name} (Map)           — e.g. "panel", "presenter"
        └── {uuid_string} (Map)     — entity UUID as hyphenated string
            ├── {field_name}        — Scalar (LWW), Text (RGA), or List
            └── __deleted           — bool; true = soft-deleted
```

Edge relationships are stored as owner-list fields (CRDT `List` objects)
on the canonical owner entity. See `edge_crdt.rs` for the ownership table.

### Versioning

- Format version `1` is the initial release.
- The `load_from_file` function returns `LoadError::Format` for any
  unrecognised version, preserving forward-error safety.
- Future versions will increment the version field; old readers will
  report a clear error rather than silently misreading data.

### Raw Automerge Sync Format

The `Schedule::save` / `Schedule::load` pair (no file envelope) is used
internally for CRDT sync operations. It produces and consumes raw automerge
bytes. This format does **not** preserve `ScheduleMetadata` and is not
intended for on-disk persistence.

---

## TODO

- XLSX spreadsheet format details (see spreadsheet-format.md)
- JSON widget format for calendar display
- Multi-year archive format
- Conversion workflows between formats
