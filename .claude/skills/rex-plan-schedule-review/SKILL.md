---
name: rex-plan-schedule-review
description: Audit + refine the schedule (phases → chunks → tasks) against every prior document. Catches uncovered scenarios, oversized chunks, dep cycles, broken TDD order, orphan tasks. Last gate before autopilot inherits the queue. Use when user says "review the schedule", "audit the plan file", "tighten the schedule", "check coverage", or pipeline orchestrator dispatches `rex-schedule-review` step.
disable-model-invocation: false
user-invocable: true
---

# rex-schedule-review

Audit + refine the schedule produced upstream. Reconcile against every prior document. Catch drift before autopilot inherits it.

I/O contract lives in `rex-utils-task-request`. Reads all inputs given. Updates the schedule file in-place at the path given. Emits an audit report.

## What good schedule review is

- **Reconcile, don't rebuild.** Goal = the schedule matches everything upstream. If it does, leave it.
- **Coverage is the headline check.** Every PRD capability → phase. Every behavior scenario → chunk. Every spec'd type / state / invariant → at least one task.
- **Sizing discipline.** Most chunks 2–8 tasks. Single-task chunks only for intrinsically large work. Fix violations.
- **TDD order is sacred.** Tests come before implementation. Any chunk where impl precedes its test → reorder.
- **Dep graph is acyclic.** Cycles deadlock autopilot. Break them.
- **Surface ambiguity, don't bury it.** Can't size a chunk without a design call → escalate, don't guess.

## Audit checklist

Run the schedule through this:

### Coverage

- [ ] Every PRD capability has a phase covering it.
- [ ] Every behavior scenario maps to at least one chunk.
- [ ] Every spec'd type / state / invariant has at least one task producing or testing it.
- [ ] Every accepted ADR's implied constraint is reflected (e.g. ADR forbids cross-context import → fitness-function task exists).
- [ ] Every bounded context owner respected — no chunk crosses ownership.

### Sizing

- [ ] No chunk has more than 8 tasks.
- [ ] No chunk has 1 task unless intrinsically large (real-IO integration, complex math, real-funds, migration).
- [ ] No phase has only 1 chunk unless it's a true single-slice capability.
- [ ] Tasks are atomic — no "implement everything" tasks.

### Ordering

- [ ] Within each chunk, tests precede implementation.
- [ ] Within each chunk, contract tests come after the units they bridge.
- [ ] Integration tests come after unit + contract tests pass.
- [ ] Fitness functions present before the implementation they constrain.

### Graph integrity

- [ ] No `blocked_by` cycles (chunks or phases).
- [ ] No dangling `blocked_by` references (id doesn't exist).
- [ ] Most chunks within a phase are parallel-safe — sequential chains are a smell.
- [ ] No phase blocked by a chunk in a later phase.

### Hygiene

- [ ] Every task has a `description`.
- [ ] Every chunk has at least one `scenario` ref and one `spec_ref`.
- [ ] No hardcoded file paths in task bodies — refs only.
- [ ] No orphan chunks (no scenarios + no specs → either delete or attach).
- [ ] State fields valid: `pending | in-progress | done | blocked`.

## Refinements you can make

Mechanical and unambiguous fixes — apply directly via `rex project schedule` CLI. Never edit `schedule.json` by hand.

| Refinement | Command(s) |
|---|---|
| Split a chunk with > 8 tasks | `rex project schedule chunk add --phase <addr> ...` (add the new chunk) → `rex project schedule task move <addr> --to-chunk <new> ...` per moved task |
| Merge two trivial chunks | `rex project schedule task move <addr> --to-chunk <kept>` per task → `rex project schedule chunk remove <emptied>` |
| Reorder tasks within a chunk | `rex project schedule task move <addr> --to <pos>` |
| Add missing description / scenario / spec-ref | `rex project schedule chunk update <addr> --scenario "..." --spec-ref "..."` |
| Add missing blocked_by | `rex project schedule chunk update <addr> --blocked-by <id>` |
| Remove orphan chunk | `rex project schedule chunk remove <addr>` |
| Promote single-task chunk | `rex project schedule task move <addr> --to-chunk <sibling>` → `rex project schedule chunk remove <empty>` |

Run `rex project schedule show` after each batch of changes to eyeball the diff.

## Refinements you must escalate

Don't decide unilaterally:

- Coverage gap (capability with no phase, scenario with no chunk) — escalate. May be a dropped scope or missing scenario.
- Chunk that touches two bounded contexts — escalate. Owner split is a design call.
- Dep cycle that can't be broken without re-slicing — escalate. Re-slicing is design.
- Sizing violation that's already justified by prior context (chunk has 9 tasks because spec splits would break the slice) — escalate for confirmation.
- Orphan chunk that *might* be a real capability missing from PRD — escalate.

## Process

1. **Load every input.** Schedule + every prior planning document. Read fully.
2. **Build coverage matrix.** Every PRD capability → phases. Every scenario → chunks. Every spec → tasks. Highlight gaps.
3. **Run audit checklist.** Section by section. Record findings.
4. **Apply mechanical refinements.** Update the schedule in place.
5. **Compile escalation list.** Anything you couldn't resolve.
6. **Re-run sanity checks** on the modified schedule (no cycles introduced, all refs still valid, sizes still in bounds).
7. **Emit audit report.**
8. **Publish.** Updated schedule + audit report to paths given by task envelope.

## Audit report shape

```md
# Schedule review — <date>

## Coverage
- PRD capabilities covered: 7/7
- Scenarios mapped: 23/24 (1 gap — see escalations)
- Specs referenced: 41/45 (4 gaps — added refs to existing chunks)

## Refinements applied
- Chunk `gap-detect-and-recover` had 11 tasks → split into `gap-detection` (4 tasks) + `recovery-orchestration` (5 tasks).
- Chunk `replay-determinism` had impl task before test → reordered.
- Added `blocked_by: [book-engine-seam]` to `recovery-orchestration` (dep implied by spec).
- Removed orphan chunk `legacy-shim` (no scenario, no spec, not in PRD).

## Sizing
- Chunks before: 12 (1 oversized, 2 trivial-single)
- Chunks after:  14 (all within bounds)
- Single-task chunks: 2 (both real-API integration — justified)

## Escalations (need user)
1. Scenario "operator kills feed during recovery" — no chunk covers it. Add to which phase?
2. Chunk `risk-engine-init` touches Strategy + Risk contexts. Owner?
3. Possible dep cycle: `oms-submit` ↔ `risk-evaluate`. How should we slice?

## Files updated
- <schedule path>
```

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| Editing prior planning docs | Out of scope. Refinement skill handles those | Stay inside the schedule file |
| Resolving coverage gaps silently | Hides scope decisions | Escalate. User decides if a missing scenario is dropped or added |
| Splitting a chunk without preserving scenario links | Future tracing breaks | Carry scenario + spec refs across both halves |
| Re-ordering tasks without rerunning checks | Could introduce new violations | Re-run checklist after every batch of changes |
| Mass-merging single-task chunks | Loses single-task justification (real-IO etc.) | Only merge when sizing rule says so |
| Adding new specs / scenarios from scratch | That's planning, not review | Escalate. Reviewer doesn't author content |
| Inventing a phase to absorb orphans | Hides scope drift | Escalate orphan chunks. User tells you where they belong |

## Hand-off

Reviewed schedule feeds:

- Autopilot — clean queue → fewer mid-execution stalls.
- Review gate — human sees the audit report; approval becomes a check on changes, not a re-derive.
- Future schedule reviews — drift is incremental once the baseline is clean.

If something here is sloppy, autopilot hits it later and stalls. Review is the cheaper place to catch it.

## Sanity-check before publish

- [ ] Every refinement applied is reflected in the audit report.
- [ ] No new cycles introduced.
- [ ] No new dangling refs introduced.
- [ ] All escalations carry a clear question for the user.
- [ ] Re-run `rex project schedule show`; eyeball the diff.
- [ ] Re-run coverage matrix shows equal-or-better coverage than before.
