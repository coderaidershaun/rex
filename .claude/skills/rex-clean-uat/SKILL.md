---
name: rex-clean-uat
description: Prepare a project for user acceptance testing. Reads what was just built and writes a minimal guide — how to run it, what to verify, how to break it. Output stays brutally short. A reviewer picks it up and evaluates in minutes. Use when user says "prep for UAT", "write the eval guide", "how do I test what was built", or pipeline orchestrator dispatches `rex-clean-uat` step.
disable-model-invocation: false
user-invocable: true
---

# rex-clean-uat

Prepare project for UAT. Output = short guide. Reviewer reads it, runs it, verifies behavior, done.

I/O contract lives in `rex-utils-task-request`. Reads inputs given (what was achieved + how to run it). Writes guide to output path given.

**Brutally short.** Goal = reviewer evaluates in minutes, not hours. Every paragraph you remove is a win.

## What good UAT prep is

- **Behavior-first.** Reviewer cares what works, not how. No module names, no struct names, no architecture.
- **Runnable in 60 seconds.** Single command if possible. Setup steps only if unavoidable.
- **Observable checks.** Each verify item is something the reviewer sees with their eyes — output, log line, UI state, file written.
- **Concrete inputs.** Real values, real commands. Not "provide a valid order".
- **Stops where it should.** Don't document features that aren't done. Don't pad with future work.

## Output shape

```md
# UAT — <what was built, 4–6 words>

## What's new
<1–2 lines, plain language. What capability now exists>

## Run
```bash
<exactly the command(s) the reviewer types>
```

## Verify
- [ ] <observable outcome 1>
- [ ] <observable outcome 2>
- [ ] <observable outcome 3>

## Try this (optional)
- <edge case worth poking>
- <failure mode worth provoking>

## Where to look if it breaks
- Logs: `<path>`
- State: `<path or command>`
```

If something doesn't apply, drop the section. Don't pad to fit the template.

## Process

1. **Identify what's done.** From inputs, find chunks marked `done` since last UAT. That defines scope.
2. **Find the entry point.** Binary? CLI? HTTP server? TUI? One sentence on how the reviewer actually invokes the system.
3. **Map scenarios to checks.** Each behavior scenario the chunk covered → one verify line. Phrased as observable outcome, not as test name.
4. **Pick edge cases worth poking.** From scenarios already marked as failure-mode / ops scenarios. Optional. Skip if golden path is enough.
5. **Locate logs + state.** Where does the reviewer look when something looks wrong?
6. **Draft. Cut.** Write it. Then cut every word that doesn't earn its place.
7. **Sanity check.** Could a reviewer who has never seen this project run it from this guide alone? If no, fix. If yes, ship.
8. **Publish.** To path given by task envelope.

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| "Welcome to the UAT guide for..." | Filler | Delete. Start at the first useful word |
| Listing every CLI flag | Reviewer wants to verify, not configure | Show the one command. Mention flags only if needed |
| Internal terms (module names, traits, structs) | Reviewer doesn't know them. Distracting | Describe behavior in user terms |
| "Set up the database, install deps, configure env vars, ..." | Setup ceremony before the point | Pre-bake setup. If unavoidable, single block, no prose |
| Long verify list (15+ items) | Nobody checks 15 items. Most get skimmed | 3–6 items. The ones that matter |
| Documenting unfinished work | Wastes reviewer time on phantoms | Strict scope = what's `done` |
| Restating BDD scenarios verbatim | Verify lines are user-facing, scenarios are test-facing | Translate. "Detected gap" → "console shows `gap detected at seq N`" |
| Wall of text under each section | Reviewer skims. Walls block skim | Bullets. One line each |
| "Try this" with 10 edge cases | Pads. Reviewer picks one and ignores rest | Two cases max. Most expressive |
| No "where to look if broken" | Reviewer stuck on first failure | Always include. Logs + state location |

## Hand-off

UAT guide feeds:

- **The reviewer** — they pick it up cold, evaluate in minutes.
- **Future regressions** — the verify list is the next session's smoke test.
- **Demo prep** — the `Run` block + `Verify` block doubles as a demo script.

If the guide takes longer to read than the feature takes to run, the guide is wrong. Cut harder.

## Length target

Most UAT guides should fit on one screen. ~30–60 lines including code blocks. If longer, you're documenting too much.
