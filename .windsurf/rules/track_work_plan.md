# Work Plan Tracking Rules

## Structure

Work items are in `docs/work-plan/` as `<PREFIX>-<###>.md` files, automatically organized into subdirectories:

- **done/** - Completed items
- **high/** - High priority or Blocked items  
- **medium/** - Medium priority or In Progress items
- **low/** - Low priority or Not Started items

### Prefixes

- **FEATURE** - New functionality
- **BUGFIX** - Fixes for defects
- **UI** - Interface improvements
- **EDITOR** - Desktop editor app
- **CLI** - Command-line interface (cosam-convert, cosam-modify)
- **DEPLOY** - Packaging, deployment, and distribution
- **CLEANUP** - Repository cleanup
- **PERFORMANCE** - Optimizations
- **DOCS** - Documentation
- **REFACTOR** - Code restructuring
- **TEST** - Test additions

### File Template

```markdown
# Brief title

## Summary
One-line summary

## Status
Open | In Progress | Completed | Blocked | Not Started

## Priority
High | Medium | Low

## Description
[Detailed description]

## Additional Sections (optional)
- Steps to Fix (for bugs)
- Implementation Details (for features)
- Acceptance Criteria
- Notes
```

## Workflow

### Adding Items

1. Create file in `docs/work-plan/` with next available number
2. Follow template structure
3. Set status to "Open" and appropriate priority
4. Run combine script to organize and regenerate

### Updating Items

- Edit file directly
- Update status as work progresses
- Run combine script to reorganize files

### Combine Scripts

**Unix:** `perl scripts/combine-workplans.pl`  
**Windows:** `.\scripts\combine-workplans.ps1`

Scripts automatically:

- Move files to subdirectories based on status/priority
- Generate `docs/WORK_PLAN.md` with reference-style links
- Add headerless link glossary at end
- Preserve leading zeros and use LF line endings

## Formatting

All files must follow markdown lint rules. For Cascade agents: suggest user run formatting on `docs/work-plan/` directory if lint errors occur.
