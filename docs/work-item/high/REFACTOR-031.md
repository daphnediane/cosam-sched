# Separate TimelineEntry Entity from Panel

## Summary

Extract timeline entries (SPLIT, BREAK, room hours) into a dedicated TimelineEntry entity following the schedule-core pattern.

## Status

Not Started

## Priority

High

## Description

Currently timeline entries (SPLIT, BREAK, room hours) are stored as Panel entities with special flags. Per the schedule-core architecture, these should be in a separate TimelineEntry entity with its own storage. This aligns with how schedule-core handles timeline entries as a distinct `timeline: Vec<TimelineEntry>` field separate from panels.

## Implementation Details

### 1. Create TimelineEntry Entity

**File: `entity/timeline_entry.rs`**

Create a new TimelineEntry entity with fields matching schedule-core:

- `uid: String` - unique identifier (e.g., "SPLIT01", "BREAK01")
- `start_time: Option<chrono::NaiveDateTime>` - when this timeline marker occurs
- `description: String` - display description (e.g., "Thursday Morning")
- `panel_type_uid: Option<String>` - reference to PanelType (e.g., "SPLIT", "BREAK")
- `note: Option<String>` - additional notes
- Implement EntityFields derive macro with proper aliases for field resolution

### 2. Create TimelineEntryEntityType

**File: `entity/mod.rs`**

Add TimelineEntryEntityType to the entity type system alongside Panel, EventRoom, etc.

### 3. Add TimelineEntry Storage to Schedule

**File: `schedule/storage.rs`**

Add timeline entry storage to Schedule:

- Add `timeline_entries: EntityStorage<TimelineEntryEntityType>` field
- Add methods for timeline entry CRUD operations
- Ensure timeline entries can be queried and indexed

### 4. Update Import/Export Logic

**Files: XLSX import/export modules**

- When importing XLSX, detect panels with `is_timeline` or `is_room_hours` PanelType
- Convert these to TimelineEntry entities instead of Panel entities
- When exporting XLSX, render TimelineEntry entities in the appropriate location (after regular panels or in a separate section)
- Update JSON export to include timeline entries as a separate array (matching schedule-core format)

### 5. Update PanelType Flags

**File: `entity/panel_type.rs`**

- Keep `is_timeline` and `is_room_hours` flags on PanelType for type classification
- These flags indicate which PanelTypes should create TimelineEntry entities vs Panel entities

### 6. Update Queries and UI

- Update any queries that currently filter for timeline panels to query TimelineEntry instead
- Update UI components to handle TimelineEntry display separately from Panel display
- Ensure timeline entries appear in the correct order in schedule views

## Acceptance Criteria

- TimelineEntry entity exists with fields matching schedule-core TimelineEntry
- Timeline entries are stored separately from Panel entities
- XLSX import correctly converts is_timeline/is_room_hours panels to TimelineEntry
- XLSX export correctly renders TimelineEntry entities
- JSON export includes timeline entries as separate array
- All existing tests pass after migration
- Timeline entries display correctly in UI

## Dependencies

- REFACTOR-002 should complete first (for PanelType flag alignment)
