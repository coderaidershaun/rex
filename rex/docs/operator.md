# Rex Operator

The operator is the heartbeat of the rex harness. It processes one work item per invocation, dispatching the right agent(s) and recording what happened.

## Sequence of Operations

### Standard Phases (onboarding, design, planning, user-support)

```
1.  Get active project        →  rex project get-active
2.  Check lock status          →  If locked: true, STOP immediately
3.  Get next work item         →  rex project next-item
3b. Pre-dispatch validation    →  Check if item should be skipped (e.g., existing-code-exploration
                                  when no existing code — mark not-required and loop to step 3)
4.  Mark item in-progress      →  rex project update-status <item> in-progress
5.  Get recent history         →  rex history get-recent
6.  Prepare agent dispatch     →  Build prompt from item config
7.  Dispatch agent(s)          →  Agent tool (BLOCKING — never background)
8.  Check agent response       →  Respect "do not mark complete" signals
9.  Record history             →  rex history insert ...
10. Mark item complete         →  rex project update-status <item> completed
11. Manage history             →  Dispatch agent with rex-manage-history skill
12. Stop                       →  Report what was done and terminate
```

### Execution Phase

When `rex project next-item` returns the execution item (item: `"run"`, phase: `"execution"`), the operator resolves the actual work from the planning tree instead of the item itself. The execution item has empty inputs/outputs and `stop-on-finish: false` — the real work comes from `rex task next`, and a wrapping loop can continuously invoke the operator to process tasks:

```
1.  Get active project        →  rex project get-active
2.  Check lock status          →  If locked: true, STOP immediately
3.  Get next work item         →  rex project next-item
3a. Get next task              →  rex task next (returns task + objective + milestone)
    └─ If "NO TASKS"          →  Mark execution item complete, skip to 9
4.  Mark in-progress           →  rex project update-status <item> in-progress
                                  rex task upsert --id <task-id> --status in-progress
5.  Get recent history         →  rex history get-recent
6.  Prepare agent dispatch     →  Build prompt from task/objective/milestone context
7.  Dispatch agent(s)          →  Agent tool (BLOCKING — never background)
8.  Check response & complete  →  rex task upsert --id <task-id> --status completed
    └─ rex task next           →  Check if more tasks remain
    └─ If "NO TASKS"          →  Execution item → completed (step 10)
    └─ If tasks remain         →  Execution item stays in-progress (skip step 10)
9.  Record history             →  rex history insert (include task/obj/milestone entities)
10. Mark execution complete    →  rex project update-status <item> completed (ONLY if all tasks done)
11. Manage history             →  Dispatch agent with rex-manage-history skill
12. Stop                       →  Report task + execution status
```

## CLI Commands Used

| Step | Command | Purpose |
|------|---------|---------|
| 1 | `rex project get-active` | Get active project fields |
| 3 | `rex project next-item` | Get next actionable item from project-status.json |
| 3a | `rex task next` | Get next eligible task + objective + milestone (execution phase) |
| 4 | `rex project update-status <item> <status>` | Mark items in-progress or completed |
| 4 | `rex task upsert --id <id> --status in-progress` | Mark task in-progress (execution phase) |
| 5 | `rex history get-recent` | Get recent history for agent context |
| 8 | `rex task upsert --id <id> --status completed` | Mark task completed (execution phase) |
| 8 | `rex task next` | Check if more tasks remain (execution phase) |
| 9 | `rex history insert --id ... --timestamp ... --summary ... --entity ... --file ...` | Record what was done |
| 10 | `rex project update-status <item> completed` | Mark execution item complete (only when all tasks done) |

## Agent Dispatch

The work item's `agent` object controls how agents are dispatched:

```json
{
  "count": 1,          // Number of agents to spawn
  "effort": "high",    // Reasoning depth
  "model": "opus",     // Model to use
  "skills": ["..."]    // Skill(s) agents should invoke
}
```

### Task-Level Agent Override (Execution Phase)

During the execution phase, tasks can carry their own `agent` object. When present, the **task's agent config takes precedence** over the execution item's agent config. This allows different tasks to use different skills, models, and effort levels.

- If the task has an `agent` field → use the task's config
- If the task has no `agent` field → fall back to the execution item's config

### Model Mapping

| Item value | Agent model param |
|------------|-------------------|
| `"opus"` | `opus` |
| `"sonnet"` | `sonnet` |
| `"haiku"` | `haiku` |

### Effort Mapping

| Item value | Instruction to agent |
|------------|---------------------|
| `"medium"` | Apply moderate reasoning depth |
| `"high"` | Think carefully and thoroughly |
| `"max"` | Think very deeply, consider all angles |
| `"ultrathink"` | Think extremely deeply, exhaust every consideration |

### Single vs Multi-Agent

- **count = 1**: One agent, one dispatch. Straightforward.
- **count > 1**: Spawn N worker agents in parallel, each focusing on a different portion. Then one coordinator agent synthesizes their work into the final output(s).

**All dispatches are blocking.** The operator waits for every agent to complete before proceeding. This is critical for headless operation.

### Execution Phase Agent Context

During execution, agents receive the full planning context — not just the work item:

- **Task** — the specific work to do (title, description, checklist, outputs, references)
- **Objective** — the strategic outcome this task contributes to
- **Milestone** — the high-level checkpoint this work is part of
- **Project** — the overall project context
- **Recent history** — what's been done recently

If the task has its own `agent` field, use it for model, effort, and skills. Otherwise, the execution item's `agent` config is used as a fallback. The task always drives the actual work, inputs, and outputs.

## History Management

After each item completes, the operator invokes an agent with the `rex-manage-history` skill to keep the recent history section at 3 entries or fewer. Older entries are summarised and moved to the archived section.

## Pre-Dispatch Validation

Before dispatching an item, the operator checks whether the item should be skipped:

- **`existing-code-exploration`**: Read `rex/<project-id>/onboarding/existing-code.md`. If it indicates no existing code (greenfield), mark the item `not-required` and skip to the next item.

This prevents wasting tokens on items whose preconditions aren't met.

## Rules

- One item per invocation — no looping
- CLI only for data mutations — never write JSON files directly
- Blocking dispatch — never run agents in background
- Respect the lock — if locked, stop immediately
- Respect agent responses — if an agent says not to mark complete, don't
- Pass full context — agents receive the project object and recent history
- Execution uses `rex task next` — the planning tree drives work, not the execution item
- Task-level agent overrides execution-level agent — when a task has its own `agent` field, use it
- Mark tasks complete via CLI — `rex task upsert --id <id> --status completed`
- Only mark execution item complete when ALL tasks are finished
