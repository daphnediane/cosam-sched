# cosam-modify command

## Summary

Add a new command-line tool for in-place modifications of XLSX schedule files

## Status

Open

## Priority

Medium

## Description

Create a new `cosam-modify` command that allows programmatic in-place modifications of XLSX schedule files, similar to what cosam-editor does but for command-line automation and scripting.

### Use Cases

- **Batch operations**: Apply the same change to multiple schedule files
- **Automation scripts**: Integrate schedule modifications into CI/CD pipelines
- **Data migration**: Convert or transform schedule data programmatically
- **Bulk updates**: Update room assignments, panel types, or presenter information across many files
- **Validation**: Check and fix schedule consistency issues automatically

### Key Features

**In-place Updates**

- Preserve existing formatting, formulas, and extra columns
- Only modify rows that have changed
- Maintain Excel-specific features and custom sheets

**Modification Operations**

- Add/remove/update panels, rooms, presenters
- Change panel types and assignments
- Update scheduling information (times, durations)
- Apply bulk transformations based on patterns

**Command Interface**

```bash
# Basic usage
cosam-modify input.xlsx --output output.xlsx

# In-place modification
cosam-modify schedule.xlsx --in-place

# Batch operations
cosam-modify *.xlsx --batch --operation update-room --old "Room A" --new "Room B"

# Scripted modifications
cosam-modify schedule.xlsx --script modifications.json
```

### Implementation Details

**Architecture**

- Reuse existing `xlsx_update` module from cosam-editor
- Add new `cosam-modify` binary application
- Support multiple operation modes and batch processing

**Core Components**

1. **Command parser** - Handle CLI arguments and operation modes
2. **Operation engine** - Execute different types of modifications
3. **Batch processor** - Handle multiple files efficiently
4. **Script runner** - Execute JSON/YAML modification scripts
5. **Validation system** - Check data integrity before/after changes

**File Formats**

- **JSON scripts**: Define complex modification sequences
- **CSV imports**: Bulk data updates from external sources
- **Configuration files**: Default settings and transformation rules

### Technical Requirements

**Dependencies**

- Reuse `schedule-core` library (xlsx_update module)
- Add CLI argument parsing (clap)
- Add JSON/YAML support for scripts
- Add logging for batch operations

**Integration Points**

- Use existing `xlsx_update::update_xlsx()` for in-place updates
- Use existing `xlsx_import` for reading source files
- Use existing data structures (Schedule, Panel, etc.)

**Performance Considerations**

- Efficient batch processing for large file sets
- Progress reporting for long-running operations
- Memory-efficient processing for large schedules
- Parallel processing where safe

### Development Phases

**Phase 1: Core Infrastructure**

- [ ] Create `cosam-modify` binary application
- [ ] Implement basic CLI argument parsing
- [ ] Add simple in-place update functionality
- [ ] Basic error handling and logging

**Phase 2: Operations Engine**

- [ ] Implement add/remove/update operations
- [ ] Add bulk transformation capabilities
- [ ] Add validation and safety checks
- [ ] Support for common modification patterns

**Phase 3: Advanced Features**

- [ ] Script execution engine (JSON/YAML)
- [ ] Batch processing with progress reporting
- [ ] CSV import/export for bulk data
- [ ] Configuration file support

**Phase 4: Integration & Testing**

- [ ] Comprehensive test suite
- [ ] Documentation and examples
- [ ] Performance optimization
- [ ] CI/CD integration examples

### Acceptance Criteria

**Core Functionality**

- [ ] Can modify XLSX files in place preserving formatting
- [ ] Supports basic add/remove/update operations
- [ ] Handles batch operations on multiple files
- [ ] Provides clear error messages and logging

**Advanced Features**

- [ ] Script execution works for complex modifications
- [ ] CSV import/export for bulk data operations
- [ ] Configuration system for default behaviors
- [ ] Performance suitable for large file sets

**Quality Standards**

- [ ] All existing cosam-editor functionality preserved
- [ ] No data corruption or loss during modifications
- [ ] Comprehensive test coverage
- [ ] Clear documentation and examples

### Future Enhancements

**Integration Opportunities**

- Web API for remote modifications
- Plugin system for custom operations
- Integration with version control systems
- Real-time synchronization with databases

**Advanced Automation**

- Conditional modification rules
- Cross-file reference updates
- Automated conflict resolution
- Schedule optimization algorithms

## Notes

This feature builds on the existing xlsx_update infrastructure used by cosam-editor, making it a natural extension of the current codebase. The focus is on programmatic access to the same in-place update capabilities that the GUI editor provides.
