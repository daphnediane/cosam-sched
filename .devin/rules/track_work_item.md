---
trigger: model_decision
description: When implementing new features or discovering out-of-scope issues
globs: work-item/**/*.md,docs/WORK_ITEMS.md
---

# Work Item Tracking

## Structure

Work items live at the project root under `work-item/`, auto-organized into subdirectories:

| Directory            | Contents                         |
| -------------------- | -------------------------------- |
| `open/1-HIGH/`       | High-priority open items         |
| `open/2-MEDIUM/`     | Medium-priority open items       |
| `open/3-LOW/`        | Low-priority open items          |
| `open/4-NEW/`        | Placeholder stubs (auto-created) |
| `idea/`              | IDEA prefix items                |
| `meta/`              | META prefix items                |
| `closed/done/`       | Completed items                  |
| `closed/rejected/`   | Rejected items                   |
| `closed/superseded/` | Superseded items                 |
| `template/`          | Per-prefix and default templates |

### Prefixes

META, FEATURE, BUGFIX, UI, EDITOR, CLI, DEPLOY, CLEANUP, PERFORMANCE, DOCS, REFACTOR, TEST, IDEA

### Statuses

Placeholder → Open → In Progress → Completed (→ `closed/done/`)

## Workflow

1. **Create:** `perl scripts/work-item-update.pl --create <PREFIX>` — creates numbered stub in `open/4-NEW/`
2. **Fill in:** Edit file, change status from `Placeholder` to `Open`
3. **Progress:** Update status as work proceeds
4. **Finalize:** Run `scripts/work-item-update.pl` to reorganize and regenerate `docs/WORK_ITEMS.md`

## Documentation

Update relevant docs when completing work:

- `docs/architecture.md` — design changes
- `docs/json-schedule/*.md` — schema changes
- Inline rustdocs for public APIs

## Completion Checklist

- [ ] Work item marked `Completed`
- [ ] `scripts/work-item-update.pl` run to reorganize
- [ ] Relevant documentation updated
- [ ] Commit message references work item

See `.devin/workflows/execution-rhythm.md` for the full development workflow.
