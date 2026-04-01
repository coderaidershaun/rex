# Tasks

Tasks are the atomic units of work — concrete, actionable, and assignable. They answer "what specific thing needs to be done next?" A task should be completable in a single work session, have a clear definition of done, and require no further decomposition to execute. Tasks are the leaves of the planning tree.

**Example:** "Implement the token-generation endpoint for password reset."

## JSON Schema

```json
{
  "id": "t-token-endpoint",
  "objective_id": "o-password-reset",
  "title": "Implement the token-generation endpoint for password reset",
  "description": "POST /auth/reset-token — generates a cryptographically secure token, stores it with a 15-minute TTL, returns 202",
  "status": "not-started",
  "references": ["docs/auth-spec.md#token-generation", "o-password-reset"],
  "outputs": ["src/auth/reset_token.rs", "tests/auth/reset_token_test.rs"],
  "checklist": [
    { "id": "c1", "item": "Endpoint returns 202 with valid email", "done": false },
    { "id": "c2", "item": "Token expires after 15 minutes", "done": false },
    { "id": "c3", "item": "Rate limited to 3 requests per hour", "done": false }
  ],
  "upstream": [],
  "downstream": ["t-email-template"]
}
```

## CLI Commands

### Create a task

```bash
rex task upsert \
  --id t-token-endpoint \
  --objective o-password-reset \
  --title "Implement the token-generation endpoint for password reset" \
  --description "POST /auth/reset-token — generates a secure token, stores with 15min TTL, returns 202" \
  --add-reference "docs/auth-spec.md#token-generation" \
  --add-output src/auth/reset_token.rs \
  --add-output tests/auth/reset_token_test.rs \
  --add-checklist "c1:Endpoint returns 202 with valid email" \
  --add-checklist "c2:Token expires after 15 minutes" \
  --add-checklist "c3:Rate limited to 3 requests per hour"
```

The parent objective's `tasks` list is automatically updated to include `t-token-endpoint`.

### Update a task

```bash
# Mark as in-progress
rex task upsert --id t-token-endpoint --status in-progress

# Check off a checklist item
rex task upsert --id t-token-endpoint --check c1

# Complete the task
rex task upsert --id t-token-endpoint --status completed --check c2 --check c3

# Re-parent to a different objective
rex task upsert --id t-token-endpoint --objective o-auth-v2-reset

# Add a dependency chain
rex task upsert --id t-email-template --add-upstream t-token-endpoint
```

### Get a task

```bash
rex task get t-token-endpoint
```

### List tasks

```bash
# All tasks
rex task list

# Filter by objective
rex task list --objective o-password-reset

# Filter by status
rex task list --status blocked

# Both filters
rex task list --objective o-password-reset --status not-started
```

### Remove a task

```bash
rex task remove t-token-endpoint
```

Removes the task from its parent objective's `tasks` list and cleans up upstream/downstream references in sibling tasks.

## List Modification Flags

Same flags as milestones — see [milestones.md](milestones.md#list-modification-flags).

### Get the next task to work on

```bash
rex task next
```

Returns the highest-priority eligible task along with its parent objective and milestone as a single JSON object:

```json
{
  "task": { "id": "t-token-endpoint", ... },
  "objective": { "id": "o-password-reset", ... },
  "milestone": { "id": "m-auth", ... }
}
```

**Eligibility rules** — a task is eligible if:
- It is `in-progress` (resume unfinished work) or `not-started` with all upstream tasks `completed`
- Its parent objective is not `blocked` and all objective-level upstream deps are `completed`
- Its parent milestone is not `blocked` and all milestone-level upstream deps are `completed`

**Priority ordering** (highest to lowest):
1. `in-progress` tasks — always resume unfinished work first
2. Tasks under in-progress objectives in in-progress milestones — finish current work
3. Tasks under not-started objectives in in-progress milestones — continue current milestone
4. Tasks under in-progress objectives in not-started milestones — finish scattered objectives
5. All other eligible tasks — start new work

Within each tier, tasks are ranked by **transitive downstream impact** (tasks that unblock the most future work win), then by array position (milestones, objectives, tasks) as a tiebreaker.

Exits with an error if no eligible tasks remain (all completed or all blocked by unmet dependencies).

## Agentic Usage Patterns

### Discover next work

```bash
# Intelligent next-task selection (recommended)
rex task next

# Or manual filtering
rex task list --status not-started
rex task list --objective o-password-reset --status not-started
```

### Claim and work

```bash
# Start working
rex task upsert --id t-token-endpoint --status in-progress

# Check off items as you go
rex task upsert --id t-token-endpoint --check c1

# Complete
rex task upsert --id t-token-endpoint --status completed --check c2 --check c3
```

### Build the tree top-down

```bash
# 1. Create milestone
rex milestone upsert --id m-auth --title "Auth operational" --description "..."

# 2. Create objectives under it
rex objective upsert --id o-reset --milestone m-auth --title "Password reset" --description "..."
rex objective upsert --id o-oauth --milestone m-auth --title "OAuth integration" --description "..."

# 3. Create tasks under objectives
rex task upsert --id t-token --objective o-reset --title "Token endpoint" --description "..."
rex task upsert --id t-email --objective o-reset --title "Email template" --description "..." --add-upstream t-token
```
