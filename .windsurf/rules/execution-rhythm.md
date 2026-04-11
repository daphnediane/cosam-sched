---
trigger: model_decision
description: When implementing multi-phase plan artifacts or work items
---
# Execution Rhythm for Plan Implementation

For each work item or phase/entity in a plan artifact:

1. Mark item/phase/entity as `In Progress`
2. Implement only that scope (no other phases/entities)
3. Add/update tests and run `cargo test`
4. Update documentation (`docs/system-analysis.md`, `docs/field-system.md`, or subsystem docs as appropriate; also inline rust docs)
5. Mark item/phase/entity as `Completed`
6. Per `.windsurf/rules/track_work_item.md`, run `scripts/combine-workitems.pl` to update `docs/WORK_ITEMS.md` and reorganize work-item files
7. Follow `.windsurf/rules/prepare-comment.md`, create `next_commit.tmp`, if in doubt ask use for AI model.
8. Run `git commit -F ./next_commit.tmp`
9. State next step and wait for approval

**One phase/entity/work item per commit. Always wait for user approval.**

Some plan artifacts have have work items that are associated with individual phases, in that case
use the work items not the plan artifact to track the status. Some work items may have several
child work items, those should be done in separate commits, and the main work items should be
marked as completed only when all child work items are done.
