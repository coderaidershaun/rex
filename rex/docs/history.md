# History

The history system tracks what work has been done across agent sessions. It provides context continuity so agents picking up work can understand what happened recently and what was done in the past.

History is stored at `rex/<project-id>/history.json` with two sections:

- **recent** — detailed entries from the last three agent sessions
- **archived** — compacted summaries of older work

The intended workflow: agents insert entries into `recent` as they work. When a session ends and the recent list grows beyond three sessions, older entries are compacted (summarized) and moved to `archived` via `insert-compacted` + `remove-from-recent`.

## JSON Schema

```json
{
  "recent": [
    {
      "id": "session-3-auth-impl",
      "timestamp": "2026-04-01T14:30:00Z",
      "summary": "Implemented token generation endpoint and email template for password reset",
      "entities": ["t-token-endpoint", "t-email-template"],
      "files": ["src/auth/reset_token.rs", "templates/reset_email.html"],
      "session": "agent-session-003"
    }
  ],
  "archived": [
    {
      "id": "compact-week-1",
      "timestamp": "2026-03-28T00:00:00Z",
      "summary": "Set up project scaffolding, defined auth milestones and objectives, completed initial design review",
      "entities": ["m-auth", "o-password-reset", "o-oauth"],
      "files": ["src/lib.rs", "docs/design/auth-architecture.md"],
      "session": null
    }
  ]
}
```

### Entry Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique kebab-case identifier for this entry |
| `timestamp` | string | ISO-8601 timestamp of when the entry was recorded |
| `summary` | string | Brief description of what was done |
| `entities` | string[] | Milestone/objective/task IDs that were affected |
| `files` | string[] | Files that were created or modified |
| `session` | string? | Agent session identifier, if available |

## CLI Commands

### Insert a recent entry

```bash
rex history insert-recent \
  --id session-3-auth-impl \
  --timestamp "2026-04-01T14:30:00Z" \
  --summary "Implemented token generation endpoint and email template" \
  --entity t-token-endpoint \
  --entity t-email-template \
  --file src/auth/reset_token.rs \
  --file templates/reset_email.html \
  --session agent-session-003
```

### Remove from recent

```bash
rex history remove-from-recent session-3-auth-impl
```

### Insert a compacted (archived) entry

```bash
rex history insert-compacted \
  --id compact-week-1 \
  --timestamp "2026-03-28T00:00:00Z" \
  --summary "Set up project scaffolding, defined auth milestones, completed design review" \
  --entity m-auth \
  --entity o-password-reset \
  --entity o-oauth
```

### Remove from compacted (archived)

```bash
rex history remove-from-compacted compact-week-1
```

### List all history

```bash
rex history list
```

Outputs the full `history.json` contents as JSON to stdout.

## Agentic Usage Patterns

### Session start — read context

```bash
rex history list
```

Agent reads recent and archived entries to understand what has been done.

### During a session — log work

```bash
rex history insert-recent \
  --id session-4-reset-ui \
  --timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --summary "Built the password reset UI form with validation" \
  --entity t-reset-ui \
  --file src/components/ResetForm.tsx \
  --session agent-session-004
```

### Session rotation — compact old entries

```bash
# Compact the oldest recent entry into an archived summary
rex history insert-compacted \
  --id compact-session-1 \
  --timestamp "2026-03-25T00:00:00Z" \
  --summary "Initial project setup: scaffolding, dependency config, CI pipeline"

# Remove the detailed recent entry
rex history remove-from-recent session-1-setup
```
