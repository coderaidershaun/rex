# Rex Init

Initialize the rex harness in any repository so it works as a standalone agent-orchestrated development environment.

## Usage

```bash
rex init              # Interactive — prompts for agent OS
rex init --claude     # Non-interactive — Claude Code
rex init --cursor     # Non-interactive — Cursor
```

## What It Does

Copies all skills, hooks, configuration, and documentation into the current directory. The rex binary embeds these files at compile time, so no network access or external downloads are needed.

### Files Created

| Item | Claude Code | Cursor | Purpose |
|------|------------|--------|---------|
| Skills | `.claude/skills/` | `.cursor/skills/` | All rex and rust agent skills (40 directories) |
| Hook scripts | `.claude/hooks/` | `.cursor/hooks/` | `commit-and-push.sh` (auto-commit on agent stop) |
| Hook config | `.claude/settings.json` | `.cursor/hooks.json` | Registers the stop hook with the agent OS |
| Root file | `CLAUDE.md` | `AGENTS.md` | Points agents to `rex/docs/README.md` |
| Docs | `rex/docs/` | `rex/docs/` | All CLI and process documentation (9 files) |
| Registry | `rex/projects.json` | `rex/projects.json` | Empty project registry |

### Configuration Format Differences

The hook configuration follows each agent OS's native format:

**Claude Code** (`.claude/settings.json`):

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

**Cursor** (`.cursor/hooks.json`):

```json
{
  "version": 1,
  "hooks": {
    "stop": [
      {
        "type": "command",
        "command": ".cursor/hooks/commit-and-push.sh",
        "timeout": 30,
        "failClosed": false
      }
    ]
  }
}
```

- Hooks live in a dedicated `hooks.json` file (not inside settings)
- Top-level `"version": 1` field required
- Event names are lowercase (`stop`)
- Hook objects are flat (no nested matcher/hooks structure)
- Hook script paths are relative to the project root
- Each hook supports `timeout` (seconds) and `failClosed` (boolean)

### Skills

Both agent OSes use the same skill format: a directory containing a `SKILL.md` file with YAML frontmatter. Claude Code looks in `.claude/skills/`, Cursor looks in `.cursor/skills/` (and also `.claude/skills/` for backward compatibility).

### Root File

The root file (`CLAUDE.md` or `AGENTS.md`) gives the agent context about the rex harness. It contains a quick reference table linking to all documentation files under `rex/docs/`.

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

This means you can run `rex init` on a repo that already has `.claude/` or `.cursor/` configuration, custom skills, and existing hooks — rex will add only what's missing.

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
| `IO error: not a terminal` | Interactive mode requires a TTY — use `--claude` or `--cursor` flag |
| Permission denied on hooks | The init command sets hook scripts to `755` — re-run if permissions were changed |
