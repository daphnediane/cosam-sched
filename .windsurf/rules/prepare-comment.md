---
trigger: model_decision
description: When finishing work and waiting for the user to commit
---
# Preparing Commit Comments

## Commit Files

Commit messages go into a file in the repo root. **Always overwrite** the target file with the new message content (use `edit` tool if it already exists; do not use `write_to_file` on an existing file).

- **`next_commit.tmp`** — Normal commits. Work is at a logical completion point; tests/builds are expected to pass.
- **`next_amend.tmp`** — Checkpoint amend commits. Used during active refactoring to save progress at logic breakpoints via `git commit --amend`, without requiring everything to compile or pass yet.

## Template

```text
<tag/area>: <short concise subject> [<work item>]

<Description paragraph, typically one sentence>

- <Brief list of work done>
- <More work> [<work item>]
  - <Nested details>
- <More work>

Prompt:
<What I asked windsurf to do, main prompt>

Written with assistance from Windsurf AI
[model]
```

- `<tag/area>` follows conventional commit style: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `style`
- Work item references like `[FEATURE-007]` go inline after the subject line or after relevant bullets
- The `Prompt:` section captures the main user request; only include what actually landed — omit superseded asks; if summarized across turns or corrected, note it: `(summarized)` or `(corrected: originally asked X)`
- Replace `[model]` with the actual AI model used; ask the user if unknown

## Example

```text
feat: Add work item tracking system [FEATURE-001]

Implement a structured work item system for tracking tasks.

- Add docs/work-item/ with per-item markdown files [META-001]
- Add scripts/combine-workitems.pl to generate docs/WORK_ITEMS.md
- Update .gitignore to preserve .windsurf/rules/

Prompt:
Set up a work item tracking system with individual markdown files and a combine script.

Written with assistance from Windsurf AI
Claude Opus 4.6 Thinking
```

## Process

### Reviewing changes before writing

Before writing the commit message, review what actually changed:

- **Working changes:** Check the staged/unstaged diff (IDE working changes panel or `git diff --cached`) to confirm what is actually in scope.
- **For checkpoint amends:** Also check the HEAD commit (`git show --stat HEAD` or `git log -1`) to understand what is already recorded, so the updated message accurately reflects the cumulative state.

Use this to ensure bullets match reality — not what was planned or discussed.

### Writing the file

The `write_to_file` tool **cannot overwrite an existing file** — it will fail if the file already exists. Both `next_commit.tmp` and `next_amend.tmp` are frequently left behind from previous work, so always assume the file may exist.

To write the file:

- **Preferred:** Use the `edit` tool to replace the entire contents.
- **Alternative:** Delete the file first, then use `write_to_file`:
  - macOS/Linux: `rm next_commit.tmp` (or `next_amend.tmp`)
  - Windows: `del next_commit.tmp` (or `next_amend.tmp`)

### `next_commit.tmp` — normal commits

1. This file is almost always being replaced (previous session's message is stale).
2. The existing `[model]` line may hint at the model to use if it matches the current session — preserve it if it looks right, otherwise ask or leave `[model]` as a placeholder.
3. After writing `next_commit.tmp`, **empty `next_amend.tmp`** if it exists (truncate to zero bytes or delete it), since the checkpoint series is now superseded by the real commit.

### `next_amend.tmp` — checkpoint amends

1. If `next_amend.tmp` is **empty or missing**, use `next_commit.tmp` as a starting point and adapt it for the current in-progress state.
2. If `next_amend.tmp` already has content, **update it in place** to reflect the latest logic breakpoint — do not start from scratch unless the scope has changed significantly.
3. Keep the bullet list current and pruned: remove or consolidate bullets for work that was undone, superseded, or subsumed into a larger change. Bullets should reflect the net state of the refactor, not a chronological log. It is okay for the description to note work is in progress.

### General

- Follow the template format exactly.
- Include all significant changes in the bullet points.
- Ensure the file ends with the AI attribution line.
