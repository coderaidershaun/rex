---
name: rex-plan-uat
description: Interview the user to align on what good UAT looks like for this project. One question at a time, with a recommended answer each time, until both agent and user agree on reviewer, time budget, demo medium, success criteria, and failure modes worth poking. Output = a UAT plan that `rex-clean-uat` will later turn into the runnable guide. Use when user says "plan UAT", "what should UAT look like", "agree on acceptance", or pipeline orchestrator dispatches `rex-plan-uat` step.
disable-model-invocation: false
user-invocable: true
---

# rex-plan-uat

Interview the user. Land on a shared picture of what UAT looks like for this project. Output = a UAT plan that downstream `rex-clean-uat` will turn into the actual guide.

I/O contract lives in `rex-utils-task-request`. Reads inputs given. Writes UAT plan to output path given.

**Dialogue, not monologue.** Questions one at a time. Recommend an answer each time. Wait for feedback. Move on.

## What good UAT planning is

- **Reviewer-shaped.** UAT's shape depends on who reviews. End user, operator, auditor, ops on-call — different demos.
- **Time-budgeted.** A 5-minute review is a different artefact than a 30-minute one. Pin the budget early.
- **Observable success.** Each acceptance criterion is something the reviewer sees with their eyes — output, log line, UI state, file written.
- **Forward-looking.** This runs *before* execution, not after. We're agreeing what the demo *will* check, not what it does check.
- **Iterative.** Quick alignment > exhaustive interrogation. Ask, recommend, listen, adjust.

## The interview — what to extract

Drive the conversation through these. One at a time. Recommend an answer based on prior inputs.

| Topic | Question shape | Why it matters |
|-------|----------------|----------------|
| Reviewer | Who picks this up? End user / operator / auditor / ops / you-pretending-to-be-fresh? | Demo language + depth scale with role |
| Time budget | How long should this take to evaluate — 5 / 15 / 30 min? | Bounds the verify list |
| Demo medium | TUI walkthrough / CLI / log inspection / web UI / notebook / staging deployment? | Determines `Run` block shape |
| Hands-on or watch | Reviewer drives, or you walk them through? | Affects guide vs script |
| Where it runs | Local / container / staging / real venue (paper money) / dry-run? | Setup ceremony cost |
| Success criteria | What 3–6 observable things, when true, mean "ship it"? | Becomes the verify list |
| Failure modes | Which edge cases / failure paths matter most to reviewer? | Becomes the optional "try this" |
| Stop-the-presses | What would make reviewer reject regardless of green checks? | Catches blind-spot rejections |
| Where to look on failure | Logs / state files / dashboards reviewer should check? | Reviewer unblock path |

Skip topics that are obvious from inputs. Don't ask what the docs already answer.

## Interview discipline

- **One question per turn.** Wall of questions = user freezes.
- **Recommend, don't blank-page.** "I'd recommend operator review, since the BDD scenarios are mostly ops-facing — sound right?" Faster to accept/reject than design from blank.
- **Code lookup > user lookup.** Don't ask "what's the entry point?" — read `Cargo.toml`, find the binary. Confirm if uncertain.
- **Update inline.** Each agreed answer goes into the UAT plan immediately. No batch.
- **Stop when both clear.** Aim for ~5–8 questions total, not 20. If still fuzzy after 8, escalate the fuzziness itself.

## UAT plan shape

```md
# UAT plan — <feature / phase>

## Reviewer
<role + 1-line context>

## Time budget
<minutes>

## Where it runs
<env + setup notes>

## Demo medium
<TUI / CLI / etc + entry point>

## Success criteria
1. <observable outcome 1>
2. <observable outcome 2>
3. <observable outcome 3>

## Failure modes worth poking
- <edge case + how reviewer triggers it>
- <ops scenario + expected degradation>

## Stop-the-presses
- <thing that would reject the build regardless>

## Where to look if it breaks
- Logs: <path>
- State: <path>
- Dashboards: <link>

## Open questions
- <unresolved item if any>
```

## Process

1. **Orient.** Read inputs. Find what was specified, what scenarios exist, where the entry point will live.
2. **Draft a guess.** Pre-fill the plan from inputs. This is what you recommend in each interview turn.
3. **Run interview.** One question at a time. Each carries your recommended answer + why.
4. **Update inline.** Each answered question → that section of the plan written / refined.
5. **Confirm done.** Read back the plan. Ask: "anything missing or off?" One round of polish.
6. **Publish.** UAT plan to output path given by task envelope.

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| Wall of questions in one message | User freezes. Quality drops | One question. Wait. Next |
| Open-ended "what do you want UAT to look like?" | Hands the design to the user. Slow + disengaged | Recommend a concrete answer. User accepts/rejects fast |
| Asking what the docs say | Reads as lazy. Wastes turn | Read the doc. Confirm only if ambiguous |
| Defining UAT after build | That's `rex-clean-uat`'s job, not this skill | Plan-UAT runs *before* execution. Forward-looking |
| Writing the actual guide | Out of scope. This is the plan, not the guide | UAT plan ≠ UAT guide. clean-uat consumes this and produces guide |
| 15 success criteria | Reviewer skims, picks 3, ignores rest | 3–6, ranked by importance |
| Vague criteria ("system works") | Not observable | Concrete: "console shows X, file Y exists, response Z within 200ms" |
| Skipping stop-the-presses | Reviewer rejects after green-tick → confused user | Always ask. Catches blind spots |
| Interview drags > 8 questions | Diminishing returns. Decision fatigue | Stop. Surface remaining fuzziness as Open Questions in plan |

## Hand-off

UAT plan feeds:

- **`rex-clean-uat`** — consumes plan + actual built artefacts → produces runnable UAT guide.
- **`rex-plan-bdd`** — UAT success criteria sometimes surface scenarios that were missed. Loop back if so.
- **Review gate** — gives the human reviewer prior visibility on what they'll be asked to verify.

Plan-UAT is not the demo. It's the agreement on what the demo will check.
