---
description: Amend changes
---
# Amend Changes

1. Determine the project folder name for the current workspace.
2. Check if a folder specific commit template exists at `.templates\next-commit.txt`
3. Otherwise use a base template at `.templates\next-commit-base.txt`
4. Check if `next-amend.txt` exists in the root folder
   - If exists, use `edit` tool to update the contents
   - Otherwise, read the current commit message with `git log -n1 --format="%B" HEAD` and create the file based on that content.
5. Compose commit message following the template format to `./next-amend.txt` following `comment-file.md` rules.
6. Add AI usage declaration after a blank line as specified in `attribution.md`.
   a. Use the `ask_user_question` tool to ask the user about AI model if the model is unknown or the user hasn't previously specified a model for this session.
7. Run the amend command:
   a. Execute `git add -A && git commit --amend -F ./next-amend.txt`
   b. Set `SafeToAutoRun` to `false` — the command requires user approval since it amends a commit
   c. Wait for user approval before proceeding
