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
- Add scripts/fix-markdown-format.pl to ensure markdown lint compliance
- Create Windsurf rules documenting work plan structure and management
- Add scripts/update-workplan.sh wrapper script for easy regeneration
- Document completed work from commit history (FEATURE-001, FEATURE-002)
- Update .gitignore to preserve .windsurf/rules directory

Written with assistance from Windsurf AI
Claude Opus 4.6 Thinking
```

## Process

1. Check the existing `next_commit.tmp` file if it exists to see if it contains uncommitted work
2. Use the `edit` tool to replace the entire contents of `next_commit.tmp`
3. Follow the format exactly as specified
4. Include all significant changes in the bullet points
5. Ensure the file ends with the AI attribution line
6. The file will be overwritten for each new commit, so always start fresh
