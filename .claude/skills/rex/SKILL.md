---
name: rex
description: Top-level rex pipeline driver. Run when the user types `/rex`, asks "what's next on the project", "drive the pipeline", "do the next step", or wants a status read on the active project. Shells out to the `rex` CLI to fetch project + step envelopes, then dispatches the next step.
disable-model-invocation: false
user-invocable: true
---

# /rex — pipeline driver

Entry point for the rex pipeline. On invocation you are either driving the next step or reporting state.

The below must be followed to the letter. Do not skip any steps. You are the in the drivers seat of a very important project.

## Step 1 — Fetch state from disk

Run these in order. Do not skip.

| # | Command | Why |
|---|---------|-----|
| 1 | `rex project meta` | Project envelope. JSON: `project-id`, `title`, `complexity`, progress counters. |
| 2 | `rex project step` | Step envelope for the first incomplete step, OR `{"status":"all-steps-complete"}`. |
| 3 | `rex project` | End-to-end pipeline + completion status of each step. Use to brief the user on overall position. |

If `rex project step` returns `{"status":"all-steps-complete"}`, report progress to the user and stop. No outer loop.

**Important Rules** — `rex project step`

- Always pass the `rex project meta` result to any subagent you call.
- `rex project step` determines whether you act yourself or dispatch a subagent.
- If the step object has no `agent` field, you complete the step yourself using the named `skill`.
- If the step object has an `agent` field, dispatch to that agent and pass it the step object.

## OUTER LOOP

Dispatch on the `step` field of the step envelope.

### case `step == "task-execution"`

This step has no `agent` or `skill`. The CLI orchestrates it via `chunk-next` / `chunk-prior` / `task complete`.

Inner loop — repeat until `chunk-next` returns `{"status":"all-chunks-complete"}`:

  [ ] run `rex project chunk-next` → parse JSON. Call this `current_chunk`.
  [ ] if `current_chunk` is `{"status":"all-chunks-complete"}` → break inner loop
  [ ] run `rex project chunk-prior` → parse JSON (may be `{"status":"no-prior-chunk"}` on first chunk)
  [ ] launch `rex-rust-software-engineer` subagent. Pass: project meta, `current_chunk` (the chunk to build, with its `tasks`, `scenarios`, `spec_refs`), chunk-prior JSON (context on what just shipped).
  [ ] launch `rex-rust-senior-auditor` subagent. Pass: project meta, same `current_chunk` + chunk-prior JSON. Audit the engineer's diff, fix bugs, improve ergonomics, deepen architecture.
  [ ] task-completion sub-loop — drain every `pending`/`in-progress` task in `current_chunk.tasks` (run `rex project task complete` once per task). Stop when one of:
        - response is `{"status":"no-active-task"}` (schedule exhausted)
        - returned task's `id` is the last `id` in `current_chunk.tasks` (chunk auto-promoted; CLI now points at next chunk)
  [ ] continue inner loop (next chunk)

After inner loop ends:
  [ ] run `rex project step complete` to mark `task-execution` itself done
  [ ] continue outer loop

### case step has no `agent` field

  [ ] load the `skill` named on the step
  [ ] ingest the `inputs` document(s) if any
  [ ] complete the step using the skill
  [ ] write the `outputs` document(s) if any
  [ ] **if step is `discovery`** — once discovery is complete you have a sharper read on the project than the slug-derived title gave us. Refine `project.yaml` metadata in one call: `rex project update --title "<concise human title>" --subtitle "<one-line hook>" --description "<2–3 sentence brief>"`. Skip any field that is already accurate; pass an empty string to clear.
  [ ] run `rex project step complete`
  [ ] continue outer loop

### case step has an `agent` field

  [ ] dispatch to that agent. Pass project meta + the full step object.
  [ ] run `rex project step complete`
  [ ] continue outer loop

### case `rex project step` returned `{"status":"all-steps-complete"}`

  [ ] report final state to user. Stop.

### default

  [ ] unrecognised state. Print the offending JSON. Stop and let the user investigate.
