---
description: Commit changes
---
# Commit Changes

1. Determine the project folder name for the current workspace.
2. Check if a folder specific commit template exists at `.templates\next-commit.txt`
3. Otherwise use a base template at `.templates\next-commit-base.txt`
4. Compose commit message following the template format to `./next-commit.txt` following `comment-file.md` rules.
5. Add AI usage declaration after a blank line as specified in `attribution.md`.
   a. Use the `ask_user_question` tool to ask the user about AI model if the model is unknown or the user hasn't previously specified a model for this session.
6. Run the commit command:
   a. Execute `git add -A && git commit -F ./next-commit.txt && mv ./next-commit.txt ./next-amend.txt`
   b. Set `SafeToAutoRun` to `false` — the command requires user approval since it creates a commit
   c. Wait for user approval before proceeding
