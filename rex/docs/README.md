# Rex: Agent-Orchestrated Project Harness

Rex is a CLI-driven harness that orchestrates LLM agents through a structured software development lifecycle. It guides projects from initial idea through onboarding, design, planning, and execution — with the **operator** acting as the central heartbeat that dispatches agents, tracks progress, and records history.

## How It Works

Rex manages a **project registry** (`rex/projects.json`) with one active project at a time. Each project progresses through ordered phases, with every work item tracked in a `project-status.json` file. The operator reads the next incomplete item, dispatches the appropriate agent(s) with the right skill/model/effort, waits for completion, records what happened, and stops. One item per invocation.

```
User creates project
        |
        v
   +---------+     +--------+     +-----------+     +-----------+
   |Onboarding| --> | Design | --> | Planning  | --> | Execution |
   | 14 items |     | 9 items|     |  3 items  |     |  1 item   |
   +---------+     +--------+     +-----------+     +-----------+
        |               |               |                  |
        v               v               v                  v
   .md files      design docs      planning.json     completed code
   in onboarding/ in design/       milestones/       in project dir
                                   objectives/       (driven by
                                   tasks             rex task next)
```

All phases live in a single `project-status.json` file. The operator calls `rex project next-item` to get the next incomplete item across all phases. For phases 1-3, the item itself describes the work. For phase 4, the single execution item tells the operator to switch to `rex task next` and work through the planning tree.

## Initialization

Before creating projects, initialize the rex harness in your repository:

```bash
rex init [--claude | --cursor]
```

This copies all skills, hooks, settings, and documentation into the current directory. Prompts for the agent OS (Claude Code or Cursor) to determine the config directory:

| Agent OS | Config dir | Root file |
|----------|-----------|-----------|
| Claude Code | `.claude/` | `CLAUDE.md` |
| Cursor | `.cursor/` | `AGENTS.md` |

**What gets created:**
- `<config-dir>/skills/` — all rex and rust skills (40 skill directories)
- `<config-dir>/hooks/commit-and-push.sh` — auto-commit on agent stop
- `<config-dir>/settings.json` — hook configuration
- `rex/docs/` — all CLI and process documentation
- `rex/projects.json` — empty project registry
- `CLAUDE.md` or `AGENTS.md` — points to `rex/docs/README.md`

**Safe to re-run:** existing files are never overwritten. Only missing files/folders are created. If `CLAUDE.md`/`AGENTS.md` already exists, the rex section is appended. If `settings.json` already exists, rex hooks are merged in.

## Phase Overview

### Phase 0: Project Creation

```bash
rex project create
```

Interactive command that scaffolds a new project. Prompts for:
- **Project ID** (kebab-case identifier)
- **Complexity** (small / medium / large)
- **Title, subtitle, description** (can be deferred)
- **Directory** (auto-detects matching directories)
- **Category** (library / binary / refactor)
- **Design selections** (which optional design/onboarding items to include)

Creates:
- `rex/<project-id>/` with subdirectories: `onboarding/`, `design/`, `planning/`, `execution/`, `uat/`, `user-support/`
- `rex/<project-id>/project-status.json` — the ordered work item manifest
- Scaffolds project code via `cargo new` if needed

### Phase 1: Onboarding (14 items)

Gathers everything the agents need to know before designing the system. Each item produces a markdown file in `rex/<project-id>/onboarding/`.

| # | Item | Skill | Model | Effort | Required | Purpose |
|---|------|-------|-------|--------|----------|---------|
| 1 | `goal` | `rex-onboarding-goal` | opus | high | always | Define what, who, and why |
| 2 | `scope` | `rex-onboarding-scope` | opus | high | always | In-scope, out-of-scope, deferred |
| 3 | `existing-code` | `rex-onboarding-existing-code` | sonnet | medium | optional | What code exists already |
| 4 | `libraries-and-sdks` | `rex-onboarding-libraries-and-sdks` | sonnet | medium | optional | Preferred crates and dependencies |
| 5 | `research` | `rex-onboarding-research` | sonnet | medium | optional | Topics needing investigation |
| 6 | `resources` | `rex-onboarding-resources` | sonnet | medium | optional | MCPs, CLI tools, reference codebases |
| 7 | `user-expertise` | `rex-onboarding-user-expertise` | opus | high | optional | User's domain knowledge and instincts |
| 8 | `success-measures` | `rex-onboarding-success-measures` | opus | high | optional | Concrete pass/fail criteria |
| 9 | `known-risks` | `rex-onboarding-known-risks` | opus | high | optional | Project risks + agent-driven risks |
| 10 | `uat` | `rex-onboarding-uat` | opus | high | always | What the user expects to test |
| 11 | `environment-variables` | `rex-onboarding-environment-variables` | opus | high | optional | Secrets, keys, config (never values) |
| 12 | `idea-generation` | `rex-onboarding-idea-generation` | opus | ultrathink | optional | Non-obvious improvements from all context |
| 13 | `skill-building` | `rex-onboarding-skill-building` | opus | ultrathink | optional | Create specialist agent skills |
| 14 | `checklist` | `rex-onboarding-checklist` | opus | ultrathink | always | Synthesize all inputs into design/planning checklist |

**Input chaining:** Each onboarding item receives all preceding onboarding outputs as inputs, so later items have full context.

**Stop-on-finish:** Only the final `checklist` item stops the operator. Items 1-13 flow continuously.

**Output:** `checklist.json` with items categorized as design-must-haves, architecture-constraints, planning-milestones, objectives, tasks-to-plan-for, research-and-prototyping, risk-mitigations, and out-of-scope — each tagged with a phase (design or planning).

### Phase 2: Design (9 items)

Creates the complete system blueprint. Each item produces documents in `rex/<project-id>/design/`. All design items have `stop-on-finish: true`.

| # | Item | Skill | Agents | Model | Effort | Required | Purpose |
|---|------|-------|--------|-------|--------|----------|---------|
| 1 | `existing-code-exploration` | `rex-design-rust-existing-code-exploration` | 3 | opus | high | refactor only | Forensic analysis of existing codebase |
| 2 | `library-review` | `rex-design-rust-library-review` | 1 | opus | high | optional | Version check + API docs for unfamiliar crates |
| 3 | `module-design` | `rex-design-rust-modules` | 1 | opus | max | always | File/folder layout (500-line rule, domain-first) |
| 4 | `architecture-design` | `rex-design-rust-architecture` | 1 | opus | max | always | Types, traits, enums, function signatures |
| 5 | `integration-testing` | `rex-design-rust-integration-tests` | 1 | opus | max | optional | Failure mode analysis, real-world test strategy |
| 6 | `foreign-critique` | `rex-design-foreign-critique` | 3 | opus | max | optional | Adversarial cross-document consistency review |
| 7 | `error-handling` | `rex-design-rust-errors` | 1 | sonnet | high | always | Error types, propagation, thiserror strategy |
| 8 | `architecture-proposal` | `rex-design-rust-architecture-proposal` | 1 | opus | max | always | Synthesize all design into polished .md + .html |
| 9 | `user-acceptance` | `rex-design-user-acceptance` | 1 | opus | max | optional | Human gate — user reviews and approves design |

**Multi-agent items:** `existing-code-exploration` and `foreign-critique` dispatch 3 worker agents in parallel, then a coordinator synthesizes their findings.

**Input dependencies:** Design items receive targeted subsets of onboarding outputs plus earlier design outputs. For example, `architecture-design` receives the module design, library review, existing code exploration, plus key onboarding docs.

**Quality gates:**
- `foreign-critique` — adversarial review checking cross-document consistency (can directly edit module and architecture docs)
- `user-acceptance` — human approval gate (blocks until user explicitly accepts)

### Phase 3: Planning (3 items)

Breaks the design into an executable work tree stored in `rex/<project-id>/planning/planning.json`. All items use opus at max effort with `stop-on-finish: true`.

| # | Item | Skill | Purpose |
|---|------|-------|---------|
| 1 | `milestones` | `rex-planning-milestones` | Major checkpoints (1-3 per topic, review milestones after heavy ones) |
| 2 | `objectives` | `rex-planning-objectives` | Strategic outcomes per milestone (1-3 per milestone) |
| 3 | `tasks` | `rex-planning-tasks` | Atomic work units per objective (1-3 per objective, single-session) |

**Hard constraint:** 1-3 items at each level. If more are needed, the parent level must be split.

**Hierarchy:** `Milestone -> Objective -> Task`, with explicit upstream/downstream dependencies at every level. Dependencies are bidirectional — adding an upstream automatically registers the downstream.

**Planning inputs:** Full onboarding context + all design documents + planning.json itself (for building incrementally).

### Phase 4: Execution (1 item)

After planning completes, `project-status.json` contains a single execution item:

```json
{
  "item": "run",
  "stop-on-finish": false,
  "agent": {
    "count": 1,
    "effort": "max",
    "model": "opus",
    "skills": ["rust-team-coordinator"]
  },
  "inputs": [],
  "outputs": [],
  "status": "not-started"
}
```

This item is intentionally minimal — inputs and outputs are empty because they come from the planning tree at runtime. When the operator encounters this item (phase = `"execution"`), it switches from the linear `project-status.json` sequence to the **planning tree**:

1. Calls `rex task next` to find the highest-priority eligible task from `planning.json`
2. Gets back the task + its parent objective + its parent milestone as full JSON
3. Dispatches an agent with `rust-team-coordinator` skill + that full planning context
4. Marks the task completed when done
5. Checks if more tasks remain — if so, the execution item stays `in-progress`
6. Only marks the execution item `completed` when ALL tasks are finished

The execution item persists across many operator invocations. Each invocation processes one task, records history, and stops. The next invocation picks up the next task via `rex task next` again.

**`stop-on-finish: false`** — a wrapping loop can continuously invoke the operator to process tasks without pausing between them.

**Task priority scoring** (lower tier = higher priority):

| Tier | Condition |
|------|-----------|
| 0 | Task is already in-progress (resume unfinished work) |
| 1 | Task in an in-progress objective within an in-progress milestone |
| 2 | Task in a not-started objective within an in-progress milestone |
| 3 | Task in an in-progress objective within a not-started milestone |
| 4 | Everything else |

Within tiers, tasks are ranked by transitive downstream impact (more dependents = higher priority), then by array position.

**Eligibility rules:** A task is eligible only if all its upstream tasks are completed, its parent objective is not blocked with all objective-level upstreams completed, and its parent milestone is not blocked with all milestone-level upstreams completed.

## The Operator

The operator (`rex-operator` skill) is the heartbeat. It processes exactly one work item per invocation, then stops. The user (or a loop) invokes it repeatedly to drive the project forward.

### Standard Phase Sequence (Onboarding, Design, Planning)

```
1. rex project get-active          -> Get project, check it exists
2. Check lock status               -> If locked: STOP
3. rex project next-item           -> Get next incomplete item from project-status.json
4. rex project update-status       -> Mark item in-progress
5. rex history get-recent          -> Get recent history for agent context
6. Build agent prompt              -> From item config (skill, inputs, outputs, effort, model)
7. Dispatch agent(s)               -> BLOCKING (never background)
8. Check agent response            -> Respect "do not mark complete" signals
9. rex history insert-recent       -> Record what was done
10. rex project update-status      -> Mark item completed
11. Dispatch rex-manage-history    -> Keep recent history at 3 entries max
12. Stop and report
```

### Execution Phase Sequence

```
1. rex project get-active          -> Get project
2. Check lock status               -> If locked: STOP
3. rex project next-item           -> Get next item (execution phase)
3a. rex task next                  -> Resolve actual task from planning tree
    If "NO TASKS" -> mark execution item complete, skip to step 9
4. rex project update-status       -> Mark execution item in-progress
   rex task upsert --status        -> Mark task in-progress
5. rex history get-recent          -> Context for agent
6. Build prompt from task context  -> Task + objective + milestone + project + history
7. Dispatch agent(s)               -> BLOCKING
8. rex task upsert --status        -> Mark task completed
   rex task next                   -> Check if more tasks remain
   If NO TASKS -> mark execution item completed
   If tasks remain -> execution item stays in-progress
9. rex history insert-recent       -> Record task/objective/milestone entities
10. rex project update-status      -> Mark execution complete (only if ALL tasks done)
11. Dispatch rex-manage-history    -> Archive old history
12. Stop and report
```

### Agent Dispatch

Each work item in `project-status.json` specifies how its agent(s) should be dispatched:

```json
{
  "item": "architecture-design",
  "stop-on-finish": true,
  "agent": {
    "count": 1,
    "effort": "max",
    "model": "opus",
    "skills": ["rex-design-rust-architecture"]
  },
  "inputs": ["rex/<id>/onboarding/goal.md", "..."],
  "outputs": ["rex/<id>/design/architecture-design.md"],
  "status": "not-started"
}
```

| Field | Purpose |
|-------|---------|
| `count` | Number of agents (1 = single, N = N workers + 1 coordinator) |
| `effort` | Reasoning depth: medium, high, max, ultrathink |
| `model` | LLM model: opus, sonnet, haiku |
| `skills` | Skill(s) to invoke |
| `inputs` | Files the agent should read |
| `outputs` | Files the agent should produce |
| `stop-on-finish` | Whether the operator stops after this item completes |

## CLI Command Reference

### Initialization

| Command | Purpose |
|---------|---------|
| `rex init` | Initialize the rex harness in the current directory (interactive) |
| `rex init --claude` | Initialize for Claude Code (non-interactive) |
| `rex init --cursor` | Initialize for Cursor (non-interactive) |

### Project Management

| Command | Purpose |
|---------|---------|
| `rex project create` | Interactive project creation with scaffolding |
| `rex project get-active` | Display the currently active project |
| `rex project activate <id>` | Switch to a different project |
| `rex project remove <id>` | Remove a project (optionally delete source) |
| `rex project update-directory <dir>` | Change project source directory |
| `rex project update-title <title>` | Update project title |
| `rex project update-subtitle <subtitle>` | Update project subtitle |
| `rex project update-description <desc>` | Update project description |
| `rex project update-status <item> <status>` | Update item status (not-started / in-progress / completed / not-required) |
| `rex project next-item` | Get next incomplete item as JSON |

### Checklist

| Command | Purpose |
|---------|---------|
| `rex checklist init` | Initialize empty checklist |
| `rex checklist add --category <cat> --id <id> --title <t> --description <d> --phase <p>` | Add item |
| `rex checklist list [--category <c>] [--phase <p>] [--complete] [--incomplete]` | List with filters |
| `rex checklist get <id>` | Get specific item |
| `rex checklist update <id> [--title <t>] [--description <d>] [--phase <p>]` | Update item fields |
| `rex checklist complete <id>` | Mark done |
| `rex checklist uncomplete <id>` | Mark not done |
| `rex checklist remove <id>` | Delete item |
| `rex checklist set-context <context>` | Set checklist context text |

### Planning Tree (Milestones / Objectives / Tasks)

All three levels share the same command pattern and list modification flags:

| Command | Purpose |
|---------|---------|
| `rex milestone upsert --id <id> [--title <t>] [--description <d>] [--status <s>]` | Create or update milestone |
| `rex milestone get <id>` | Get milestone as JSON |
| `rex milestone list [--status <s>]` | List milestones |
| `rex milestone remove <id>` | Remove milestone |
| `rex objective upsert --id <id> --milestone <m> [--title <t>] [--description <d>]` | Create or update objective under milestone |
| `rex objective get <id>` | Get objective as JSON |
| `rex objective list [--milestone <m>] [--status <s>]` | List objectives |
| `rex objective remove <id>` | Remove objective |
| `rex task upsert --id <id> --objective <o> [--title <t>] [--description <d>]` | Create or update task under objective |
| `rex task get <id>` | Get task as JSON |
| `rex task list [--objective <o>] [--status <s>]` | List tasks |
| `rex task remove <id>` | Remove task |
| `rex task next` | Get highest-priority eligible task + objective + milestone |

**Shared list modification flags** (available on all `upsert` commands):
- `--add-reference <path>` / `--remove-reference <path>` — file paths, URLs, entity IDs
- `--add-output <path>` / `--remove-output <path>` — artifact paths
- `--add-upstream <id>` / `--remove-upstream <id>` — dependency (auto-maintains bidirectional)
- `--add-downstream <id>` / `--remove-downstream <id>` — reverse dependency
- `--add-checklist <ID:TEXT>` / `--remove-checklist <id>` — verification items
- `--check <id>` / `--uncheck <id>` — toggle checklist items

### History

| Command | Purpose |
|---------|---------|
| `rex history insert-recent --id <id> --timestamp <ts> --summary <s> [--entity <e>]... [--file <f>]...` | Add to recent history |
| `rex history remove-from-recent <id>` | Remove from recent |
| `rex history insert-compacted --id <id> --timestamp <ts> --summary <s> [--entity <e>]...` | Add to archived history |
| `rex history remove-from-compacted <id>` | Remove from archived |
| `rex history get-recent` | Get recent entries as JSON |
| `rex history list` | Get all history (recent + archived) as JSON |

## project-status.json Structure

The file contains 5 phase keys, processed in order by `rex project next-item`:

```json
{
  "user_support": [ ... ],    // 1 item (pre-completed on create)
  "onboarding":   [ ... ],    // 14 items
  "design":       [ ... ],    // 9 items
  "planning":     [ ... ],    // 3 items
  "execution":    [ ... ]     // 1 item (bridges to planning tree via rex task next)
}
```

The operator walks this list linearly. Each item has a status (`not-started`, `in-progress`, `completed`, `not-required`). Items marked `not-required` are skipped. When the operator reaches the execution item, it switches from linear progression to the planning tree — calling `rex task next` repeatedly until all tasks are done.

## Data Storage

```
rex/
  projects.json                          # Project registry (active + inactive)
  <project-id>/
    project-status.json                  # 5-phase work item manifest
    onboarding/
      goal.md                            # Project goal
      scope.md                           # Boundaries
      checklist.json                     # Synthesized design/planning checklist
      ...                                # Other onboarding outputs
    design/
      module-design.md                   # File/folder layout
      architecture-design.md             # Type architecture
      error-handling.md                  # Error strategy
      architecture-proposal.md           # Final synthesized proposal
      architecture-proposal.html         # HTML viewer with diagrams
      ...                                # Other design outputs
    planning/
      planning.json                      # Milestones, objectives, tasks
    execution/                           # Execution-phase artifacts
    uat/                                 # User acceptance testing artifacts
    user-support/                        # User input/output
    history.json                         # Recent (3 max) + archived work entries
```

## Required vs Optional Items

**Always required (onboarding):** goal, scope, uat, checklist

**Always required (design):** module-design, architecture-design, error-handling, architecture-proposal

**Conditionally required:** existing-code-exploration (required for refactor category projects)

**All other items** are optional — the user selects which to include during `rex project create`. Unselected items are marked `not-required` and skipped by the operator.

## Key Design Decisions

- **CLI-only mutations** — agents never write JSON files directly; all state changes go through `rex` commands
- **One item per operator invocation** — prevents runaway execution; the user or a loop controls pacing
- **Blocking dispatch** — the operator always waits for agents to finish; never runs them in background
- **Bidirectional dependencies** — adding an upstream automatically creates the downstream link
- **Stop-on-finish** — items marked with this flag cause the operator to stop even if more items follow (all design and planning items have this)
- **History rotation** — recent history is capped at 3 entries; older entries are summarized and archived
- **Lock mechanism** — a locked project causes the operator to stop immediately
- **Agent "do not complete" signals** — agents can prevent the operator from marking an item complete (used by user-acceptance and other interactive steps)
