---
trigger: model_decision
description: When implementing multi-phase or multi-entity work plans
---
# Execution Rhythm for Plan Implementation

For each phase/entity in a work plan:

1. Mark phase/entity as `in_progress` in plan file and work-plan file
2. Implement only that scope (no other phases/entities)
3. Add/update tests and run `cargo test`
4. Update documentation
5. Mark phase/entity as `completed` in plan file and work-plan file
6. Run `scripts/combine-workplans.pl` to update `docs/WORK_PLAN.md` and reorganize plan files
7. Follow `.windsurf/rules/prepare-comment.md`, create `next_commit.tmp`, if in doubt ask use for AI model.
8. Run `git commit -F ./next_commit.tmp`
9. State next step and wait for approval

**One phase/entity per commit. Always wait for user approval.**
