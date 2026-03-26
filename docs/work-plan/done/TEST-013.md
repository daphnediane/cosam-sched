# cosam-modify Test Coverage

## Summary

Add comprehensive tests for cosam-modify's new undo/redo/show-history commands

## Status

Completed

## Priority

High

## Description

cosam-modify now supports persistent undo/redo via EditHistory, but lacks automated tests for these features.

### Test Requirements

#### 1. Undo/Redo Commands

- Test undo with no history (should show "Nothing to undo")
- Test undo after single modification
- Test undo after multiple modifications
- Test redo after undo
- Test redo with no redo history
- Test that undo/redo persists across invocations

#### 2. Show History Command

- Test show-history with empty history (JSON and human formats)
- Test show-history after single command
- Test show-history after multiple commands
- Test show-history JSON output structure
- Verify undoCount and redoCount fields

#### 3. Integration Tests

- Full workflow: modify → save → load → undo → save → load → verify
- Test with both JSON and XLSX files
- Verify changeLog is preserved/updated correctly

#### 4. Edge Cases

- Undo after clearing metadata
- Undo after complex reschedule operations
- Undo after presenter changes
- Undo history depth limit (50 commands)

### Implementation Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    fn create_test_schedule() -> ScheduleFile {
        // Helper to create a minimal test schedule
    }
    
    #[test]
    fn test_undo_no_history() {
        let mut sf = create_test_schedule();
        // Test undo with empty history
    }
    
    #[test]
    fn test_undo_single_command() {
        let mut sf = create_test_schedule();
        // Apply a command, then undo
        // Verify original state restored
    }
    
    // ... more tests
}
```

### Test Files Structure

- `apps/cosam-modify/src/main.rs` - Add test module
- Use `tempfile` crate for temporary test files
- Test both JSON and XLSX round-trips

## Acceptance Criteria

- All undo/redo scenarios tested
- Show history output verified
- Cross-invocation persistence tested
- Edge cases covered
- Tests pass with `cargo test -p cosam-modify`
