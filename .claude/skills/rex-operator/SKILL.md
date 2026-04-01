---
name: rex-operator
description: The heartbeat of the rex harness. Stewards the active project from one item to the next — fetching the current work item, dispatching agents with the right skills/model/effort, recording history, and managing session lifecycle. Use this skill whenever the user says "run the operator", "rex go", "next item", "continue the project", "start working", "operate", or any variation of wanting to advance the active rex project. Also trigger when the user invokes /rex-operator directly.
disable-model-invocation: false
user-invocable: true
---

# Rex Operator

You are the operator — the orchestrator that drives a rex project forward one item at a time. Each invocation processes exactly one work item from the active project's `project-status.json`, executes it (either directly or via sub-agents), records what happened, and continues or stops.

**Hybrid dispatch:** Items that require user interaction (onboarding, user-support, and design user-acceptance) are executed **directly by you** — you invoke the skill yourself and talk to the user. Non-interactive items (other design, planning, execution) are dispatched to **sub-agents**.

Do not ask the user questions unless the work item's skill requires user interaction. Do not improvise or skip steps. Follow the sequence exactly.

---

## Step 1: Get the active project

```bash
rex project get-active
```

Parse the output to extract all project fields. You need:
- `id` — the project identifier (used in file paths like `rex/<id>/...`)
- `directory` — the project's working directory
- `locked` — whether the project is locked

If there is no active project, report this to the user and stop.

---

## Step 2: Check lock status

If `locked` is `true`: **stop immediately**. Print a message like "Project is locked. Operator cannot proceed." and terminate the session. Do not continue to any further steps. No exceptions.

---

## Step 3: Get the next work item

```bash
rex project next-item
```

This returns a JSON object describing the next actionable item from `project-status.json`. It includes:

```json
{
  "item": "goal",
  "phase": "onboarding",
  "stop-on-finish": false,
  "agent": {
    "count": 1,
    "effort": "high",
    "model": "opus",
    "skills": ["rex-onboarding-goal"]
  },
  "inputs": [],
  "outputs": ["rex/my-project/onboarding/goal.md"],
  "status": "not-started"
}
```

If all items are completed, report this and stop.

**After getting the item, check its `phase` field.** If the phase is `"execution"`, follow the **Execution Phase** path described in Step 3a. For all other phases (onboarding, design, planning, user-support), continue with the standard flow at Step 4.

---

## Step 3a: Execution Phase — Resolve the Next Task

The execution phase works differently from other phases. Instead of the item itself describing a single piece of work, the execution item is a container — the real work lives in the planning tree (milestones → objectives → tasks). The operator uses `rex task next` to find the next task to work on, and dispatches an agent for that specific task.

```bash
rex task next
```

This returns the next eligible task along with its parent objective and milestone:

```json
{
  "task": {
    "id": "t-token-endpoint",
    "objective_id": "o-password-reset",
    "title": "Implement the token-generation endpoint",
    "description": "POST /auth/reset-token — generates a secure token...",
    "status": "not-started",
    "references": ["docs/auth-spec.md#token-generation"],
    "outputs": ["src/auth/reset_token.rs"],
    "checklist": [...],
    "upstream": [],
    "downstream": ["t-email-template"]
  },
  "objective": {
    "id": "o-password-reset",
    "milestone_id": "m-auth-system",
    "title": "Users can securely reset their passwords via email",
    ...
  },
  "milestone": {
    "id": "m-auth-system",
    "title": "User authentication system is fully operational",
    ...
  }
}
```

**If `rex task next` returns "NO TASKS - Please mark as item complete":** All tasks in the planning tree are done. Mark the execution item as complete and proceed to history/stop:

```bash
rex project update-status <execution-item-name> completed
```

Then skip to Step 9 to record history.

**Otherwise:** You have a task to work on. Save the full task, objective, and milestone objects — you'll need them for the agent prompt. Continue to Step 4.

---

## Step 4: Mark as in-progress

### Standard phases (onboarding, design, planning, user-support)

```bash
rex project update-status <item-name> in-progress
```

### Execution phase

Mark both the execution item and the specific task as in-progress:

```bash
rex project update-status <execution-item-name> in-progress
rex task upsert --id <task-id> --status in-progress
```

---

## Step 5: Get recent history

```bash
rex history get-recent
```

Capture the output — you'll pass this context to the agent(s) so they understand what's happened recently in the project.

---

## Step 6: Prepare the dispatch

### Determine dispatch mode

Check whether this item requires user interaction:

- **Direct invoke** if: phase is `"onboarding"` or `"user-support"`, OR the item name is `"user-acceptance"`
- **Sub-agent dispatch** if: everything else (design items other than user-acceptance, planning, execution)

This determines how the item gets executed in Step 7.

---

### Direct invoke (user-interactive items)

When the dispatch mode is **direct invoke**, the operator does NOT spawn a sub-agent. You ARE the agent. The user will talk directly to the skill through you — no relay, no middleman.

Prepare as follows:

1. **Read all `inputs` files yourself.** Don't delegate this — you need the context.
2. **Note the effort level.** Before invoking the skill, state the effort level in your output:
   - `"medium"` → "Applying moderate reasoning depth."
   - `"high"` → "Thinking carefully and thoroughly."
   - `"max"` → "ultrathink. Thinking very deeply about this item."
   - `"ultrathink"` → "ultrathink. Thinking extremely deeply about this critical item."
   
   The literal word `ultrathink` must appear for max/ultrathink levels — it triggers deep reasoning.
3. **`agent.model` is informational only** — you run on the session's model. This is fine because interactive items benefit from the user's chosen model.
4. **`agent.count` is ignored** — you are one agent. No parallelization for interactive items.
5. **Proceed to Step 7** to invoke the skill(s).

---

### Sub-agent dispatch (non-interactive items)

When the dispatch mode is **sub-agent dispatch**, build a prompt for the sub-agent.

From the work item JSON, extract:

| Field | What it tells you |
|-------|-------------------|
| `agent.count` | How many agents to spawn |
| `agent.model` | Which model to use: `"opus"` → opus, `"sonnet"` → sonnet, `"haiku"` → haiku |
| `agent.effort` | Reasoning depth to request in the agent prompt |
| `agent.skills` | Which skill(s) to invoke — the agent should use these |
| `inputs` | Files the agent MUST read before executing |
| `outputs` | Files the agent MUST write to |

#### Building the agent prompt

The prompt you give each sub-agent must include:

1. **The full project object as JSON** — paste the complete project object (from step 1) so the agent has all metadata: id, category, complexity, title, subtitle, description, directory, locked status.
2. **Recent history** — paste the recent history JSON (from step 5) so the agent knows what's been done.
3. **The skill(s) to invoke** — tell the agent which skill(s) to invoke from `agent.skills`, using the Skill tool. Be explicit: "You MUST invoke the skill `/<skill-name>` using the Skill tool. Do not follow from memory — actually call it."
   - **Single skill** (`agent.skills` has 1 entry): The agent invokes that one skill.
   - **Multiple skills** (`agent.skills` has 2+ entries): List ALL skills and tell the agent to invoke them sequentially, in order.
4. **Input files** — list every file in `inputs` and instruct the agent to read them before starting work.
5. **Output files** — list every file in `outputs` and instruct the agent to write its results there.
6. **Effort level** — include the effort level as a literal instruction at the end of the prompt. Map the effort field:
   - `"medium"` → "Apply moderate reasoning depth."
   - `"high"` → "Think carefully and thoroughly."
   - `"max"` → "Think very deeply. Take your time and consider all angles. ultrathink"
   - `"ultrathink"` → "ultrathink. Think extremely deeply. This is a critical task — exhaust every consideration before concluding."

   **Important:** For `"max"` and `"ultrathink"` effort levels, the literal word `ultrathink` must appear in the prompt — it triggers deep reasoning mode.

#### Example agent prompt (sub-agent dispatch)

```
You are working on the rex project "My Auth System" (id: auth-system).

## Project (full metadata)
{
  "id": "auth-system",
  "category": "binary",
  "complexity": "high",
  "title": "My Auth System",
  "subtitle": "JWT-based authentication service",
  "description": "A Rust binary that provides authentication endpoints with JWT sessions",
  "directory": "/Users/someone/Code/auth-system",
  "locked": false
}

## Recent History
<paste recent history JSON or "No recent history.">

## Your Task
You MUST invoke the skill `/<skill-from-agent.skills>` using the Skill tool. Do not follow the skill from memory — actually call it via the Skill tool.

Before you begin, read these input files:
- rex/auth-system/onboarding/goal.md

Write your output to:
- rex/auth-system/design/architecture.md

ultrathink. Think very deeply. Take your time and consider all angles.
```

### Execution phase

For execution, the agent prompt is built from the **task** (from `rex task next`), not the execution item. The execution item's `agent` config still controls the model, effort, and skills — but the task provides the actual work, inputs, and outputs.

#### Building the execution agent prompt

The prompt must include:

1. **The project context** — same as standard phases.
2. **Recent history** — same as standard phases.
3. **The milestone context** — pass the full milestone object so the agent understands the high-level goal it's contributing to.
4. **The objective context** — pass the full objective object so the agent understands the strategic outcome this task serves.
5. **The task details** — pass the full task object: title, description, checklist items, references, outputs, upstream/downstream dependencies.
6. **The skill to use** — from `agent.skills` on the execution item.
7. **Input files** — the task's `references` array contains file paths and entity IDs the agent should read. Pass all file paths as inputs.
8. **Output files** — the task's `outputs` array lists files the agent must write to.
9. **Effort level** — from `agent.effort` on the execution item.

#### Example agent prompt (execution phase)

```
You are working on the rex project "My Auth System" (id: auth-system).

## Project (full metadata)
{
  "id": "auth-system",
  "category": "binary",
  "complexity": "high",
  "title": "My Auth System",
  "subtitle": "JWT-based authentication service",
  "description": "A Rust binary that provides authentication endpoints with JWT sessions",
  "directory": "/Users/someone/Code/auth-system",
  "locked": false
}

## Recent History
<paste recent history JSON or "No recent history.">

## Milestone
<paste full milestone JSON>

## Objective
<paste full objective JSON>

## Your Task
<paste full task JSON>

You MUST invoke the skill `/<skill-from-agent.skills>` using the Skill tool. Do not follow the skill from memory — actually call it via the Skill tool.

Before you begin, read these reference/input files:
- docs/auth-spec.md#token-generation

Write your output to:
- src/auth/reset_token.rs
- tests/auth/reset_token_test.rs

ultrathink. Think very deeply. Take your time and consider all angles.
```

---

## Step 7: Execute the item

### Direct invoke (user-interactive items)

When the dispatch mode is **direct invoke** (from Step 6), you execute the skill yourself. No sub-agents. No relay. You ARE the agent.

1. **Read all `inputs` files** listed in the work item.
2. **Invoke each skill** in `agent.skills` sequentially using the Skill tool (e.g., `skill: "rex-onboarding-goal"`).
3. **The skill runs in your context.** It will ask the user questions directly. The user responds directly to you. No middleman. No paraphrasing.
4. **If `agent.skills` has multiple entries**, invoke them in order. Complete each skill fully before moving to the next.
5. **Write output files** as instructed by the skill.

That's it. You are the agent doing the work. Proceed to Step 8 when the skill completes.

---

### Sub-agent dispatch — single agent (count = 1)

Spawn one agent using the Agent tool with these **exact parameters**:

| Parameter | Value |
|-----------|-------|
| `prompt` | The full prompt from Step 6 |
| `model` | Map directly from `agent.model`: `"opus"` → `"opus"`, `"sonnet"` → `"sonnet"`, `"haiku"` → `"haiku"` |
| `description` | `"rex: <item-name>"` (or `"rex: <task-id>"` for execution phase) |
| `run_in_background` | **Do NOT set this.** The operator must block and wait. |

**You must set the `model` parameter on the Agent tool call itself** — not just mention the model in the prompt text. This is how the agent gets the right model.

### Sub-agent dispatch — multiple agents (count > 1)

When `agent.count` is greater than 1, spawn that many worker agents plus one coordinator agent. **Every worker agent MUST use the same `model` from `agent.model`.** The model is not optional — it must be set on each Agent tool call.

1. **Worker agents** — Spawn `agent.count` agents in parallel (in a single message with multiple Agent tool calls). Each worker:
   - Gets the **same base prompt** (full project JSON, history, inputs, outputs, effort level)
   - Gets the **same model** (`agent.model` — set on each Agent tool call)
   - Gets the **same skill(s)** (`agent.skills` — listed in the prompt)
   - Gets a **role differentiator** to split the work:
     - Worker 1: "You are worker 1 of N. Focus on the first portion of the analysis."
     - Worker 2: "You are worker 2 of N. Focus on the second portion of the analysis."
     - etc.
   
   Each worker should write its findings to a temporary intermediate file, e.g., `rex/<project-id>/<phase>/<item>-worker-<N>.md`

2. **Coordinator agent** — After all workers complete, spawn one final agent (same `model`) that:
   - Reads all worker output files
   - Synthesizes them into the final output file(s) specified in `outputs`
   - Cleans up the intermediate worker files

**Critical: Do not launch agents in the background and move on.** The operator must wait until all agents complete and report back. This is essential for headless operation.

**Note:** Sub-agents dispatched here are always non-interactive. They should never need to ask the user questions. If a sub-agent unexpectedly returns a question instead of completing its work, leave the item as in-progress and report what happened.

---

## Step 8: Check completion and mark task complete

### Direct invoke (user-interactive items)

Once the skill completes:
- If the skill wrote its output files and the conversation reached a natural conclusion, proceed to Step 9.
- If the skill indicated it should not be marked complete (e.g., user-acceptance where the user said "stop for now"), leave the item as `in-progress` and skip to Step 11.

### Sub-agent dispatch (standard phases)

Once the agent(s) report back, check their response text.

In rare cases, an agent may instruct you **not to mark the item as complete** — for example, if it encountered an issue or wants the item to remain in-progress for a follow-up session.

- If the agent says **do not mark complete**: leave the item as `in-progress` and skip to Step 11.
- Otherwise: continue to Step 9.

### Execution phase

If the agent says **do not mark complete**: leave the task as `in-progress` and skip to Step 11.

Otherwise, **mark the task as complete**:

```bash
rex task upsert --id <task-id> --status completed
```

Then check whether all tasks in the planning tree are now done:

```bash
rex task next
```

- If `rex task next` returns **"NO TASKS - Please mark as item complete"**: all tasks are finished. The execution phase is done — you will mark the execution item as complete in Step 10.
- If `rex task next` returns **another task**: more work remains. The execution item stays `in-progress` — do NOT mark it as complete. Continue to Step 9 to record history (skipping Step 10).

---

## Step 9: Record history

Insert a history entry for what was just done:

### Standard phases

```bash
rex history insert-recent \
  --id "session-<item-name>-<timestamp-short>" \
  --timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --summary "<brief description of what the agent accomplished>" \
  --entity <item-name> \
  --file <each-output-file>
```

### Execution phase

```bash
rex history insert-recent \
  --id "session-<task-id>-<timestamp-short>" \
  --timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --summary "<brief description of what the agent accomplished on the task>" \
  --entity <task-id> \
  --entity <objective-id> \
  --entity <milestone-id> \
  --file <each-task-output-file>
```

Include the task, objective, and milestone IDs as entities so the history captures the full context of what was worked on.

The summary should be a concise description of what was accomplished — not a dump of the agent's full response. One or two sentences.

---

## Step 10: Mark item as complete

### Standard phases

```bash
rex project update-status <item-name> completed
```

### Execution phase

Only mark the execution item as complete if Step 8 confirmed that **all tasks are done** (i.e., `rex task next` returned "NO TASKS").

```bash
rex project update-status <execution-item-name> completed
```

If tasks remain, **skip this step entirely** — the execution item stays as `in-progress`. It will be picked up again on the next operator invocation, which will run `rex task next` to find the next task.

---

## Step 11: Manage history

Dispatch an agent with the `rex-manage-history` skill to condense/archive history if the recent section has grown beyond 3 entries:

```
Use the skill "rex-manage-history" to check and condense the project history for project "<project-id>".
```

Use the same model as the work item's agent, or default to `sonnet` (history management is lightweight). **Wait for this agent to complete before proceeding.**

---

## Step 12: Continue or stop

Check the `stop-on-finish` field from the work item (captured in Step 3).

### If `stop-on-finish` is `false` — continue

The operator should **loop back to Step 3** to get the next work item and keep processing. Do not stop. Print a brief status line and continue:

```
Completed: <item-name> (<phase>). Continuing to next item...
```

This is the default for most items. The operator keeps going until it hits an item with `stop-on-finish: true` or runs out of items.

### If `stop-on-finish` is `true` — stop

The operator's job for this invocation is done. Report what was processed and stop.

#### Standard phases

```
Operator complete (stop-on-finish).
- Item: <item-name> (<phase>)
- Status: completed | left in-progress
- Output(s): <list of output files>
```

#### Execution phase

```
Operator complete (stop-on-finish).
- Phase: execution
- Task: <task-id> — <task-title>
- Task status: completed | left in-progress
- Objective: <objective-id> — <objective-title>
- Milestone: <milestone-id> — <milestone-title>
- Execution item status: completed (all tasks done) | in-progress (tasks remaining)
- Output(s): <list of task output files>
```

### If the item was left in-progress — always stop

If the item was not marked complete (agent said don't mark complete, or execution has remaining tasks), always stop regardless of `stop-on-finish`. The item needs another invocation.

---

## CLI Quick Reference

These are the **exact** commands available. Do not invent commands that aren't listed here.

```
# Project
rex project get-active
rex project next-item
rex project create
rex project remove <id>
rex project activate <id>
rex project update-status <item> <status>
rex project update-title "<title>"
rex project update-subtitle "<subtitle>"
rex project update-description "<description>"
rex project update-category <binary|library|refactor>
rex project update-complexity <low|medium|high>
rex project update-directory "<path>"

# Planning
rex milestone upsert --id <id> [--title --description --status] [--add-reference ...] [--add-upstream ...]
rex milestone get <id>
rex milestone list [--status <status>]
rex milestone remove <id>

rex objective upsert --id <id> [--milestone --title --description --status] [--add-reference ...] [--add-upstream ...]
rex objective get <id>
rex objective list [--milestone <id>] [--status <status>]
rex objective remove <id>

rex task upsert --id <id> [--objective --title --description --status] [--add-reference ...] [--add-upstream ...]
rex task get <id>
rex task list [--objective <id>] [--status <status>]
rex task remove <id>
rex task next

# History
rex history list
rex history get-recent
rex history insert-recent --id <id> --timestamp <iso8601> --summary "<text>" [--entity ...] [--file ...] [--session <id>]
rex history remove-from-recent <id>
rex history insert-compacted --id <id> --timestamp <iso8601> --summary "<text>" [--entity ...] [--file ...] [--session <id>]
rex history remove-from-compacted <id>

# Checklist
rex checklist init [--date <YYYY-MM-DD>]
rex checklist add --category <cat> --id <id> --title "<title>" --description "<desc>" [--phase <phase>]
rex checklist list [--category <cat>] [--phase <phase>] [--complete] [--incomplete]
rex checklist get <id>
rex checklist update <id> [--title --description --phase]
rex checklist complete <id>
rex checklist uncomplete <id>
rex checklist remove <id>
rex checklist set-context "<text>"
```

**Important:** There is no `rex project update` command. Title, subtitle, and description must be updated with separate commands (`update-title`, `update-subtitle`, `update-description`).
---

## Rules

- **Dispatch mode is determined by the item.** Onboarding, user-support, and design `user-acceptance` items use **direct invoke** (you are the agent). Everything else uses **sub-agent dispatch**. Never spawn a sub-agent for an interactive item. Never direct-invoke a non-interactive item.
- **The work item is the contract.** Every field must be honored:
  - `agent.model` → for sub-agent dispatch: set on the Agent tool `model` parameter (required). For direct invoke: informational only (you run on the session's model).
  - `agent.skills` → every skill listed must be invoked via the Skill tool. If multiple, invoke in order.
  - `agent.count` → for sub-agent dispatch: spawn exactly this many workers. For direct invoke: ignored (you are one agent).
  - `agent.effort` → map to the correct effort instruction including `ultrathink` keyword for max/ultrathink.
  - `inputs` → every file listed must be read before starting.
  - `outputs` → every file listed must be written.
- **Respect `stop-on-finish`.** After completing an item, check its `stop-on-finish` field. If `false`, loop back to Step 3 and process the next item. If `true`, stop. If the item was left in-progress, always stop.
- **CLI only for data management.** All reads and writes to rex data structures go through `rex` commands. Never write `project-status.json`, `history.json`, or `planning.json` directly.
- **Blocking dispatch.** Never launch agents in the background. Always wait for completion. This is critical for headless/automated operation.
- **Respect the lock.** If the project is locked, stop immediately. No exceptions, no "let me just check one thing."
- **Respect agent responses.** If an agent says not to mark complete, don't mark complete.
- **Pass full context.** Agents should receive the project object and recent history so they have everything they need.
- **Execution uses `rex task next`, not the item itself.** During execution, the planning tree drives what gets worked on. The execution item's `agent` config provides the model/effort/skills, but the task provides the work, inputs, and outputs.
- **Mark tasks complete via CLI.** Always run `rex task upsert --id <id> --status completed` when a task is done. Only mark the execution item complete when all tasks are finished.
