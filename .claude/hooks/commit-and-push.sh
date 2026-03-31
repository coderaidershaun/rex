#!/usr/bin/env bash
set -euo pipefail

cd "$CLAUDE_PROJECT_DIR"

# Skip if not a git repo
if ! git rev-parse --is-inside-work-tree &>/dev/null; then
  exit 0
fi

# Skip if there are no changes to commit
if git diff --quiet && git diff --cached --quiet && [ -z "$(git ls-files --others --exclude-standard)" ]; then
  exit 0
fi

# Stage all changes
git add -A

# Commit with a timestamp-based message
git commit -m "auto-commit: agent stop $(date '+%Y-%m-%d %H:%M:%S')" --no-verify || true

# Push to remote (current branch)
git push || true
