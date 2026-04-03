---
name: rex-publish-to-crates-io
description: Publish the rex crate to crates.io — handles the full release workflow including version bump analysis, Cargo.toml update, CHANGELOG.md generation, Cargo.lock refresh, committing/pushing via /rex-publish-to-git, and cargo publish. Use this skill when the user says "publish to crates", "release a new version", "bump the version and publish", "push to crates.io", "do a release", "ship it", or any variation of wanting to cut a new crate release. Also use when the rex operator or any agent needs to publish a new version of the crate after completing work.
disable-model-invocation: false
user-invocable: true
---

# Publish to crates.io

You handle the full release workflow: figure out the right version bump, update the metadata files, commit, push, and publish. The goal is a clean, one-shot release where the commit history, changelog, and crates.io all tell the same story.

---

## Step 1: Identify what changed since the last release

Find the current version from `Cargo.toml`, then review what's happened since that version was set:

```bash
grep '^version' Cargo.toml
```

```bash
git log --oneline $(git log --all --oneline --grep="Bump to v" -1 --format="%H")..HEAD
```

This gives you every commit since the last version bump. Read through them to understand the scope of changes — new features, bug fixes, refactors, skill updates, CLI changes.

If the grep for "Bump to v" doesn't find anything, fall back to `git log --oneline -30` and use your judgment about where the last release boundary was.

Also check for uncommitted changes that should be part of this release:

```bash
git status
git diff --stat
```

If there are meaningful uncommitted changes, they'll be included in the release commit. Note them as part of the changelog.

---

## Step 2: Determine the version bump

This project uses semantic versioning. Analyze the changes from Step 1 and recommend a bump:

- **Patch** (0.0.X): Bug fixes, minor tweaks, skill instruction updates, doc changes, small improvements that don't add new user-facing capabilities
- **Minor** (0.X.0): New commands, new features, new skills that add functionality, significant workflow changes
- **Major** (X.0.0): Breaking changes to the CLI interface, removal of commands, changes that require users to update their workflows

For a pre-1.0 crate like rex, the bar for major bumps is higher — most changes are patch or minor. Present your recommendation to the user with a brief rationale, but don't block on confirmation unless the user has specifically asked to approve the version. If the user told you what version to use, use that.

---

## Step 3: Update Cargo.toml

Edit the `version` field in `Cargo.toml`:

```toml
version = "X.Y.Z"
```

Change only the version line. Don't touch anything else in the file.

---

## Step 4: Refresh Cargo.lock

The lock file needs to reflect the new version:

```bash
cargo check
```

This regenerates `Cargo.lock` with the updated version. If `cargo check` fails, investigate — there may be a compilation error that needs fixing before release. Don't publish a broken crate.

---

## Step 5: Update CHANGELOG.md

Add a new section at the top of the changelog (below the `# Changelog` header and the description line), following the established format:

```markdown
## X.Y.Z — YYYY-MM-DD

- **Feature/change name** — Description of what changed and why it matters.
- **Another change** — Description.
```

**Guidelines for writing changelog entries:**
- Each entry starts with a bolded short name for the change, followed by an em dash and a description
- Group related changes into a single entry when they're part of the same effort
- Focus on what the user/developer sees or experiences, not internal implementation details
- Use present tense ("Add", "Remove", "Fix", not "Added", "Removed", "Fixed")
- Skip auto-commits and trivial changes (typo fixes, formatting) — they're noise in a changelog
- Order entries by significance: most impactful changes first

Read the existing `CHANGELOG.md` to match the exact style — the project has a consistent voice across entries. Mirror that.

---

## Step 6: Commit and push via /rex-publish-to-git

Invoke the `/rex-publish-to-git` skill to commit all the release changes and push them. The skill will review the diff and create a meaningful commit message.

However, for version bumps specifically, the commit message should follow the project's established pattern:

```
Bump to vX.Y.Z — brief summary of what's in this release
```

Before invoking the skill, stage the changes yourself so the commit message reflects a version bump rather than a generic description:

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
```

Also stage any other files that are part of this release (source changes, new skills, etc.):

```bash
git add -A
```

Then commit with the version bump message directly — don't invoke `/rex-publish-to-git` for the commit since the message format is specific to releases:

```bash
git commit -m "$(cat <<'EOF'
Bump to vX.Y.Z — brief summary

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

Then push:

```bash
git push
```

If push fails due to remote changes, pull with rebase first:

```bash
git pull --rebase && git push
```

---

## Step 7: Publish to crates.io

```bash
cargo publish
```

If publish fails:
- **Authentication error**: The user needs to run `cargo login` with their crates.io token. Tell them and stop.
- **Validation error** (missing fields, too many keywords, etc.): Fix the issue, re-commit via Step 6, and retry.
- **Version already exists**: The version is already published. This usually means a retry — confirm with the user whether they want to bump again or stop.
- **Compilation error**: Something is broken. Fix it first, then restart from Step 3.

---

## Step 8: Report

```
Published rex-cli vX.Y.Z to crates.io
- Version: X.Y.Z (patch|minor|major bump)
- Commit: <short-hash>
- Changes: <count> changelog entries
```

---

## Rules

- Never publish a version that doesn't compile. Always run `cargo check` before publishing.
- Never skip the CHANGELOG update — every published version gets a changelog entry.
- Never reuse a version number that's already on crates.io. If `cargo publish` says the version exists, bump again.
- Don't include sensitive files in the publish. The `exclude` field in Cargo.toml handles this, but double-check if new sensitive paths have been added.
- If there are no meaningful changes since the last version (only auto-commits with no substance), tell the user there's nothing worth releasing and stop.
- The Cargo.lock commit and the version bump should be a single commit, not separate ones. The project has historically done these as separate commits, but combining them is cleaner.
