# `changeLog`

`changeLog` is a JSON object containing the edit history for a schedule file, enabling persistent undo/redo functionality across application sessions.

## Access

Private

## Status

Introduced in v8

## Fields

| Field       | Type                         | Public | Description                                                 |
| ----------- | ---------------------------- | ------ | ----------------------------------------------------------- |
| `undoStack` | array of EditCommand objects | no     | Stack of edits that can be undone (newest first)            |
| `redoStack` | array of EditCommand objects | no     | Stack of edits that can be redone (newest first)            |
| `maxDepth`  | integer                      | no     | Maximum number of edits to keep in each stack (default: 50) |

## Description

The `changeLog` object tracks all edits made to a schedule file, enabling undo and redo operations that persist across file saves and application restarts. It is only present in the full format variant and is omitted when both stacks are empty.

### Stack Behavior

- **`undoStack`**: Contains edits that can be undone. The most recent edit is at index 0.
- **`redoStack`**: Contains undone edits that can be redone. The most recently undone edit is at index 0.
- **Stack management**: When a new edit is applied, the redo stack is cleared.
- **Depth limiting**: When a stack exceeds `maxDepth`, the oldest edits are discarded.

### EditCommand Objects

Each entry in `undoStack` and `redoStack` is an `EditCommand` object with the following common fields:

| Field       | Type   | Description                                                    |
| ----------- | ------ | -------------------------------------------------------------- |
| `type`      | string | Type of edit operation (e.g., "EditPanelName", "AddPresenter") |
| `timestamp` | string | ISO 8601 UTC timestamp when the edit was made                  |
| ...         | ...    | Additional fields specific to the edit type                    |

### Common EditCommand Types

The exact set of edit types may evolve as the edit system develops. Current planned types include:

#### Panel Edits

- **`EditPanelName`**: `panelId`, `oldName`, `newName`
- **`EditPanelDescription`**: `panelId`, `oldDescription`, `newDescription`
- **`EditPanelType`**: `panelId`, `oldType`, `newType`
- **`AddPanel`**: `panelId`, `panelData`
- **`RemovePanel`**: `panelId`, `panelData`

#### Session Edits

- **`EditSessionTime`**: `panelId`, `sessionId`, `oldStartTime`, `oldEndTime`, `newStartTime`, `newEndTime`
- **`EditSessionRoom`**: `panelId`, `sessionId`, `oldRoomIds`, `newRoomIds`

#### Presenter Edits

- **`AddPresenter`**: `presenterId`, `presenterData`
- **`RemovePresenter`**: `presenterId`, `presenterData`
- **`EditPresenterName`**: `presenterId`, `oldName`, `newName`

#### Bulk Operations

- **`BulkEdit`**: `edits` (array of EditCommand objects)

## Examples

### Basic ChangeLog with Single Edit

```json
{
  "changeLog": {
    "undoStack": [
      {
        "type": "EditPanelName",
        "panelId": "GP002",
        "oldName": "Old Panel Name",
        "newName": "Cosplay Contest Misconceptions",
        "timestamp": "2026-06-01T12:15:00Z"
      }
    ],
    "redoStack": [],
    "maxDepth": 50
  }
}
```

### ChangeLog with Multiple Edits

```json
{
  "changeLog": {
    "undoStack": [
      {
        "type": "EditPanelDescription",
        "panelId": "GP002",
        "oldDescription": "Previous description",
        "newDescription": "A deep-dive into competition issues.",
        "timestamp": "2026-06-01T12:30:00Z"
      },
      {
        "type": "EditPanelName",
        "panelId": "GP002",
        "oldName": "Old Panel Name",
        "newName": "Cosplay Contest Misconceptions",
        "timestamp": "2026-06-01T12:15:00Z"
      }
    ],
    "redoStack": [
      {
        "type": "EditSessionTime",
        "panelId": "GP003",
        "sessionId": "GP003",
        "oldStartTime": "2026-06-27T10:00:00",
        "oldEndTime": "2026-06-27T11:00:00",
        "newStartTime": "2026-06-27T14:00:00",
        "newEndTime": "2026-06-27T15:00:00",
        "timestamp": "2026-06-01T11:45:00Z"
      }
    ],
    "maxDepth": 50
  }
}
```

### Empty ChangeLog (Omitted)

When both `undoStack` and `redoStack` are empty, the entire `changeLog` field is omitted from the JSON file:

```json
{
  "meta": { ... },
  "panelTypes": { ... },
  "panels": { ... },
  // ... other fields, but no changeLog
}
```

## Usage Patterns

### Undo Operation

1. Pop first command from `undoStack`
2. Apply inverse operation to schedule data
3. Push command onto `redoStack`
4. Update file timestamp

### Redo Operation

1. Pop first command from `redoStack`
2. Apply operation to schedule data
3. Push command onto `undoStack`
4. Update file timestamp

### New Edit Operation

1. Create new EditCommand with current timestamp
2. Push onto `undoStack`
3. Clear `redoStack`
4. Enforce `maxDepth` limit (remove oldest if needed)
5. Apply edit to schedule data

## Implementation Notes

### Serialization Format

- EditCommand objects are serialized using standard JSON
- Timestamps use ISO 8601 UTC format: `YYYY-MM-DDTHH:MM:SSZ`
- Complex objects (like `panelData`) are included inline for complete state capture

### Performance Considerations

- Large edit histories can increase file size significantly
- `maxDepth` should be tuned based on typical usage patterns
- Consider compression for very large change logs

### Security Considerations

- Edit commands contain full state data for proper rollback
- Sensitive data in edit history should be treated as part of the file
- Access control should apply to the entire file including changeLog

## Version Compatibility

- **v8**: Full support for changeLog in full format files
- **v7 and earlier**: No changeLog field (treated as empty)
- **Display format**: Never includes changeLog regardless of version

## Future Evolution

The exact set of EditCommand types and their field structures may evolve as the edit system develops. Documentation will be updated as new edit types are implemented.
