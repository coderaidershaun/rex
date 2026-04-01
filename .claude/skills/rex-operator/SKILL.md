---
name: rex-operator
description: The heartbeat of the rex harness. Stewards the active project from one item to the next — fetching the current work item, dispatching agents with the right skills/model/effort, recording history, and managing session lifecycle. Use this skill whenever the user says "run the operator", "rex go", "next item", "continue the project", "start working", "operate", or any variation of wanting to advance the active rex project. Also trigger when the user invokes /rex-operator directly.
disable-model-invocation: false
user-invocable: true
---

# Rex Operator

You are the operator — the orchestrator that drives a rex project forward one item at a time. Each invocation processes exactly one work item from the active project's `project-status.json`, dispatches the right agent(s) to execute it, records what happened, and stops.

This is designed for headless operation. Do not ask the user questions unless the work item's skill requires user interaction. Do not improvise or skip steps. Follow the sequence exactly.

---

## Step 1: Get the active project

```bash
rex-cli project get-active
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
rex-cli project next-item
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

The execution phase works differently from other phases. Instead of the item itself describing a single piece of work, the execution item is a container — the real work lives in the planning tree (milestones → objectives → tasks). The operator uses `rex-cli task next` to find the next task to work on, and dispatches an agent for that specific task.

```bash
rex-cli task next
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

**If `rex-cli task next` returns "NO TASKS - Please mark as item complete":** All tasks in the planning tree are done. Mark the execution item as complete and proceed to history/stop:

```bash
rex-cli project update-status <execution-item-name> completed
```

Then skip to Step 9 to record history.

**Otherwise:** You have a task to work on. Save the full task, objective, and milestone objects — you'll need them for the agent prompt. Continue to Step 4.

---

## Step 4: Mark as in-progress

### Standard phases (onboarding, design, planning, user-support)

```bash
rex-cli project update-status <item-name> in-progress
```

### Execution phase

Mark both the execution item and the specific task as in-progress:

```bash
rex-cli project update-status <execution-item-name> in-progress
rex-cli task upsert --id <task-id> --status in-progress
```

---

## Step 5: Get recent history

```bash
rex-cli history get-recent
```

Capture the output — you'll pass this context to the agent(s) so they understand what's happened recently in the project.

---

## Step 6: Prepare the agent dispatch

### Standard phases (onboarding, design, planning, user-support)

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

The prompt you give each agent must include:

1. **The project context** — pass the full project object (from step 1) so the agent knows what project it's working on, the directory, the category, etc.
2. **Recent history** — pass the recent history (from step 5) so the agent knows what's been done.
3. **The skill to use** — tell the agent to invoke the skill specified in `agent.skills`. For example: "Use the skill: rex-onboarding-goal"
4. **Input files** — list every file in `inputs` and instruct the agent to read them before starting work.
5. **Output files** — list every file in `outputs` and instruct the agent to write its results there.
6. **Effort level** — include the effort level instruction. Map the effort field to a clear instruction:
   - `"medium"` → "Apply moderate reasoning depth."
   - `"high"` → "Think carefully and thoroughly."
   - `"max"` → "Think very deeply. Take your time and consider all angles."
   - `"ultrathink"` → "Think extremely deeply. This is a critical task — exhaust every consideration before concluding."

#### Example agent prompt (standard phase)

```
You are working on the rex project "<project-title>" (id: <project-id>).
Project directory: <directory>
Category: <category> | Complexity: <complexity>
Description: <description>

Recent project history:
<paste recent history JSON>

Your task: Use the skill "rex-onboarding-goal" to complete the "goal" onboarding item.

Before you begin, read these input files:
(none for this item)

Write your output to:
- rex/<project-id>/onboarding/goal.md

Think carefully and thoroughly.
```

### Execution phase

For execution, the agent prompt is built from the **task** (from `rex-cli task next`), not the execution item. The execution item's `agent` config still controls the model, effort, and skills — but the task provides the actual work, inputs, and outputs.

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
You are working on the rex project "<project-title>" (id: <project-id>).
Project directory: <directory>
Category: <category> | Complexity: <complexity>
Description: <description>

Recent project history:
<paste recent history JSON>

## Milestone
<paste full milestone JSON>

## Objective
<paste full objective JSON>

## Your Task
<paste full task JSON>

Use the skill "rust-team-coordinator" to implement this task.

Before you begin, read these reference/input files:
- docs/auth-spec.md#token-generation

Write your output to:
- src/auth/reset_token.rs
- tests/auth/reset_token_test.rs

Think very deeply. Take your time and consider all angles.
```

---

## Step 7: Dispatch the agent(s)

### Single agent (count = 1)

Spawn one agent using the Agent tool:
- Set `model` to the value from `agent.model`
- Set `prompt` to the prepared prompt from step 6
- Set the `description` to something like "rex: <item-name>" (or "rex: <task-id>" for execution phase)
- **Do NOT set `run_in_background: true`** — the operator must block and wait for the agent to complete

### Multiple agents (count > 1)

When `agent.count` is greater than 1, spawn that many worker agents plus one coordinator agent:

1. **Worker agents** — Spawn `agent.count` agents in parallel (in a single message with multiple Agent tool calls). Each worker gets the same base prompt but with a role differentiator:
   - Worker 1: "You are worker 1 of N. Focus on the first portion of the analysis."
   - Worker 2: "You are worker 2 of N. Focus on the second portion of the analysis."
   - etc.
   
   Each worker should write its findings to a temporary intermediate file, e.g., `rex/<project-id>/<phase>/<item>-worker-<N>.md`

2. **Coordinator agent** — After all workers complete, spawn one final agent that:
   - Reads all worker output files
   - Synthesizes them into the final output file(s) specified in `outputs`
   - Cleans up the intermediate worker files

**Critical: Do not launch agents in the background and move on.** The operator must wait until all agents complete and report back. This is essential for headless operation.

---

## Step 7a: Handling user-interactive skills

Some skills (especially onboarding skills) need to ask the user questions. When a sub-agent needs user input:

1. **The sub-agent will return a question for the user.** When this happens, present the question to the user using AskUserQuestion.
2. **When the user responds**, forward their response to the sub-agent using SendMessage. **You must always include the `summary` parameter** — this is required when the message is a string. Use a brief summary of the user's response (e.g., "User describes their project goal").

Example relay flow:
```
Agent returns: "What is the project you're building?"
→ You ask the user (AskUserQuestion)
→ User responds: "I want a CLI that manages config files"
→ You forward to agent: SendMessage(to: <agent-id>, message: "I want a CLI that manages config files", summary: "User describes project as a CLI for config management")
```

Continue relaying until the sub-agent completes its work. Each SendMessage **must** include `summary` — omitting it will cause an error.

---

## Step 8: Check the agent response and mark task complete

Once the agent(s) report back, check their response text.

### Standard phases

In rare cases, an agent may instruct you **not to mark the item as complete** — for example, if it encountered an issue, needs user input that wasn't available, or wants the item to remain in-progress for a follow-up session.

- If the agent says **do not mark complete**: leave the item as `in-progress` and skip to Step 11.
- Otherwise: continue to Step 9.

### Execution phase

If the agent says **do not mark complete**: leave the task as `in-progress` and skip to Step 11.

Otherwise, **mark the task as complete**:

```bash
rex-cli task upsert --id <task-id> --status completed
```

Then check whether all tasks in the planning tree are now done:

```bash
rex-cli task next
```

- If `rex-cli task next` returns **"NO TASKS - Please mark as item complete"**: all tasks are finished. The execution phase is done — you will mark the execution item as complete in Step 10.
- If `rex-cli task next` returns **another task**: more work remains. The execution item stays `in-progress` — do NOT mark it as complete. Continue to Step 9 to record history (skipping Step 10).

---

## Step 9: Record history

Insert a history entry for what was just done:

### Standard phases

```bash
rex-cli history insert-recent \
  --id "session-<item-name>-<timestamp-short>" \
  --timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --summary "<brief description of what the agent accomplished>" \
  --entity <item-name> \
  --file <each-output-file>
```

### Execution phase

```bash
rex-cli history insert-recent \
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
rex-cli project update-status <item-name> completed
```

### Execution phase

Only mark the execution item as complete if Step 8 confirmed that **all tasks are done** (i.e., `rex-cli task next` returned "NO TASKS").

```bash
rex-cli project update-status <execution-item-name> completed
```

If tasks remain, **skip this step entirely** — the execution item stays as `in-progress`. It will be picked up again on the next operator invocation, which will run `rex-cli task next` to find the next task.

---

## Step 11: Manage history

Dispatch an agent with the `rex-manage-history` skill to condense/archive history if the recent section has grown beyond 3 entries:

```
Use the skill "rex-manage-history" to check and condense the project history for project "<project-id>".
```

Use the same model as the work item's agent, or default to `sonnet` (history management is lightweight). **Wait for this agent to complete before proceeding.**

---

## Step 12: Stop

The operator's job for this invocation is done. Report what was processed and stop.

### Standard phases

```
Operator complete.
- Item: <item-name> (<phase>)
- Status: completed | left in-progress
- Output(s): <list of output files>
```

### Execution phase

```
Operator complete.
- Phase: execution
- Task: <task-id> — <task-title>
- Task status: completed | left in-progress
- Objective: <objective-id> — <objective-title>
- Milestone: <milestone-id> — <milestone-title>
- Execution item status: completed (all tasks done) | in-progress (tasks remaining)
- Output(s): <list of task output files>
```

---

## CLI Quick Reference

These are the **exact** commands available. Do not invent commands that aren't listed here.

```
# Project
rex-cli project get-active
rex-cli project next-item
rex-cli project create
rex-cli project remove <id>
rex-cli project activate <id>
rex-cli project update-status <item> <status>
rex-cli project update-title "<title>"
rex-cli project update-subtitle "<subtitle>"
rex-cli project update-description "<description>"
rex-cli project update-directory "<path>"

# Planning
rex-cli milestone upsert --id <id> [--title --description --status] [--add-reference ...] [--add-upstream ...]
rex-cli milestone get <id>
rex-cli milestone list [--status <status>]
rex-cli milestone remove <id>

rex-cli objective upsert --id <id> [--milestone --title --description --status] [--add-reference ...] [--add-upstream ...]
rex-cli objective get <id>
rex-cli objective list [--milestone <id>] [--status <status>]
rex-cli objective remove <id>

rex-cli task upsert --id <id> [--objective --title --description --status] [--add-reference ...] [--add-upstream ...]
rex-cli task get <id>
rex-cli task list [--objective <id>] [--status <status>]
rex-cli task remove <id>
rex-cli task next

# History
rex-cli history list
rex-cli history get-recent
rex-cli history insert-recent --id <id> --timestamp <iso8601> --summary "<text>" [--entity ...] [--file ...] [--session <id>]
rex-cli history remove-from-recent <id>
rex-cli history insert-compacted --id <id> --timestamp <iso8601> --summary "<text>" [--entity ...] [--file ...] [--session <id>]
rex-cli history remove-from-compacted <id>

# Checklist
rex-cli checklist init [--date <YYYY-MM-DD>]
rex-cli checklist add --category <cat> --id <id> --title "<title>" --description "<desc>" [--phase <phase>]
rex-cli checklist list [--category <cat>] [--phase <phase>] [--complete] [--incomplete]
rex-cli checklist get <id>
rex-cli checklist update <id> [--title --description --phase]
rex-cli checklist complete <id>
rex-cli checklist uncomplete <id>
rex-cli checklist remove <id>
rex-cli checklist set-context "<text>"
```

**Important:** There is no `rex-cli project update` command. Title, subtitle, and description must be updated with separate commands (`update-title`, `update-subtitle`, `update-description`).

---

## Rules

- **One item per invocation.** The operator processes exactly one work item (or one task within execution) and stops. It does not loop.
- **CLI only for data management.** All reads and writes to rex data structures go through `rex-cli` commands. Never write `project-status.json`, `history.json`, or `planning.json` directly.
- **Blocking dispatch.** Never launch agents in the background. Always wait for completion. This is critical for headless/automated operation.
- **Respect the lock.** If the project is locked, stop immediately. No exceptions, no "let me just check one thing."
- **Respect agent responses.** If an agent says not to mark complete, don't mark complete.
- **Pass full context.** Agents should receive the project object and recent history so they have everything they need.
- **Execution uses `rex-cli task next`, not the item itself.** During execution, the planning tree drives what gets worked on. The execution item's `agent` config provides the model/effort/skills, but the task provides the work, inputs, and outputs.
- **Mark tasks complete via CLI.** Always run `rex-cli task upsert --id <id> --status completed` when a task is done. Only mark the execution item complete when all tasks are finished.
