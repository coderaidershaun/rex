# Rex Init

Initialize the rex harness in any repository so it works as a standalone agent-orchestrated development environment.

## Usage

```bash
rex init
```

## What It Does

Copies all skills, hooks, configuration, and documentation into the current directory. The rex binary embeds these files at compile time, so no network access or external downloads are needed.

### Files Created

| Item | Location | Purpose |
|------|----------|---------|
| Skills | `.claude/skills/` | All rex and rust agent skills (41 directories) |
| Hook scripts | `.claude/hooks/` | `commit-and-push.sh` (auto-commit on agent stop) |
| Hook config | `.claude/settings.json` | Registers the stop hook with Claude Code |
| Root file | `CLAUDE.md` | Points agents to `rex/docs/README.md` |
| Docs | `rex/docs/` | All CLI and process documentation (9 files) |
| Registry | `rex/projects.json` | Empty project registry |

### Configuration Format

The hook configuration follows Claude Code's native format (`.claude/settings.json`):

```json
{
  "hooks": {
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "$CLAUDE_PROJECT_DIR/.claude/hooks/commit-and-push.sh"
          }
        ]
      }
    ]
  }
}
```

- Hooks are nested inside `settings.json` under a `"hooks"` key
- Event names are PascalCase (`Stop`)
- Each event has an array of matcher objects, each containing a `hooks` array
- Hook script paths use `$CLAUDE_PROJECT_DIR` for the project root

### Skills

Skills use a directory containing a `SKILL.md` file with YAML frontmatter. Claude Code looks in `.claude/skills/`.

### Root File

The root file (`CLAUDE.md`) gives the agent context about the rex harness. It contains a quick reference table linking to all documentation files under `rex/docs/`.

## Splice Behavior

Running `rex init` is safe on directories that already have agent configuration. Existing files are never overwritten:

| Scenario | Behavior |
|----------|----------|
| File doesn't exist | Created |
| File already exists | Skipped |
| Root file exists but has no rex section | Rex section appended |
| Root file exists with rex section | Skipped |
| Hook config exists but no rex hooks | Rex hooks merged in |
| Hook config exists with rex hooks | Skipped |

This means you can run `rex init` on a repo that already has `.claude/` configuration, custom skills, and existing hooks — rex will add only what's missing.

## Init Via Project Create

When running `rex project create`, you are prompted whether to initialize the rex harness **inside** the project directory. If you choose Yes, `rex init` is run automatically inside the project directory — no separate `rex init` step is needed. This creates a self-contained project with its own `.claude/`, `rex/docs/`, and `rex/projects.json`.

This is useful for monorepos created with `rex mono empty`, where each project under `libs/` can have its own independent harness.

## After Init

Once initialized, create your first project:

```bash
rex project create
```

Then run the operator to start processing:

```
/rex-operator
```

See [README.md](README.md) for the full end-to-end process.

## Error Handling

| Error | Cause |
|-------|-------|
| Permission denied on hooks | The init command sets hook scripts to `755` — re-run if permissions were changed |
