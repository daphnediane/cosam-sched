---
trigger: model_decision
description: When implementing new features, or when limitations or bugs are discovered but not in scope
globs: docs/work-item/**/*.md,docs/WORK_ITEMS.md
---
# Work Item Tracking Rules

## Structure

Work items are in `docs/work-item/` as `<PREFIX>-<###>.md` files, automatically organized into subdirectories:

- **new/** - Placeholder stubs not yet ready to work (auto-created by `--create`)
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

### Statuses

- **Placeholder** - Newly created stub in `new/`; fill in and promote to Open
- **Open** - Ready to be worked
- **Not Started** - Acknowledged but not yet scheduled
- **In Progress** - Actively being worked
- **Blocked** - Waiting on another item (list in Blocked By section)
- **Completed** - Done; moved to `done/`
- **Superseded** - Replaced by another item; moved to `rejected/`
- **Rejected** - Will not be done; moved to `rejected/`

### Templates

Per-prefix templates are in `docs/work-item/template/`:

- **`default-template.md`** - Used for any prefix without a specific template
- **`BUGFIX-template.md`** - Adds How Found, Reproduction, Steps to Fix, Testing sections
- **`META-template.md`** - Adds Work Items section; defaults to High priority
- **`IDEA-template.md`** - Minimal; starts as Placeholder/Low

## Workflow

1. Run `perl scripts/work-item-update.pl --create <PREFIX>` to create a properly
   numbered placeholder file and print its path; edit the file to fill in details
   and change status from `Placeholder` to `Open`
   - Multiple tags: `--create FEATURE --create BUGFIX` or `--create FEATURE,BUGFIX`
2. Edit the file directly to update status as work progresses
3. Run `perl scripts/work-item-update.pl` to reorganize files and regenerate
   `docs/WORK_ITEMS.md` / `docs/FUTURE_IDEAS.md`

`Placeholder` status is for newly created stubs not yet ready to be worked; they
live in `new/` until status is changed, then the tool moves them automatically
the next time it is run.

## Documentation Updates

When completing work items, update relevant docs:

- **Architecture/entity changes** — `docs/system-analysis.md` + inline rust docs
- **Field system changes** — `docs/field-system.md`
- **Edge system changes** — both `docs/field-system.md` and `docs/system-analysis.md`
- **Design decisions** — create `IDEA-###.md` to record reasoning and alternatives
- **New subsystems** — create `docs/<subsystem>.md` + reference in `system-analysis.md`

Cross-reference between documents when changes affect multiple areas.

## Commit Messages

See `prepare-comment.md` for the full commit message format, template, and process.

When saving progress mid-refactor at logic breakpoints (without requiring
builds or tests to pass), write to `next_amend.tmp` instead of
`next_commit.tmp` — this signals a `git commit --amend` checkpoint rather
than a normal commit.

## Formatting

All files must follow markdown lint rules. For Cascade agents: suggest user run formatting on `docs/work-item/` directory if lint errors occur.
