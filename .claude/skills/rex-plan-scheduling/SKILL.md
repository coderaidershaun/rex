---
name: rex-plan-scheduling
description: Build the autopilot work queue. Take refined inputs and emit a phases → chunks → tasks plan that an autopilot can execute one chunk at a time. Sizes chunks for a single agent session. Use when user says "build the plan", "schedule the work", "phases chunks tasks", "what does autopilot do next", or pipeline orchestrator dispatches `rex-planning-scheduling` step.
disable-model-invocation: false
user-invocable: true
---

# rex-planning-scheduling

Turn refined plan documents into the autopilot work queue. Output = phases → chunks → tasks. One chunk = one agent session.

I/O contract lives in `rex-utils-task-request`. Reads all inputs given. Writes the plan to the output path given. No tracker entries, no GitHub issues — the plan file is the queue.

## Three tiers

| Tier | Maps to | Granularity | Who acts |
|------|---------|-------------|----------|
| **Phase** | A PRD capability / milestone | Weeks. Demoable when complete. Ordered by dep. | Autopilot iterates phases |
| **Chunk** | A vertical slice. One+ behavior scenarios | One agent session. End-to-end through every layer | One agent per chunk. Merge unit |
| **Task** | One atomic unit of work invoking one skill | Minutes–hours. Fits inside a chunk | Agent dispatches via `rex-utils-task-request` |

## Sizing rules

**Don't underestimate agents.** A capable agent runs an entire chunk in one go — typically 2–8 tasks per chunk. Trust this.

**Most chunks: 2–8 tasks.** Standard vertical slice. Tests, implementation, wiring, integration.

**Single-task chunks** when the task is intrinsically large or volatile:

- Integration test hitting a real exchange / API / DB (real-world IO + reconnect cycles).
- Complex algorithmic implementation (matching engine, consensus, FFT, anything with subtle math).
- Real-funds runbook step (HITL — needs human-in-the-loop dispatch).
- Data migration / backfill scripts.
- Anything where one wrong byte costs hours of recovery.

**Split a chunk when** task count > 8 OR scope spans two bounded contexts OR success criteria split into independent demos. Mega-chunks usually hide poor decomposition.

## Task ordering within a chunk

Default: tests first, then implementation, then integration. TDD discipline.

Typical chunk task pattern:

1. Write unit tests (`rex-code-tests-unit-testing`)
2. Write contract test if chunk crosses a seam (`rex-code-tests-contract-seams`)
3. Write fitness function if chunk introduces an invariant (`rex-code-tests-fitness-functions`)
4. Implement until tests green
5. Write integration test if chunk touches real-world IO (`rex-code-tests-integration-testing`)
6. Wire into broader system

Skip steps that don't apply. Don't pad.

## Schedule format

The schedule is created and managed via the `rex project schedule` CLI. Agents never read or write `schedule.json` directly. The on-disk format is JSON matching this shape:

```json
{
  "project": "<project-id>",
  "phases": [
    {
      "id": "market-data-ingestion",
      "description": "Ingest + normalise + publish trades from supported venues.",
      "blocked-by": [],
      "state": "pending",
      "chunks": [
        {
          "id": "gap-detect-and-recover",
          "description": "Detect order book sequence gaps and trigger recovery.",
          "scenarios": [
            "Given live book at seq 1050, when delta seq 1052 arrives, then gap detected, recovery started, snapshot reload requested"
          ],
          "spec-refs": ["docs/spec/market-data.md#gap-invariant"],
          "blocked-by": [],
          "state": "pending",
          "tasks": [
            { "id": "gap-detection-unit", "skill": "rex-code-tests-unit-testing", "description": "Unit tests for sequence gap detection logic.", "state": "pending" },
            { "id": "implement-gap-detection", "description": "Implement sequence tracker + recovery trigger.", "state": "pending" }
          ]
        }
      ]
    }
  ]
}
```

Required fields per tier:

- **Phase**: `id`, `description`, `blocked-by`, `state`, `chunks`.
- **Chunk**: `id`, `description`, `scenarios`, `spec-refs`, `blocked-by`, `state`, `tasks`.
- **Task**: `id`, `description`, `state`. Optionally `skill`, `inputs`, `outputs`.

`state` ∈ `{pending, in-progress, done, blocked}`.

## Process

1. **Load every input.** Refined plan documents. Read fully.
2. **Identify phases.** Map each PRD capability to a phase. Sequence by dep. Independent capabilities can be parallel phases if autopilot supports parallelism.
3. **Slice phases into chunks.** One chunk per vertical slice. Each chunk maps to one+ behavior scenarios. Each chunk is demoable on its own.
4. **Apply sizing rules.** For each chunk: standard (2–8 tasks) vs single-task (intrinsically large) vs split (too big).
5. **Derive tasks per chunk.** TDD order. Pick the right test skill per task. Implementation tasks come after their tests.
6. **Resolve dep graph.** `blocked_by` between chunks; between phases. No cycles. Most chunks within a phase should be parallel-safe — sequential deps are a smell.
7. **Quiz user** if anything is ambiguous (a chunk you can't size, a phase whose scope is unclear).
8. **Publish.** Build a temporary local JSON file matching the format above, then call:
   ```sh
   rex project schedule replace --file <path>
   ```
   The CLI validates (no cycles, slug uniqueness, no state regression), persists `schedule.json`, and updates counters in `project.yaml` automatically.

   Alternative for incremental builds — add entities in order (phases first, then chunks per phase, then tasks per chunk):
   ```sh
   rex project schedule phase add --description "Market data ingestion" --id market-data-ingestion
   rex project schedule chunk add --phase 1 --description "Gap detect + recover" --id gap-detect-and-recover --scenario "..."
   rex project schedule task add --chunk 1.1 --description "Unit tests for gap detection" --skill rex-code-tests-unit-testing
   rex project schedule task add --chunk 1.1 --description "Implement gap detection"
   ```

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| Tasks named "Step 1", "Step 2" | Order encoded in name. Reorders break refs | Stable id describing the work |
| Chunk with 15 tasks | One agent can't hold this. Hides multiple slices | Split into 2+ chunks |
| Single-task chunk for trivial work | Wastes a chunk boundary. Group with sibling tasks | Merge into a 2–4 task chunk |
| Implementation task before its test task | Breaks TDD order. Autopilot writes code without target | Reorder. Tests first |
| Chunk spans two bounded contexts | Two owners → drift → contract bugs | Split per context. Use a contract test chunk to glue them |
| `blocked_by` cycle | Autopilot deadlocks | Re-derive deps. There's always an acyclic ordering |
| Task without a `description` | Agent has to guess intent | Every task carries a one-line description |
| Hardcoded file paths in plan | Paths rot. Plan rots | Reference scenarios + specs. Skill picks paths at execution |
| Over-decomposing (1-line tasks) | Coordination overhead > work | Group related lines into one task |
| Under-decomposing (huge "implement everything" task) | Unrunnable in one session | Break by sub-capability or test layer |

## Hand-off

Plan file feeds the autopilot loop:

- Autopilot picks the next pending task whose chunk's `blocked_by` is satisfied.
- Dispatches via `rex-utils-task-request` (skill name + inputs/outputs from task entry).
- On task complete → flip `state: done`. On chunk's last task done → chunk done. On phase's last chunk done → phase done.
- On stuck → escalate to user with the task id + reason.

If the plan is right, autopilot has no creative decisions to make — only execution. If the plan is wrong, autopilot stalls early. That's the design.

## Sanity-check before publish

- [ ] Every chunk has at least one scenario reference.
- [ ] Every chunk has at least one spec reference.
- [ ] No chunk has more than 8 tasks.
- [ ] No `blocked_by` cycles.
- [ ] Every task carries a `description`.
- [ ] TDD order preserved within each chunk.
- [ ] Every phase is independently demoable.
- [ ] Single-task chunks justified by sizing rule (intrinsically large only).
