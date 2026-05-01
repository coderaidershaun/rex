---
name: rex-plan-discovery
description: Walk the design tree top-down — confirm goal, scope, constraints, domain terms — one node at a time. Capture as glossary entries + draft ADRs as decisions land. Output paths come from the task envelope. Use when user starts a new project, says "discovery", "what are we building", "grill my plan", "sharpen scope", or pipeline orchestrator dispatches `rex-plan-discovery` step.
disable-model-invocation: false
user-invocable: true
---

# rex-plan-discovery

I/O contract lives in `rex-utils-task-request`. Reads inputs given. Writes to outputs given. Never hardcode paths in this skill — task envelope is authoritative.

<what-to-do>

Walk design tree top-down. Confirm each node before descending.

Order: **goal → scope → constraints → domain terms → open boundaries.**

One question per node. Recommended answer each time. Wait feedback. Descend.

Question answerable by code reading -> read code instead. Don't ask.

</what-to-do>

<important>

Complexity bad. Complexity *very* bad. Complexity = enemy.

Help ask questions - as many as must - to reach elegant understanding in preparation for docs.

Challenge user if request can have better, less complex, elegant solution.

</important>

<supporting-info>

## Output targets

Task envelope (`rex-utils-task-request`) tells you where to write. Artefacts emerging from discovery:

- **Goal** — one-paragraph "what + why" landed first.
- **Scope** — in/out list, explicit non-goals.
- **Glossary / context doc** — resolved terms, domain language. Format: [CONTEXT-FORMAT.md](./CONTEXT-FORMAT.md).
- **Decision drafts (ADRs)** — `status: proposed` only. Format: [ADR-FORMAT.md](./ADR-FORMAT.md).

Lazy create. Only write when there's something to write. Don't materialise empty files.

## The design tree — walk in this order

### 1. Goal

Confirm before anything else. "You want X to achieve Y. Right?" Recommend a tightened version. Land one paragraph.

User says "build a trading engine" -> push: "Engine for what — execution, market-making, backtesting? Why now — new venue, replace existing, research?" One question at a time.

Goal landed -> write to goal artefact.

### 2. Scope

Goal locked -> bound it. What's in. What's explicitly out. Non-goals matter as much as goals — they kill scope creep later.

"Goal = X. Scope IN: A, B, C. Scope OUT: D, E. Agree?" Recommend an answer. Force the OUT list to be non-empty.

Scope landed -> write to scope artefact.

### 3. Constraints

External pressures that shape the rest: regulatory, vendor lock-in, latency budgets, deploy targets, team size, deadline. Surface them now — they become constraint-class ADR drafts.

"Hard deadline is X. Means we can't Y. Confirmed?"

Each constraint that meets the ADR bar -> draft ADR (`status: proposed`).

### 4. Domain terms

Walk the vocabulary. Every term the user uses, sharpen or reject. Conflicts with existing glossary -> call out. Vague terms -> propose canonical.

- "You say 'account' — Customer or User? Different."
- "Glossary say 'cancellation' = X. You mean Y. Which?"

Term resolved -> write to glossary now. No batching.

No impl details in glossary. Only terms a domain expert cares about.

### 5. Open boundaries

Where the design tree forks but you can't yet collapse. Surface them. Don't decide.

"Storage = Postgres or event log? Defer to DDD/C4."

Open questions get logged inline next to glossary, NOT as ADR drafts. ADR drafts wait until later steps earn them.

## Cross-ref code

User states behavior -> check code agrees. Contradiction? Surface: "Code cancel entire Orders. You said partial possible. Which?"

## ADRs — write as DRAFTS only

You write ADRs at status `proposed`. You do NOT harden them. `rex-plan-refinement` runs the verdict later (accepted / killed / superseded / escalated).

Offer a draft ADR only when all 3 true:

1. **Hard reverse** — change cost later = real
2. **Surprising w/o context** — future reader wonder "why this way?"
3. **Real trade-off** — genuine alternatives, picked for specific reason

Any missing -> skip.

At discovery (pre-PRD): only write **constraint** drafts (received from outside — regulatory, stakeholder, vendor lock-in, deadline). Shape + boundary drafts wait until DDD/C4 has earned them.

Every draft carries:

- `status: proposed`
- short title + 1–3 sentence body
- source attribution (where the constraint came from / what alternatives were rejected and why)

Always include `status: proposed` frontmatter so refinement can find drafts.

## Ergonomics

- **One question at a time.** Wall of questions = user freeze.
- **Recommended answer w/ each question.** No "what do you think?". User accept/reject faster than design from blank.
- **Descend only after confirm.** Goal not landed -> don't ask scope. Scope not landed -> don't ask constraints. Tree walked depth-first, one node at a time.
- **Code lookup > user lookup.** Don't make user recite what file say. Read file.
- **Update docs in same turn as decision.** Stale docs = next session re-litigate.

</supporting-info>

<!-- Adapted from Matt Pocock [YouTube] -->
