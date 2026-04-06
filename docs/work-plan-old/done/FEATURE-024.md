# JSON Format Documentation

## Summary

Create documentation for the new v8 JSON format with changeLog support

## Status

Completed

## Priority

Medium

## Description

The ScheduleFile refactor introduced JSON version 8 with an optional `changeLog` field. This needs proper documentation in the docs/json-format/ directory.

### Required Documentation Files

#### 1. `docs/json-format/v8-full.md`

- Document the complete JSON structure for full schedule files
- Show example with `changeLog` included
- Explain version differences (v7 vs v8)
- Reference existing sub-structure docs

#### 2. `docs/json-format/changeLog-v8.md`

- Document the `changeLog` object structure:

  ```json
  {
    "undoStack": [/* EditCommand objects */],
    "redoStack": [/* EditCommand objects */],
    "maxDepth": 50
  }
  ```

- Explain that it's omitted when empty
- Document each EditCommand type (reference to edit module docs)
- Note: Exact command formats may evolve as edit system develops

#### 3. `docs/json-format/meta-v8.md`

- Copy of meta-v7.md with version updated to 8
- Explain that full files use version 8
- Note that display/export files remain at version 7

#### 4. Update `docs/json-format/README.md`

- Add v8 to version history table
- Clarify which formats use which version:
  - Full files (with history): v8
  - Display exports: v7
  - Empty files: v4

### Implementation Details

- Run `perl scripts/combine-json-docs.pl` after creating files
- Ensure all markdown files pass linting
- Add examples showing changeLog in action

### Migration Notes

- No v7→v8 migration code needed (alpha software)
- All files regenerated from canonical spreadsheets
- Backward compatibility: files without changeLog load fine

## Acceptance Criteria

- All four documentation files created
- Combined documentation generated
- Examples provided
- Version history updated
