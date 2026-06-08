---
description: Execute a work item or plan phase with proper tracking
---

# Execution Rhythm

Execute a single work item or plan phase. See `.devin/rules/execution-rhythm.md` for principles and guidelines.

## Prerequisites

- Plan artifact or work item file exists in `work-item/`
- Item/phase marked ready to start

## Steps

### 1. Start

- Read the plan/work item
- Mark as `In Progress`

### 2. Implement

- Implement only current scope (no scope creep)
- Add/update tests for changes

### 3. Verify

```bash
cargo test    # must pass
cargo fmt     # format code
cargo clippy  # fix warnings
```

### 4. Document

- Update inline documentation
- Update relevant docs per `docs/architecture.md`

### 5. Complete Tracking

- Mark item/phase as `Completed`
- Run `scripts/work-item-update.pl` to regenerate `docs/WORK_ITEMS.md`

### 6. Commit or Amend

**New work:** Follow `.devin/workflows/commit-changes.md` — creates new commit from `./next-commit.txt`

**Fixes to same work item:** Follow `.devin/workflows/amend-changes.md` — amends previous commit using `./next-amend.txt`

### 7. Next Steps

- State next step
- Wait for user direction

## Phase Boundary Handling

If current phase breaks infrastructure that future phases fix:

- Pull in **minimum necessary** scope from next phase
- Only to keep `cargo test` passing
- Document absorbed scope in commit message
- Mark absorbed sub-tasks done in future work item

See `.devin/rules/execution-rhythm.md` for detailed flexibility guidelines.
