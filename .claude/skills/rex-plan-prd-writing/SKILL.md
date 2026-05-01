---
name: rex-plan-prd-writing
description: Synthesize current context into PRD (Product Requirements Doc). Stage 2 of rex pipeline — translates client request / problem statement into product or platform definition. Use when user wants PRD from current context, says "write PRD", "draft product spec", or pipeline orchestrator dispatches `rex-plan-prd-writing` step.
disable-model-invocation: false
user-invocable: true
---

# rex-plan-prd-writing

Takes problem statement → produces PRD. Feeds spec-driven development (stage 3).

**No interview.** Synthesize from convo + repo. Ask only if blocking ambiguity.

## Task envelope (if dispatched via pipeline)

See `rex-utils-task-request`. Expect:

- `inputs`: problem statement file(s), CONTEXT.md, ADRs, prior PRDs.
- `outputs`: PRD path (e.g. `docs/prd/<slug>.md`). If absent → return inline.
- `instructions`: overrides skill defaults.
- `complexity`: `low` = single-section PRD, `high` = full template + cross-context scenarios.

Read every input. Write every output. Flip `completed: true` on success.

## Process

1. **Orient.** Skim repo. Read CONTEXT.md (domain glossary), `docs/adr/`, prior PRDs. Reuse vocab. Respect prior decisions.
2. **Identify scope unit.** Subsystem (e.g. market data engine) vs platform (e.g. trading engine). PRD depth scales w/ scope — see pipeline.md §8 (compress when small).
3. **Sketch deep modules.** List modules to build/modify. Prefer deep modules (big functionality, simple stable interface). Confirm w/ user only if non-obvious.
4. **Draft PRD** using template below.
5. **Publish.** Write to output path. If issue-tracker integration exists, file w/ `needs-triage` label.

## PRD template

```md
# PRD — <title>

## Problem statement
User-perspective problem. Business reason system exists. Constraints. Non-goals.

## Solution
User-perspective solution. What changes for them.

## Users
Roles: who uses, who operates, who depends on output.

## In scope
Capabilities platform must deliver.

## Out of scope
Explicit exclusions. Prevent scope creep.

## Non-functional requirements
Latency, durability, availability, observability, schema versioning, security, audit.

## User stories
Long numbered list. Format: `As <actor>, I want <feature>, so that <benefit>`.
Cover golden path + edge cases + ops scenarios.

## Implementation decisions
- Modules to build/modify (deep where possible).
- Module interfaces (signature-level, not file-level).
- Tech clarifications from user.
- Architectural decisions.
- Schema changes.
- API contracts.
- Cross-context interactions.

No file paths. No code. Both rot fast.

## Testing decisions
- Test external behavior, not internals.
- Modules to test.
- Prior art (similar tests in codebase).
- Fitness functions worth promoting (see `rex-code-tests-fitness-functions`).

## Open questions
Unresolved items. Owner + deadline if known.

## Further notes
Anything else that future-you needs.
```

## Rules

- Use project's ubiquitous language (CONTEXT.md). No new synonyms.
- "Canonical" only if multiple representations exist + one is agreed standard. See pipeline.md §9.
- Distinguish raw vs canonical vs authoritative if data crosses boundaries.
- PRD ≠ spec. PRD = intent + scope. Specs (schemas, state machines, invariants) come next stage (`rex-plan-sdd`).
- No file paths in PRD body. No code snippets.
- Compress sections that don't apply. Don't pad.

## Hand-off

PRD output feeds:

- `rex-plan-sdd` — formalize contracts/invariants/state machines.
- `rex-plan-ddd` — discover bounded contexts.
- `rex-plan-c4-architecture` — communicate structure.

Mark hand-off targets in `outputs` of next pipeline step.

<!-- Adapted from Matt Pocock — refined for rex pipeline (pipeline.md stage 2). -->
