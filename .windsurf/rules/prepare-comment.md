---
trigger: model_decision
description: When finishing work and waiting for the user to commit
---
# Preparing Commit Comments

## Commit Message Format

All commit messages must follow this format:

1. **Tagged short line** (type: brief description)
   - Types: feat, fix, docs, style, refactor, test, chore
   - Examples: "feat: Add work plan tracking system", "fix: Resolve presenter parsing issue"

2. **Brief description** from user point of view
   - One or two sentences explaining what was done

3. **Bullet points** detailing specific changes
   - Use hyphens (-) for each bullet point
   - Be specific about what was added, modified, or removed

4. **AI attribution line**
   - Must end with: "Written with assistance from Windsurf AI\n[model]"
   - Replace [model] with the actual model used (e.g., "Claude Opus 4.6 Thinking")

## Example

```text
feat: Add work plan tracking system

Implement a structured work plan system for tracking project tasks and progress.

- Create docs/work-plan/ directory with individual markdown files for each work item
- Add scripts/combine-workplans.pl to generate docs/WORK_PLAN.md from individual files
- Create Windsurf rules documenting work plan structure and management
- Document completed work from commit history (FEATURE-001, FEATURE-002)
- Update .gitignore to preserve .windsurf/rules directory

Written with assistance from Windsurf AI
Claude Opus 4.6 Thinking
```

## Process

1. Use either the `write_to_file` or `edit` tool to create or update the `next_commit.tmp` file.
2. If `next_commit.tmp` already exists, Cascade cannot overwrite it using `write_to_file` and must use the `edit` tool or you can remove it first
   - On Unix/macOS: `rm next_commit.tmp`
   - On Windows: `del next_commit.tmp`
3. Follow the format exactly as specified
4. Include all significant changes in the bullet points
5. Ensure the file ends with the AI attribution line
6. The file may be left behind from a previous commit, so always start fresh unless instructed otherwise
