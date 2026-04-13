---
trigger: model_decision
description: When implementing new features, or when limitations or bugs are discovered but not in scope
globs: docs/work-item/**/*.md,docs/WORK_ITEMS.md
---
# Work Item Tracking Rules

## Structure

Work items are in `docs/work-item/` as `<PREFIX>-<###>.md` files, automatically organized into subdirectories:

- **done/** - Completed items
- **rejected/** - Superseded or Rejected items
- **meta/** - Meta/project-level items (META prefix, any priority)
- **idea/** - Open design questions and deferred ideas (IDEA prefix)
- **high/** - High priority open items
- **medium/** - Medium priority open items
- **low/** - Low priority open items

### Prefixes

- **META** - Project-level meta items and phase trackers (always in meta/)
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
- **IDEA** - Open design questions, unexplored alternatives, deferred ideas (always in idea/)

### File Template

```markdown
# Brief title

## Summary
One-line summary

## Status
Open | In Progress | Completed | Blocked | Not Started | Superseded | Rejected

## Priority
High | Medium | Low

## Blocked By (optional)
- PREFIX-###: short description

## Description
[Detailed description]

## Work Items (optional, META prefix only)
- PREFIX-###: short description

## Additional Sections (optional)
- Steps to Fix (for bugs)
- Implementation Details (for features)
- Acceptance Criteria
- Notes
```

## Workflow

### Adding Items

1. Check `docs/WORK_ITEMS.md` "Next Available IDs" section for the next free number
   (the combine script regenerates this list; it lists numbers not yet used by any prefix)
2. Create file in `docs/work-item/` as `<PREFIX>-<###>.md` using that number
3. Follow template structure
4. Set status to "Open" and appropriate priority
5. Run combine script to organize and regenerate

### Updating Items

- Edit file directly
- Update status as work progresses
- Run combine script to reorganize files

### Combine Script

**All platforms:** `perl scripts/combine-workitems.pl`

The script automatically:

- Move files to subdirectories based on status/priority
- Generate `docs/WORK_ITEMS.md` with reference-style links (excludes IDEA items)
- Generate `docs/FUTURE_IDEAS.md` from IDEA-prefix items
- Add headerless link glossary at end
- Preserve leading zeros and use LF line endings

## Documentation Updates

When completing work items, update relevant documentation, see docs/doc-index.md for the complete list.

Always cross-reference between documents when changes affect multiple areas.

## Commit Messages

See `prepare-comment.md` for the full commit message format, template, and process.

When saving progress mid-refactor at logic breakpoints (without requiring
builds or tests to pass), write to `next_amend.tmp` instead of
`next_commit.tmp` — this signals a `git commit --amend` checkpoint rather
than a normal commit.

## Formatting

All files must follow markdown lint rules. For Cascade agents: suggest user run formatting on `docs/work-item/` directory if lint errors occur.
