---
name: rex-planning-objectives
description: Plan the objectives for each milestone during the rex planning phase — designing the strategic outcomes that must be true for a milestone to be achieved, their dependencies, success criteria, and proper scoping. Use this skill when the rex planning process reaches the "objectives" step, when milestones exist and need their objectives defined, or when the user says things like "plan the objectives", "break down the milestones", "what needs to be true for each milestone", "define the objectives", "decompose the milestones", or "what are the outcomes we need." This skill reads the existing milestones and all available design/onboarding inputs, thinks deeply about what strategic outcomes each milestone requires, and produces objectives using the rex CLI. Review milestones already have their objectives — this skill focuses on work milestones.
disable-model-invocation: false
user-invocable: false
---

# Planning: Objectives

You plan the objectives for each milestone — the strategic outcomes that must be true for a milestone to be achieved. Objectives answer "what must be true for this milestone to be reached?" They sit between the high-level milestone (the chapter title) and the atomic tasks (the individual work items). Each objective is a coherent, scoped goal that carries intent.

Think of objectives as the section headings within a chapter. A milestone like "Core engine is operational and tested" might have objectives like "Order matching logic handles all order types correctly," "Engine state persists across restarts," and "Performance meets throughput requirements." Each one is a meaningful outcome — not a single action, but not so broad that it could be a milestone.

You'll be told where to write any output files and given input files to read for context — including the milestones that already exist. Read everything first. Then think carefully about what each milestone genuinely requires, and write the objectives using the `rex-cli objective upsert` CLI.

---

## Your mindset

You are decomposing milestones into their essential conditions — the things that must be true, not the work that must be done. The difference matters: "Implement the matching engine" describes work. "Order matching logic handles all order types correctly" describes an outcome that can be verified.

**Outcome-oriented.** Every objective should describe a state of the world, not an activity. When you're done, someone should be able to read each objective and answer yes or no to "is this true?" If the answer requires checking a list of activities, you've written a task list, not objectives.

**Right-sized scoping.** An objective is not a task and not a milestone. The sizing test: Could a single agent complete this in one work session? Then it's a task — push it down. Could this stand alone as a major project checkpoint? Then it's a milestone — push it up. An objective occupies the middle: it requires multiple tasks to achieve, but it's one coherent goal within a larger milestone.

**1 to 3 objectives per milestone. No more.** This is a hard constraint, not a suggestion. Agents have a habit of generating exhaustive lists of objectives that don't add value — turning a clean milestone into a bureaucratic maze. If you can't capture what a milestone needs in 3 objectives, the milestone itself is too broad and must be split. The discipline of staying within 1-3 forces you to think at the right altitude: strategic outcomes, not implementation checklists.

**Necessary and sufficient.** For each milestone, your objectives should be collectively sufficient — if every objective is met, the milestone is achieved. They should also be individually necessary — removing any one would leave the milestone incomplete. If an objective could be removed without affecting the milestone, it doesn't belong there.

**Dependency awareness.** Objectives within a milestone often depend on each other. Data models must exist before business logic can process them. Core logic must work before integration tests can validate it. Map these dependencies explicitly — agents picking up objectives need to know what must be finished first.

---

## Reading the inputs

### The milestones (critical input)

The existing milestones are your primary input. For each milestone, understand:
- **What it achieves** — The title and description tell you the target state
- **Its checklist** — The definition-of-done items hint at what objectives are needed
- **Its references** — Design docs and specs that inform what the milestone requires
- **Its outputs** — What artifacts the milestone produces
- **Its position in the dependency graph** — What comes before and after it

Read the milestones using:
```bash
rex-cli milestone list
rex-cli milestone get <milestone-id>
```

**Review milestones already have their objectives.** The milestones skill creates review milestones with two objectives (audit and fix). Don't create additional objectives for review milestones unless you have a strong, specific reason. Focus your work on the work milestones.

### From design documents

- **Architecture** — Type-level design tells you what logical subsystems exist within each milestone's scope. Each subsystem or major type cluster might map to an objective.
- **Modules** — File/folder structure tells you where code boundaries are. Module groups often align with objective boundaries.
- **Error handling** — The error strategy may warrant its own objective within a milestone if the error types are substantial.
- **Library review** — Complex integrations (database, networking, async runtime) may need dedicated objectives.
- **Integration tests** — The test plan tells you what must be verifiable, which shapes objective success criteria.

### From onboarding documents

- **Goal and scope** — Keep objectives grounded in what's actually being built
- **Success measures** — These often map directly to objective-level success criteria
- **Known risks** — High-risk areas may need objectives that specifically address the risky aspect
- **UAT** — What the user will test informs what the final milestone's objectives must deliver

### From the checklist

Cross-reference the onboarding checklist. Items flagged for the planning phase may translate into objectives or inform their success criteria.

---

## The objectives planning process

### Step 1: Understand each milestone's intent

For each work milestone (skip review milestones), answer:
- What state does this milestone represent?
- What's different about the project before and after this milestone?
- What could go wrong if this milestone were declared "done" but actually wasn't?

That last question is particularly valuable — it reveals the hidden objectives. If declaring the milestone done prematurely would cause downstream problems, the thing that would catch the premature declaration is probably an objective you need.

### Step 2: Identify the necessary outcomes

For each milestone, list the outcomes that must all be true for it to be achieved. Work from the milestone's description, checklist, and referenced design documents.

Common objective patterns:
- **Core logic works** — The central behavior the milestone is about
- **Edge cases handled** — The important non-happy-path scenarios
- **Integration points connected** — Where this milestone's code touches other systems
- **Quality bar met** — Performance, reliability, or correctness thresholds
- **Artifacts produced** — Configuration, documentation, or output files that downstream work needs

Not every milestone needs all of these. A foundation milestone might just need "core types defined and validated" and "error handling in place." A feature milestone might need "happy path works," "error cases handled," and "integration tests pass."

### Step 3: Scope-check each objective

For each candidate objective, apply the sizing tests:

**Too small (really a task):**
- "Write the Order struct" — that's a single action, not a strategic outcome
- "Add serde derives" — implementation detail
- "Create the config file" — one step of a larger goal

**Too big (really a milestone):**
- "The entire API is functional" — that's a phase transition, not one outcome
- "All tests pass" — too broad; which tests? testing what behavior?

**Right-sized (an objective):**
- "Order validation rejects all malformed inputs with descriptive errors"
- "WebSocket connections maintain state across reconnections"
- "Configuration loads from file, environment, and CLI with correct precedence"

Each of these requires multiple tasks but represents one coherent verifiable outcome.

### Step 4: Design the dependency graph within each milestone

Objectives within a milestone may depend on each other. Map these:

- **Data before logic** — Type definitions and data models before the functions that process them
- **Core before periphery** — The main behavior before error handling, logging, or metrics
- **Internals before interfaces** — Business logic before API endpoints or CLI handlers
- **Foundation before tests** — The code must exist before integration tests can exercise it

These dependencies determine the order agents will work through objectives. Getting them wrong means agents will hit walls — trying to write tests for code that doesn't exist yet, or trying to integrate with types that haven't been defined.

Also consider dependencies *across* milestones. If an objective in milestone B needs something produced by milestone A, that's already handled by the milestone-level upstream dependency. But if an objective in milestone B specifically needs a particular objective from milestone A (not just the milestone as a whole), add a cross-milestone objective dependency using `--add-upstream`.

### Step 5: Define success criteria

Each objective should have a checklist — the specific, verifiable items that prove the objective is met. These are more granular than the milestone's checklist but more strategic than task-level checklists.

Good checklist items:
- "All order types (market, limit, cancel) processed correctly" — verifiable by running tests
- "Error messages include context sufficient for debugging" — verifiable by inspection
- "Latency under 10ms for single-order processing" — verifiable by benchmarking

Bad checklist items:
- "Code is clean" — subjective, not binary
- "Everything works" — not specific enough to verify
- "Tests written" — describes activity, not an outcome

### Step 6: Challenge your objectives

Before writing them to the CLI, interrogate your plan:

**The sufficiency test.** For each milestone, read your objectives as a list. If every single one is achieved, is the milestone truly complete? If not, you're missing an objective.

**The necessity test.** For each objective, imagine removing it. Would the milestone still be achievable? If yes, this objective doesn't belong to this milestone — it might belong to a different one, or it might not be an objective at all.

**The overlap test.** Are any two objectives within the same milestone describing the same outcome in different words? If they'd be done by the same work, merge them.

**The agent test.** Could an agent pick up one of your objectives, read its description and checklist, and understand what "done" means without having to guess? If not, the objective needs more clarity.

### Step 7: Enforce the 1-3 constraint — or escalate

After your challenge round, count the objectives per milestone. If any work milestone has more than 3 objectives, you have a structural problem — the milestone is too broad.

**Do not simply drop objectives to fit.** If you genuinely identified 5 necessary outcomes for a milestone, those outcomes don't disappear by pretending they don't exist. Instead, the milestone must be split.

**The escalation procedure:**

1. Identify which milestone has too many objectives
2. Determine how to split it into 2 milestones, each with 1-3 objectives, that preserve the original intent
3. Spawn subagents to perform the restructuring:
   - One agent to split the milestone using `rex-cli milestone upsert` (creating the new milestone, updating the old one)
   - One agent to rewire all upstream/downstream dependencies — every milestone that pointed to the old one as upstream or downstream must be checked and updated. The two new milestones must be correctly sequenced relative to each other and to the rest of the graph
   - After both complete, verify the dependency graph is intact: no orphaned references, no broken chains, no circular dependencies
4. Re-evaluate the objectives for the now-smaller milestones — they should fit within 1-3 each

This is an exceptional case, not the normal flow. If you find yourself needing to split milestones frequently, the milestones planning was done at too high a level. But it's better to fix the structure than to force-fit too many objectives or silently omit necessary ones.

---

## Writing the objectives using the CLI

Once you've planned all objectives (1-3 per work milestone, no exceptions), write them using the rex CLI. **Do not write planning.json directly.**

### Objective creation

```bash
rex-cli objective upsert \
  --id o-<milestone-topic>-<aspect> \
  --milestone m-<parent-milestone-id> \
  --title "Clear statement of the outcome that must be true" \
  --description "What this objective means concretely, including context on why it matters and what success looks like" \
  --add-reference design/architecture.md \
  --add-output src/module/relevant_file.rs \
  --add-checklist "c1:First verification point" \
  --add-checklist "c2:Second verification point" \
  --add-upstream o-<dependency-objective-id>
```

The parent milestone's `objectives` list is automatically updated.

### ID conventions

Use consistent, readable IDs that encode the hierarchy:
- `o-<milestone-topic>-<aspect>` (e.g., `o-core-engine-matching`, `o-core-engine-persistence`, `o-api-layer-routes`)
- The milestone topic prefix groups related objectives visually when listed
- The aspect suffix distinguishes objectives within the same milestone

### Ordering within a milestone

When creating objectives, order them by dependency — create upstream objectives first so downstream objectives can reference them with `--add-upstream`. This also produces a natural reading order when listing objectives.

### References and outputs

- **References** — Point to the design documents, specs, or other objectives that provide context. An agent picking up this objective should be able to read its references and understand the full picture.
- **Outputs** — List the files or artifacts this objective produces. This helps downstream objectives and tasks know what to expect.

---

## Writing output files

If you were given an output file path, also write a summary document:

```markdown
# Objectives Plan

**Date:** YYYY-MM-DD

## Overview
Brief description of the objectives structure and the reasoning behind it.

## Objectives by Milestone

### Milestone: [Title] (`m-id`)

#### 1. [Objective Title] (`o-id`)
**Description:** What this objective achieves
**Upstream objectives:** dependencies (or "none")
**Checklist:**
- [ ] Verification point 1
- [ ] Verification point 2
**References:** relevant design docs
**Outputs:** expected artifacts

#### 2. [Objective Title] (`o-id`)
...

(Repeat for each work milestone)

## Cross-Milestone Dependencies
Any objective-level dependencies that span milestones, with reasoning.

## Key Decisions
Why objectives were scoped this way, what alternatives were considered, what trade-offs were made.
```

---

## What done looks like

You're done when:
1. Every work milestone has 1-3 objectives (no exceptions — if any milestone needed more, it was split first)
2. Objectives are collectively sufficient and individually necessary for each milestone
3. All objectives have been created via the CLI using `rex-cli objective upsert`
4. All intra-milestone and cross-milestone dependencies are correctly wired
5. Each objective has a meaningful checklist that defines verifiable success criteria
6. Any requested output files have been written
7. Review milestones have been left alone (their objectives were created by the milestones skill)
