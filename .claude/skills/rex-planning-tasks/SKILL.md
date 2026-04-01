---
name: rex-planning-tasks
description: Plan the tasks for each objective during the rex planning phase — designing the atomic, actionable work items that agents will execute, with precise upstream/downstream dependency management. Use this skill when the rex planning process reaches the "tasks" step, when objectives exist and need their tasks defined, or when the user says things like "plan the tasks", "break down the objectives", "what work needs doing", "define the tasks", "create the task breakdown", or "what should agents actually build." This skill reads existing milestones/objectives and all design inputs, then produces tasks using the rex CLI with careful attention to dependency chains. Maximum 1-3 tasks per objective.
disable-model-invocation: false
user-invocable: false
---

# Planning: Tasks

You plan the tasks for each objective — the atomic units of work that agents will pick up and execute. Tasks answer "what specific thing needs to be done next?" Each task should be completable in a single work session, have a clear definition of done, and require no further decomposition.

Tasks are the leaves of the planning tree. Everything above them (milestones, objectives) describes outcomes and states. Tasks describe actions — concrete, assignable work with a beginning and an end. "Implement the token-generation endpoint for password reset" is a task. "Users can securely reset their passwords" is an objective. Don't confuse the two.

You'll be told where to write any output files and given input files to read for context — including the milestones and objectives that already exist. Read everything first. Then design the tasks with obsessive attention to dependency ordering, and write them using the `rex-cli task upsert` CLI.

---

## Your mindset

**Atomic and actionable.** A task should be something an agent can pick up, understand in isolation (with its references), and complete without needing to make strategic decisions. The strategic decisions were made at the objective and milestone levels. Tasks are execution.

**1 to 3 tasks per objective. No more.** This is a hard constraint. Agents love to generate exhaustive task lists — 8 tasks for something that genuinely needs 2. This creates busywork, inflates the plan, and produces tasks that don't add value. If you can't capture what an objective needs in 3 tasks, the objective is too broad and must be split. The discipline forces you to think about what work genuinely matters versus what's noise.

**Upstream and downstream are everything.** This is the most critical aspect of task planning. A task's upstream dependencies tell an agent "don't start until these are done." Its downstream dependencies tell the system "these are blocked until this completes." Get the dependencies wrong and agents will either: (a) start work that fails because a prerequisite doesn't exist, or (b) sit idle waiting for something that's already done but not wired correctly.

Every task must have its upstream and downstream dependencies explicitly set. No exceptions. Even if a task has no dependencies (it's a starting task), state that explicitly with an empty upstream. An agent should never have to guess what it can start.

**Designed for handoff.** The agent executing a task is not you. It has no memory of this planning conversation. It will read the task's title, description, checklist, references, and dependencies — and that's all it knows. If those fields don't contain enough information for cold-start execution, the task is underspecified.

---

## Reading the inputs

### The objectives (critical input)

The existing objectives are your primary input. For each objective, understand:
- **What outcome it describes** — The title and description define the target state
- **Its checklist** — Success criteria that tasks must collectively satisfy
- **Its parent milestone** — The broader context this objective sits within
- **Its dependencies** — Which objectives must complete first, which come after
- **Its references** — Design docs and specs that inform what tasks are needed

Read the objectives using:
```bash
rex-cli objective list
rex-cli objective get <objective-id>
```

Also read the milestones for broader context:
```bash
rex-cli milestone list
rex-cli milestone get <milestone-id>
```

### From design documents

- **Architecture** — Type definitions, function signatures, and data flow tell you exactly what code needs to be written. Each significant struct, trait, or function cluster may map to a task.
- **Modules** — The file/folder plan tells you where code goes. Tasks should align with module boundaries — a task that touches 5 modules is too broad.
- **Error handling** — Error types and propagation patterns are often their own task within an objective.
- **Library review** — Integration with unfamiliar crates may need a dedicated task if the integration is non-trivial.
- **Integration tests** — Test tasks should reference the test plan so the agent knows exactly what tests to write.

### From onboarding documents

- **Goal and scope** — Keep tasks grounded; don't create tasks for out-of-scope work
- **Success measures** — These inform task-level checklists (the concrete verification points)
- **Existing code** — If code already exists, tasks may involve modification rather than creation

---

## The task planning process

### Step 1: Understand each objective's requirements

For each objective, answer:
- What concrete work must happen for this outcome to become true?
- What artifacts (files, tests, configs) must exist when this objective is complete?
- What's the natural order of that work?

Work from the objective's description, checklist, and referenced design documents. The design documents are particularly important — they contain the specific structs, functions, and module assignments that tell you exactly what to build.

### Step 2: Identify the atomic work items

For each objective, list the concrete actions needed. Then compress ruthlessly into 1-3 tasks.

**How to compress:** If you have 5 candidate tasks, ask which ones are genuinely independent pieces of work versus which ones are just steps within a single coherent task. "Define the struct," "implement the methods," and "add the derives" are not three tasks — they're one task: "Implement the Order type with its methods and derives." An agent writing the struct will naturally add the derives and methods in the same session.

Common task patterns:
- **Implement core types and logic** — The central code for an objective (structs, functions, core behavior)
- **Implement integration/wiring** — Connecting the core logic to other parts of the system (routes, CLI handlers, database calls)
- **Implement tests** — Unit tests, integration tests, or both for the objective's scope

Not every objective needs all three. Some objectives are purely about wiring (1 task). Some are about complex logic (2 tasks: logic + tests). Match the tasks to the actual work, not to a template.

### Step 3: Size-check each task

**Too small (merge into another task):**
- "Add `#[derive(Debug)]` to the Config struct" — that's a line of code, not a task
- "Create the `models/` directory" — that happens as part of writing the first model
- "Import the serde crate" — that's a dependency of implementing serialization, not a standalone task

**Too big (split or push up to objective):**
- "Implement the entire WebSocket server" — if this involves connection handling, message parsing, state management, and reconnection logic, it's an objective
- "Write all tests for the matching engine" — if the matching engine has 3 objectives, each with its own tests, this doesn't belong in any single objective

**Right-sized (a task):**
- "Implement the OrderBook struct with insert, cancel, and best-price methods"
- "Wire the /orders endpoint to the matching engine with request validation"
- "Write integration tests for order submission through the REST API"

Each is completable in one session, produces clear artifacts, and an agent can understand exactly what to do.

### Step 4: Design the dependency graph

This is the most important step. Every task must have its upstream and downstream dependencies explicitly defined.

**Within an objective:**
Tasks within the same objective often depend on each other:
- Implementation before tests (you can't test what doesn't exist)
- Core types before logic that uses them (if split into separate tasks)
- Wiring depends on both the thing being wired and the thing it wires into

**Across objectives within the same milestone:**
If task T2 in objective B needs something produced by task T1 in objective A, add T1 as an upstream of T2. This is the most common source of dependency bugs — tasks that implicitly assume another objective's work is done without declaring the dependency.

**Across milestones:**
Cross-milestone task dependencies should be rare (milestone-level dependencies usually handle this). But if a specific task in milestone 2 depends on a specific task's output from milestone 1, wire it explicitly.

**How to verify your dependency graph:**

1. **The cold-start test.** For each task with no upstream dependencies (a "root" task), ask: "Can an agent start this right now with nothing else completed?" If not, you're missing an upstream.

2. **The completion cascade test.** Walk through tasks in dependency order. After completing each task, check: "Are any downstream tasks now unblocked that shouldn't be? Are any tasks still blocked that should now be free?" Both indicate wiring errors.

3. **The parallel opportunity test.** Look for tasks that have no dependency relationship to each other. These can run in parallel. If everything is serial, ask whether some dependencies are artificial — is task B genuinely blocked by task A, or could they run concurrently?

4. **The orphan test.** Every task should be reachable from a root task via the dependency chain. Orphaned tasks (no upstream, no downstream, disconnected from everything) are a red flag — they're either unnecessary or missing their dependencies.

### Step 5: Define clear completion criteria

Each task gets a checklist — the specific, binary items that prove the task is done. These should be concrete enough that an agent (or a reviewer) can verify them without judgment calls.

Good checklist items:
- "OrderBook::insert handles duplicate order IDs by returning an error"
- "Endpoint returns 400 with a descriptive message for malformed JSON"
- "Integration test covers the happy path: submit order → match → fill notification"

Bad checklist items:
- "Code is well-structured" — subjective
- "Implementation complete" — circular
- "Handles edge cases" — which ones?

### Step 6: Enforce the 1-3 constraint — or escalate

After planning, count the tasks per objective. If any objective has more than 3 tasks, the objective is too broad.

**Do not drop tasks to fit.** If you genuinely need 5 tasks for an objective, those tasks represent real work. The objective must be split.

**The escalation procedure:**

1. Identify which objective has too many tasks
2. Determine how to split it into 2 objectives, each with 1-3 tasks, that preserve the original intent
3. Spawn subagents to perform the restructuring:
   - One agent to split the objective using `rex-cli objective upsert` (creating the new objective, updating the old one, ensuring both are parented to the same milestone)
   - One agent to rewire all upstream/downstream dependencies — every objective and task that pointed to the old objective as upstream or downstream must be checked and updated. The two new objectives must be correctly sequenced relative to each other and to sibling objectives
   - After both complete, verify the dependency graph is intact: no orphaned references, no broken chains, no circular dependencies
4. Verify the parent milestone still has at most 3 objectives. If the split pushed the milestone over 3 objectives, escalate further — the milestone itself must be split. Spawn agents to split the milestone and rewire its upstream/downstream dependencies across the milestone graph
5. Re-evaluate the tasks for the now-smaller objectives — they should fit within 1-3 each

This cascade (task overflow → split objective → possible milestone split) is rare but essential. The constraint exists because plans that balloon in size produce more work than they're worth. Smaller, focused units keep agents productive.

---

## Writing the tasks using the CLI

Once you've planned all tasks (1-3 per objective, no exceptions), write them using the rex CLI. **Do not write planning.json directly.**

### Task creation

```bash
rex-cli task upsert \
  --id t-<objective-topic>-<action> \
  --objective o-<parent-objective-id> \
  --title "Clear, actionable description of the work" \
  --description "What to build, where it goes, what it integrates with. Include enough context for an agent to start cold." \
  --add-reference design/architecture.md \
  --add-reference design/modules.md \
  --add-output src/module/file.rs \
  --add-output tests/integration/test_file.rs \
  --add-checklist "c1:First completion criterion" \
  --add-checklist "c2:Second completion criterion" \
  --add-upstream t-<dependency-task-id> \
  --add-downstream t-<dependent-task-id>
```

The parent objective's `tasks` list is automatically updated.

### ID conventions

Use consistent, readable IDs that encode the hierarchy:
- `t-<objective-topic>-<action>` (e.g., `t-matching-impl`, `t-matching-tests`, `t-api-routes-wiring`)
- The objective topic prefix groups related tasks visually
- The action suffix describes what the task does

### Dependency wiring

Wire dependencies as you create tasks. Create upstream tasks first so downstream tasks can reference them.

```bash
# Task with upstream dependency
rex-cli task upsert \
  --id t-matching-tests \
  --objective o-core-matching \
  --title "Write integration tests for order matching" \
  --description "..." \
  --add-upstream t-matching-impl

# Cross-objective dependency
rex-cli task upsert \
  --id t-api-routes \
  --objective o-api-layer \
  --title "Wire REST endpoints to matching engine" \
  --description "..." \
  --add-upstream t-matching-impl \
  --add-upstream t-api-types
```

### References and outputs

- **References** — Point to the specific design documents an agent needs to execute this task. Don't just reference the top-level architecture doc — point to the specific section or module plan. An agent should be able to read the references and know exactly what structs to create, what function signatures to use, and where the code goes.
- **Outputs** — List the specific files this task produces. This helps downstream tasks know exactly what to expect and where to find it.

---

## Writing output files

If you were given an output file path, write a summary document:

```markdown
# Task Plan

**Date:** YYYY-MM-DD

## Overview
Brief description of the task structure and dependency strategy.

## Tasks by Objective

### Objective: [Title] (`o-id`) — Milestone: [Title] (`m-id`)

#### 1. [Task Title] (`t-id`)
**Description:** What this task involves
**Upstream:** t-dependency-1, t-dependency-2 (or "none — root task")
**Downstream:** t-dependent-1
**Outputs:** src/module/file.rs
**Checklist:**
- [ ] Completion criterion 1
- [ ] Completion criterion 2

(Repeat for each task)

(Repeat for each objective)

## Dependency Graph
A textual or mermaid representation of the full task dependency chain, showing which tasks can run in parallel and which are serial.

## Execution Order
The recommended order for agents to pick up tasks, based on the dependency graph. Group tasks that can run in parallel.

## Key Decisions
Why tasks were scoped this way, notable dependency choices, and any escalation decisions that were made.
```

---

## What done looks like

You're done when:
1. Every objective has 1-3 tasks (no exceptions — if any objective needed more, it was split first, and any resulting milestone splits were handled too)
2. All tasks have been created via the CLI using `rex-cli task upsert`
3. Every task has its upstream and downstream dependencies explicitly wired — no implicit dependencies
4. Each task has a meaningful checklist with concrete, verifiable completion criteria
5. Each task's references point to the specific design documents an agent needs for cold-start execution
6. Each task's outputs list the specific files it will produce
7. The dependency graph has been verified: no orphans, no broken chains, no missing upstreams
8. Any requested output files have been written
