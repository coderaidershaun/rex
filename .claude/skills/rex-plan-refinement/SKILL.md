---
name: rex-plan-refinement
description: Audit + harden every plan document produced so far. Reconcile vocabulary, surface contradictions, promote draft ADRs to accepted (or kill them), fill structural gaps. Allowed to update all inputs except the PRD. Last human-driven gate before autopilot inherits the plan. Use when user says "refine the plan", "audit the docs", "harden ADRs", "tighten the plan", or pipeline orchestrator dispatches `rex-plan-refinement` step.
disable-model-invocation: false
user-invocable: true
---

# rex-plan-refinement

Audit + harden everything produced upstream. Reconcile vocab. Promote draft ADRs to accepted. Catch drift before autopilot inherits it.

I/O contract lives in `rex-utils-task-request`. **You may update every input you receive — except the PRD.** PRD is read-only at this stage. If PRD looks wrong, escalate. Do not patch.

## What good refinement is

- **Reconcile, don't rewrite.** Goal = consistency across inputs, not redesign. If it's already coherent, leave it.
- **Vocab convergence.** Same term = same meaning everywhere. Cross-check every input against `CONTEXT.md`. Update `CONTEXT.md` when you sharpen a term.
- **Draft ADR → accepted ADR.** Drafts (status `proposed`) get a verdict here. None left dangling.
- **Surface contradictions, don't bury them.** Two inputs disagree → flag explicitly + propose a resolution. Don't silently pick a side.
- **Fill structural gaps, not creative ones.** Missing acceptance criterion on an existing scenario → add. Missing whole capability → escalate; not your call.
- **PRD is sacred.** Never edit. PRD-vs-reality disagreement → list ambiguities + escalate to user.

## Audit checklist

Run every input through this. Record findings.

- [ ] Domain vocab matches `CONTEXT.md`. Mismatch → fix + note in report.
- [ ] No two terms refer to the same concept. No one term refers to two concepts.
- [ ] Every spec'd contract has at least one behavior scenario covering it.
- [ ] Every behavior scenario maps to a spec'd type / state.
- [ ] Every bounded context has exactly one owner.
- [ ] No forbidden cross-context dependencies remain.
- [ ] Every draft ADR has a verdict: accepted, killed, superseded, or escalated.
- [ ] Every accepted ADR has source attribution (constraint origin or rejected-alternative reasoning).
- [ ] No `TBD` / `TODO` / `FIXME` left in any input. Resolve or escalate.
- [ ] "Canonical" used precisely. Each claim has multi-representation + agreed-form justification.

## Draft ADR hardening

Drafts are written during grill sessions with status `proposed`. Refinement renders verdict:

| Verdict | When | Action |
|---------|------|--------|
| `accepted` | Constraint valid + sourced, OR rejected alternative + reasoning sound | Update status. Add source attribution if missing. Renumber if numbering drifted |
| `killed` | Decision overtaken by later analysis. No longer load-bearing | Move to `docs/adr/archived/`. Add note explaining why. Never silent-delete |
| `superseded` | Newer ADR replaces this one | Update status to `superseded by ADR-NNNN`. Cross-link both |
| `escalated` | Refinement won't decide unilaterally | Leave `proposed`. Add to escalation list in audit report |

Every draft gets a verdict. No drafts survive refinement.

## Process

1. **Load every input.** Read fully. No skimming.
2. **Build vocab index.** Pull every domain term from `CONTEXT.md` + every input. Tag conflicts.
3. **Run audit checklist.** Per input. Record findings.
4. **Apply mechanical fixes.** Update inputs (NOT the PRD) where the fix is unambiguous.
5. **Harden draft ADRs.** Verdict each one per table above.
6. **Compile escalation list.** Anything you couldn't resolve unilaterally → into the audit report.
7. **Emit audit report.** What changed, what was hardened, what needs the user.
8. **Publish.** Updated inputs + audit report to paths given by task envelope.

## Audit report shape

```md
# Refinement audit — <date>

## Vocab fixes
- <term> in <doc>: was X, now Y. Source: CONTEXT.md.

## Contradictions resolved
- <doc-A> said X. <doc-B> said Y. Picked Y because <reason>.

## ADR verdicts
- ADR-0003: accepted. Source attribution added.
- ADR-0004: killed (overtaken by DDD context split). Archived.
- ADR-0005: escalated — needs user verdict on <question>.

## Gaps filled
- Scenario "X" was missing acceptance criterion. Added.

## Escalations (require user)
1. PRD ambiguity: <description>.
2. ADR-0005 verdict: <question>.
3. Cross-context dependency: <description>.

## Inputs updated
- <list of paths>
```

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| Editing PRD | PRD is source of truth at this stage | Escalate PRD issue. Never patch |
| Resolving contradictions silently | Hides decisions from audit trail | Surface in report. User decides if non-trivial |
| Mass-killing draft ADRs | Loses the rejection reasoning | Verdict + reason on each. Killed ≠ deleted |
| Adding new terms to `CONTEXT.md` | Vocab inflation | Only sharpen existing terms or merge synonyms. New concepts come from DDD, not refinement |
| Rewriting scenarios for style | Out of scope | Fix vocab + structural drift only. Style untouched |
| Skipping a draft ADR "because it looks fine" | Drafts left dangling rot worse than wrong-but-decided | Every draft gets a verdict, even if "accepted unchanged" |
| Inventing missing capabilities | That's the user's call, not refinement's | Escalate. Don't extend scope |

## Hand-off

Refined inputs feed:

- The plan-builder that produces the autopilot work queue (clean inputs → clean queue).
- Autopilot — cleaner inputs = fewer mid-execution escalations.
- Future grill sessions — vocab already converged, less re-litigation.

Refinement is the last human-driven gate. If something here is sloppy, autopilot will hit it later and stall.
