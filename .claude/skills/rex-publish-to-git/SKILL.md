---
name: rex-publish-to-git
description: Commit and push the current working state to git with a meaningful, descriptive commit message. Use this skill when the operator or an agent finishes a piece of work and needs to publish it to the remote repository, when the user says "commit and push", "publish to git", "save to git", "push my changes", or any variation of wanting to commit the current state and push it upstream. Also use this when the rex operator needs to persist completed work between sessions.
disable-model-invocation: false
user-invocable: true
---

# Publish to Git

Your job is to commit the current working changes to git with a clear, descriptive commit message and push to the remote. This is the intentional, meaningful counterpart to the auto-commit hook — where the hook saves a timestamped snapshot on session end, this skill creates a proper commit that communicates what was done and why.

---

## Step 1: Check for changes

Run these commands to understand what needs committing:

```bash
git status
```

```bash
git diff --stat
```

```bash
git diff --cached --stat
```

If there are no changes (no modified files, no staged files, no untracked files), report that there's nothing to commit and stop.

---

## Step 2: Review the changes

Understand what was changed so you can write a good commit message:

```bash
git diff
```

```bash
git diff --cached
```

Also check for untracked files that should be included:

```bash
git ls-files --others --exclude-standard
```

Read through the diffs to understand the nature of the changes — are they a new feature, a bug fix, a refactor, documentation, config changes, design documents, planning artifacts?

---

## Step 3: Stage the changes

Stage all relevant changes. For most rex workflow commits, this means everything:

```bash
git add -A
```

**Exception:** If you spot files that clearly should not be committed (credentials, `.env` files, large binaries, temp files), exclude them. Mention what you skipped and why.

---

## Step 4: Write the commit message

Write a commit message that follows the project's conventions:

**Format:**
```
<summary line — what was done, imperative mood, under 72 chars>

<optional body — why it was done, context, what changed at a high level>

Co-Authored-By: Claude <noreply@anthropic.com>
```

**Guidelines for the summary line:**
- Use imperative mood ("Add authentication endpoint" not "Added authentication endpoint")
- Be specific about what changed ("Implement JWT token generation" not "Update code")
- Keep it under 72 characters
- Don't end with a period

**When to include a body:**
- Multiple files changed across different concerns — explain what ties them together
- Non-obvious design decisions — explain why this approach was chosen
- Breaking changes or migrations — call them out

**When the commit is on behalf of the rex operator or a rex agent**, tie the message to the work item context if available. For example: "Design error handling strategy for auth module" rather than "Write errors.md".

---

## Step 5: Commit

Create the commit:

```bash
git commit -m "<message>"
```

Use a heredoc if the message has multiple lines:

```bash
git commit -m "$(cat <<'EOF'
Summary line here

Body paragraph here if needed.

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Step 6: Push

Push to the remote:

```bash
git push
```

If the current branch has no upstream tracking branch, set one:

```bash
git push -u origin $(git branch --show-current)
```

If the push fails due to remote changes, pull with rebase first:

```bash
git pull --rebase && git push
```

---

## Step 7: Report

Confirm what was done:

```
Published to git:
- Branch: <branch-name>
- Commit: <short-hash> <summary-line>
- Files: <count> changed
```

Keep it brief. The user can read the commit details themselves.

---

## Rules

- Never commit files that contain secrets (`.env`, credentials, API keys, tokens). If you find any, warn the user and exclude them.
- Never force-push. If there's a conflict, pull --rebase first. If rebase fails, stop and report the conflict to the user.
- Never amend previous commits unless explicitly asked. Always create new commits.
- Always include the `Co-Authored-By` trailer.
- If there are no changes to commit, say so and stop. Don't create empty commits.
- Don't skip git hooks (`--no-verify`) unless the user explicitly asks. If a hook fails, investigate and report rather than bypassing.
