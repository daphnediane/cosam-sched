# cosam-editor EditHistory Migration

## Summary

Migrate cosam-editor from snapshot-based undo/redo to EditHistory-based system

## Status

Open

## Priority

Low

## Description

cosam-editor currently uses a snapshot-based undo/redo system (storing complete panel states) while other apps use the command-based EditHistory. This migration would unify the undo/redo approach across all applications.

### Current Implementation

- Stores `Vec<IndexMap<String, Panel>>` snapshots
- `MAX_UNDO_STEPS = 50`
- Snapshot taken before each significant change
- Memory intensive but simple

### Target Implementation

- Use `ScheduleFile` with `EditHistory`
- Convert UI actions to `EditCommand`s
- Leverage existing edit module infrastructure

### Migration Strategy

1. **Phase 1: Dual System**
   - Keep existing snapshots as fallback
   - Start recording EditCommands alongside snapshots
   - Verify parity between systems

2. **Phase 2: Gradual Migration**
   - Convert simple operations first (field edits)
   - Progress to complex operations (panel creation/deletion)
   - Maintain test coverage

3. **Phase 3: Cleanup**
   - Remove snapshot system
   - Remove `MAX_UNDO_STEPS` constant
   - Update UI to show undo/redo counts from EditHistory

### Challenges

- **Performance**: Command creation overhead vs snapshot copying
- **Complexity**: UI actions need precise command translation
- **Risk**: Core editor functionality must remain stable

### Alternative: Keep Current System

The snapshot system has advantages:

- Simpler for GUI with frequent small changes
- No need to translate every UI action
- Already implemented and tested
- Files saved have empty `changeLog` (cleaner)

## Recommendation

Consider keeping the snapshot system for cosam-editor while other apps use EditHistory. The two approaches serve different use cases:

- **CLI tools** (cosam-modify): Benefit from persistent, descriptive history
- **GUI editor** (cosam-editor): Benefits from simple, fast snapshots

## Acceptance Criteria

- Decision made on whether to migrate or keep current system
- If migrating: complete phased implementation with tests
- If keeping: document rationale and maintain current implementation
