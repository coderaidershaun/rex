# Milestones

Milestones represent major, meaningful checkpoints in the overall project journey. They answer "what does done look like at a high level?" A milestone is reached — not worked on directly — as the cumulative result of completing all its child objectives.

Think of milestones as the chapter titles of a project. They should be binary (achieved or not), significant enough to mark a phase transition, and largely independent of how the work gets done.

**Example:** "User authentication system is fully operational."

## JSON Schema

```json
{
  "id": "m-auth-system",
  "title": "User authentication system is fully operational",
  "description": "All auth flows working, security reviewed, deployed to staging",
  "status": "not-started",
  "references": ["docs/auth-spec.md"],
  "outputs": ["docs/auth-completion-report.md"],
  "checklist": [
    { "id": "c1", "item": "All objectives complete", "done": false },
    { "id": "c2", "item": "Security review passed", "done": false }
  ],
  "objectives": ["o-password-reset", "o-oauth-setup"],
  "upstream": [],
  "downstream": ["m-user-dashboard"]
}
```

## CLI Commands

### Create a milestone

```bash
rex milestone upsert \
  --id m-auth-system \
  --title "User authentication system is fully operational" \
  --description "All auth flows working, security reviewed, deployed to staging" \
  --add-reference docs/auth-spec.md \
  --add-output docs/auth-completion-report.md \
  --add-checklist "c1:All objectives complete" \
  --add-checklist "c2:Security review passed"
```

### Update a milestone

```bash
# Update status
rex milestone upsert --id m-auth-system --status in-progress

# Add a dependency
rex milestone upsert --id m-auth-system --add-upstream m-infrastructure

# Check off a checklist item
rex milestone upsert --id m-auth-system --check c1

# Multiple modifications in one call
rex milestone upsert --id m-auth-system \
  --status completed \
  --check c2 \
  --add-downstream m-user-dashboard
```

### Get a milestone

```bash
rex milestone get m-auth-system
```

Outputs the full milestone as JSON to stdout.

### List milestones

```bash
# All milestones
rex milestone list

# Filter by status
rex milestone list --status not-started
rex milestone list --status blocked
```

Outputs a JSON array to stdout.

### Remove a milestone

```bash
rex milestone remove m-auth-system
```

Cleans up upstream/downstream references in other milestones. Warns about any orphaned objectives that reference this milestone.

## List Modification Flags

These flags work on `upsert` to modify list fields:

| Flag | Effect |
|------|--------|
| `--add-reference <val>` | Append to references (deduplicated) |
| `--remove-reference <val>` | Remove from references |
| `--add-output <val>` | Append to outputs |
| `--remove-output <val>` | Remove from outputs |
| `--add-upstream <id>` | Add upstream dep (bidirectional) |
| `--remove-upstream <id>` | Remove upstream dep (bidirectional) |
| `--add-downstream <id>` | Add downstream dep (bidirectional) |
| `--remove-downstream <id>` | Remove downstream dep (bidirectional) |
| `--add-checklist <id:text>` | Add checklist item |
| `--remove-checklist <id>` | Remove checklist item |
| `--check <id>` | Mark checklist item done |
| `--uncheck <id>` | Mark checklist item not done |

All flags are repeatable (use multiple times in one command).
