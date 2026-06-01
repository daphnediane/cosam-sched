---
trigger: model_decision
description: Guidelines for implementing work items and plan phases
---

# Execution Rhythm

## Core Principles

- **Work in complete units**: Finish one item/phase before starting the next
- **Scope discipline**: Implement only the current item/phase; resist scope creep
- **Always green**: `cargo test` must pass at every commit
- **Document as you go**: Update inline docs and relevant documentation files

## Tracking Work

- Mark items `In Progress` when starting, `Completed` when done
- Prefer work item files over plan artifacts when both exist for a phase
- Child work items require separate commits; parent completes only when all children done
- Run `scripts/work-item-update.pl` after completing work to regenerate `docs/WORK_ITEMS.md`

## Phase Boundary Flexibility

When Phase N removes infrastructure that Phase N+1 depends on for testing, you may pull in **minimum necessary** scope from Phase N+1:

- Test rewrites needed to replace deleted infrastructure
- Call-site updates made invalid by Phase N changes

**Requirements when absorbing future work:**

- Note absorbed scope in commit message
- Mark absorbed sub-tasks as done in the future phase's work item
- Do NOT pull in design work or unrelated features

## Committing

Follow `.claude/workflows/commit-changes.md` for commit workflow. See `.claude/rules/comment-file.md` for format and `.claude/rules/attribution.md` for AI attribution.

Always propose the commit command for user approval rather than auto-running.
