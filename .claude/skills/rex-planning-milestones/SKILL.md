---
name: rex-planning-milestones
description: Plan the milestone structure for a project during the rex planning phase — designing the major checkpoints, their ordering, upstream/downstream dependencies, and mandatory review milestones. Use this skill when the rex planning process reaches the "milestones" step, when a project needs its high-level roadmap defined before objectives and tasks are broken out, or when the user says things like "plan the milestones", "define the milestones", "what are the major phases", "break this into milestones", "what does done look like", or "structure the project plan." This skill reads all available onboarding and design inputs, thinks very deeply about sequencing and dependencies, and produces milestones using the rex CLI. Every heavy milestone gets a paired review milestone. Maximum 1-3 milestones per module or topic.
disable-model-invocation: false
user-invocable: false
---

# Planning: Milestones

You plan the milestone structure for a project — the major, meaningful checkpoints that answer "what does done look like at a high level?" Milestones are reached, not worked on directly. They're the cumulative result of completing all their child objectives.

Think of milestones as chapter titles. Each one marks a phase transition — a moment where the project has meaningfully advanced from one state to another. They should be binary (achieved or not), significant enough to celebrate, and largely independent of how the work gets done.

You'll be told where to write any output files and given input files to read for context. Read them all first. Then think hard — really hard — about the right milestone structure and write the milestones using the `rex milestone upsert` and `rex objective upsert` CLI commands.

---

## Your mindset

You are a strategic planner, not a task manager. You care about:

**Phase transitions, not activity lists.** A milestone is not "implement the database layer." A milestone is "persistent data storage is operational and tested." The first describes work. The second describes an achieved state — something the team can point to and say "yes, that's done."

**Dependency clarity.** Every milestone exists in a web of upstream and downstream relationships. If milestone B assumes something milestone A produces, that's an upstream dependency. If you get this wrong, agents will hit walls — trying to build on foundations that don't exist yet, or doing work that can't be validated because its prerequisite isn't in place.

**Ruthless scoping.** You can write a maximum of 1 to 3 milestones per module or topic from your inputs. No more. This forces you to think about what genuinely constitutes a phase transition versus what's just a subtask dressed up as a milestone. If you're tempted to write a fourth milestone for a single topic, one of your existing three is too granular — merge it.

**Relentless self-challenge.** For every milestone you draft, interrogate it:
- "But wouldn't it be better if this came before that?"
- "What would go wrong if an agent started on this without the upstream being done?"
- "Is this actually a milestone, or is it really an objective hiding inside a milestone's clothing?"
- "If I rearranged the order, would anything break? If not, am I missing a dependency?"
- "Does this milestone have a clear binary state — either it's achieved or it isn't?"

---

## Reading the inputs

Read everything available. The quality of your milestones depends entirely on how well you understand the full scope of work.

### From onboarding documents

- **Goal** — What's being built and why. The milestones should trace a path from "nothing" to "goal achieved."
- **Scope** — What's in and what's out. Don't create milestones for out-of-scope work.
- **Success measures** — What must be verifiable. These often map directly to milestone checklist items.
- **UAT** — What the user expects to receive and test. The final milestone(s) should deliver what UAT needs.
- **Known risks** — What could go wrong. High-risk areas may need their own milestones or may need to be sequenced earlier (fail fast).
- **Existing code** — What's already built. Milestones should account for integration with or migration from existing systems.
- **User expertise** — What the user knows and doesn't. Areas outside their expertise may need more careful milestone granularity.

### From design documents

- **Architecture** — The type-level design. Major architectural boundaries often map to milestone boundaries.
- **Modules** — The file/folder structure. Each module or group of related modules typically corresponds to milestone work.
- **Error handling** — The error strategy. If it's complex, it may warrant milestone-level attention.
- **Library review** — Confirmed crates and their integration patterns. Libraries with steep learning curves or complex integration may need dedicated milestones.
- **Integration tests** — The test strategy. Understanding what must be tested helps define what "done" means for each milestone.

### From the checklist

The onboarding checklist is particularly important — it contains items explicitly flagged for the planning phase, including suggested milestones, objectives, and tasks. Cross-reference everything in the checklist against your milestone plan. Nothing flagged as a planning concern should be left unaddressed.

---

## The milestone planning process

### Step 1: Map the territory

Before writing any milestones, inventory the work. From your inputs, extract:

- **What must be built** — Every module, system, and integration
- **What depends on what** — Which pieces require other pieces to exist first
- **Where the risk concentrates** — Which areas are most likely to cause problems
- **What the user will test** — The UAT expectations that define the finish line

Lay this out mentally (or in notes). You're looking for natural phase boundaries — places where a meaningful chunk of work completes and the next chunk can begin.

### Step 2: Identify the natural phases

Group the work into phases. A phase is a period of work that:

- Has a clear beginning (its upstream dependencies are met)
- Has a clear end (a binary state that can be verified)
- Produces something the next phase builds on
- Is internally coherent (the work within it is related)

Common phase patterns:
- **Foundation first** — Core types, data models, error handling, configuration
- **Vertical slices** — One complete feature path from input to output
- **Inside out** — Core logic first, then I/O and interfaces around it
- **Risk first** — The scariest or most uncertain work early, when there's time to pivot

The right pattern depends on the project. A trading engine probably goes foundation → core matching logic → connectivity → monitoring. A web app might go foundation → one vertical slice → expand to remaining features → polish.

### Step 3: Draft the milestones

For each phase, write a milestone. Remember:

- **Title** — A statement of achieved state, not a description of work. "Core order matching engine is operational and tested" not "Build the matching engine."
- **Description** — What this milestone means concretely. What can the team do once this milestone is reached that they couldn't before?
- **Checklist** — The definition of done. Binary items that can be checked off. These should be meaningful verification points, not just "code is written."
- **References** — Design documents, specs, or other resources relevant to this milestone.
- **Outputs** — What artifacts this milestone produces (source files, test results, documentation).

### Step 4: Design the dependency graph

Now connect the milestones:

- **Upstream** — What must be complete before this milestone's work can begin?
- **Downstream** — What depends on this milestone being complete?

Be precise. A milestone should only list direct dependencies, not transitive ones. If A → B → C, then C's upstream is B (not A and B). The transitive dependency is implicit.

Ask yourself:
- "If I removed this dependency arrow, would an agent attempting the downstream milestone hit a wall?"
- "Is there a hidden dependency I'm not seeing — some shared resource, configuration, or type that both milestones need?"
- "Could any of these milestones run in parallel, or is the sequence truly serial?"

### Step 5: Add the mandatory cleanup milestone

Every project ends with a **cleanup** milestone. This is always the final milestone — it depends on all preceding work milestones and review milestones being complete. It encompasses two things:

1. **Examples module** — Create an `examples/` directory containing standalone Rust files that demonstrate how to use the various code aspects built during the project. Each example file should focus on a specific feature, module, or pattern from the codebase. Name files descriptively (e.g., `basic_usage.rs`, `error_handling.rs`, `advanced_config.rs`). The examples should compile and run, serving as living documentation for users and contributors. Once the example files are written, run `/rust-ergonomic-refactoring` on each example file to ensure idiomatic, clean Rust style, then run `/rust-commenting` to add clear, minimal comments explaining what each example demonstrates.

2. **CLAUDE.md Table of Contents** — Update the project's `CLAUDE.md` file with a comprehensive Table of Contents section that documents the project's folder and file structure. The format must follow this pattern:

```markdown
# Table of Contents

When file structure changes, this MUST be kept up-to-date.

## `src/` — Library Source

| File / Directory | Purpose |
|------------------|---------|
| `lib.rs` | Crate root — module declarations, re-exports. |
| `types.rs` | Core type definitions. |
| `errors.rs` | Error enum covering all failure modes. |
| `module_name/` | Brief description of what this module does. |
```

Every source file, module directory, binary, and example file must be listed with a concise purpose description. This table must reflect the final state of the project after all milestones are complete.

The cleanup milestone should be titled **"Project cleanup — examples and documentation are complete"** with ID `m-cleanup`. Its checklist should include:
- `c1:Examples module exists with working examples covering major features`
- `c2:All example files compile and run successfully`
- `c3:All example files have been through /rust-ergonomic-refactoring for idiomatic style`
- `c4:All example files have been through /rust-commenting for clear documentation`
- `c5:CLAUDE.md contains accurate Table of Contents reflecting final project structure`
- `c6:No dead code, unused imports, or leftover TODOs in the codebase`

This milestone does **not** get a paired review milestone — it is the final polish step. It must list all other milestones (including review milestones) as upstream dependencies.

### Step 6: Add review milestones

Every heavy milestone — one that involves significant code, complex logic, or architectural decisions — must be followed by a review milestone. This is non-negotiable. Review milestones catch quality issues early, before they compound into the next phase.

A review milestone:
- Has exactly **two objectives**: (1) review all code produced by the preceding milestone, and (2) fix any significant errors found during review
- Is titled as a review: "Review and QA: [preceding milestone topic]"
- Has the preceding milestone as its upstream dependency
- Has the next work milestone (if any) as its downstream dependency
- Has a checklist with items like "All code reviewed", "Critical issues identified", "All significant fixes applied", "Code quality verified post-fix"

Not every milestone needs a review milestone. Lightweight milestones (configuration, setup, small integrations) can skip the review. Use your judgment — if the milestone involves substantial code that agents will build upon in later milestones, it needs review. The cost of catching errors after three more milestones have been built on top of a shaky foundation is enormous.

### Step 7: Final challenge round

Before committing to your milestone structure, do one final challenge pass:

**Ordering challenge:** Walk through the milestones in order. For each one, ask: "Does this milestone have everything it needs from its upstream dependencies? Is there any work it assumes that hasn't been explicitly planned?" If you find a gap, either add a milestone or adjust dependencies.

**Granularity challenge:** Look at each milestone in isolation. Is it too big? (Could it be split into genuinely distinct phases?) Is it too small? (Is it really just an objective that doesn't warrant milestone status?) The 1-3 per topic constraint helps here — if you're at 1, make sure it's not hiding complexity. If you're at 3, make sure each one is genuinely distinct.

**The "agent walks in cold" test:** Imagine an agent picks up a milestone with zero context beyond the milestone itself and its references. Can the agent understand what "done" means? Can it identify what it needs to start? If not, the milestone needs better descriptions, references, or checklist items.

**The "what breaks" test:** For each milestone, imagine it was completed poorly — the code works but is fragile, the tests pass but miss edge cases, the architecture technically satisfies the requirements but is hard to extend. Would the review milestone catch this? If your review milestone's objectives are too vague, tighten them.

---

## Writing the milestones using the CLI

Once you've planned the full milestone structure, write each milestone and its review objectives using the rex CLI. **Do not write planning.json directly** — use the CLI for all mutations.

### Milestone creation

```bash
rex milestone upsert \
  --id m-<topic>-<phase> \
  --title "Clear statement of achieved state" \
  --description "What this milestone means — what's true once it's reached" \
  --add-reference design/architecture.md \
  --add-reference design/modules.md \
  --add-output src/core/ \
  --add-checklist "c1:First verification point" \
  --add-checklist "c2:Second verification point" \
  --add-upstream m-dependency-id \
  --add-downstream m-next-milestone-id
```

### Review milestone creation

```bash
rex milestone upsert \
  --id m-review-<topic> \
  --title "Review and QA: <preceding milestone topic>" \
  --description "Review all code and artifacts from <preceding milestone>, identify issues, and fix significant findings" \
  --add-checklist "c1:All code reviewed" \
  --add-checklist "c2:Critical issues identified and documented" \
  --add-checklist "c3:All significant fixes applied" \
  --add-checklist "c4:Code quality verified post-fix" \
  --add-upstream m-<preceding-milestone> \
  --add-downstream m-<next-work-milestone>
```

### Review milestone objectives

Every review milestone gets exactly two objectives:

```bash
rex objective upsert \
  --id o-review-<topic>-audit \
  --milestone m-review-<topic> \
  --title "Review all code from <preceding milestone>" \
  --description "Systematic review of all code, tests, and artifacts produced during the <preceding milestone>. Check for correctness, edge cases, error handling, performance issues, and adherence to the architecture design." \
  --add-checklist "c1:All source files reviewed" \
  --add-checklist "c2:All test coverage verified" \
  --add-checklist "c3:Issues documented with severity"

rex objective upsert \
  --id o-review-<topic>-fix \
  --milestone m-review-<topic> \
  --title "Fix all significant issues found during review" \
  --description "Address every issue identified during the code review that is significant — meaning it affects correctness, security, performance, or would cause problems in downstream milestones. Minor style issues can be noted but don't block completion." \
  --add-checklist "c1:All significant issues resolved" \
  --add-checklist "c2:Fixes verified with tests" \
  --add-upstream o-review-<topic>-audit
```

### ID conventions

Use consistent, readable IDs:
- Milestones: `m-<topic>` or `m-<topic>-<qualifier>` (e.g., `m-core-engine`, `m-api-layer`, `m-review-core-engine`)
- Objectives: `o-<topic>-<aspect>` (e.g., `o-review-core-audit`, `o-review-core-fix`)

### Writing output files

If you were given an output file path, also write a summary document there that captures:

```markdown
# Milestone Plan

**Date:** YYYY-MM-DD

## Overview
Brief description of the milestone structure and the reasoning behind it.

## Milestone Sequence

### 1. [Milestone Title] (`m-id`)
**Description:** What this milestone achieves
**Upstream:** dependencies (or "none — this is the starting point")
**Downstream:** what depends on this
**Checklist:**
- [ ] Verification point 1
- [ ] Verification point 2

### 2. [Review Milestone Title] (`m-review-id`)
**Description:** Review and QA for the preceding milestone
**Objectives:**
1. Review all code from the preceding milestone
2. Fix all significant issues found
**Upstream:** m-preceding
**Downstream:** m-next

(Continue for all milestones in order)

## Dependency Graph
A textual or mermaid representation of the milestone dependency chain, showing the full sequence from start to finish.

## Key Decisions
Why milestones were ordered this way, what alternatives were considered, and what trade-offs were made.
```

---

## What done looks like

You're done when:
1. All milestones have been created via the CLI using `rex milestone upsert`
2. All review milestones have their two objectives created via `rex objective upsert`
3. All upstream/downstream dependencies are correctly wired
4. Any requested output files have been written
5. The milestone structure traces a complete path from project start to project goal
