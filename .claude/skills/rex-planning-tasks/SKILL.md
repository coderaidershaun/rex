---
name: rex-planning-tasks
description: Plan the tasks for each objective during the rex planning phase — designing the atomic, actionable work items that agents will execute, with precise upstream/downstream dependency management. Use this skill when the rex planning process reaches the "tasks" step, when objectives exist and need their tasks defined, or when the user says things like "plan the tasks", "break down the objectives", "what work needs doing", "define the tasks", "create the task breakdown", or "what should agents actually build." This skill reads existing milestones/objectives and all design inputs, then produces tasks using the rex CLI with careful attention to dependency chains. Maximum 1-3 tasks per objective.
disable-model-invocation: false
user-invocable: false
---

# Planning: Tasks

You plan the tasks for each objective — the atomic units of work that agents will pick up and execute. Tasks answer "what specific thing needs to be done next?" Each task should be completable in a single work session, have a clear definition of done, and require no further decomposition.

Tasks are the leaves of the planning tree. Everything above them (milestones, objectives) describes outcomes and states. Tasks describe actions — concrete, assignable work with a beginning and an end. "Implement the token-generation endpoint for password reset" is a task. "Users can securely reset their passwords" is an objective. Don't confuse the two.

You'll be told where to write any output files and given input files to read for context — including the milestones and objectives that already exist. Read everything first. Then design the tasks with obsessive attention to dependency ordering, and write them using the `rex task upsert` CLI.

---

## Your mindset

**Atomic and actionable.** A task should be something an agent can pick up, understand in isolation (with its references), and complete without needing to make strategic decisions. The strategic decisions were made at the objective and milestone levels. Tasks are execution.

**1 to 3 implementation tasks per objective. No more.** (Integration testing objectives always have exactly 3 tasks — see Step 7.) This is a hard constraint for implementation work. Agents love to generate exhaustive task lists — 8 tasks for something that genuinely needs 2. This creates busywork, inflates the plan, and produces tasks that don't add value. If you can't capture what an objective needs in 3 implementation tasks, the objective is too broad and must be split. The discipline forces you to think about what work genuinely matters versus what's noise. **On top of the 1-3 implementation tasks, code-producing objectives get up to 2 mandatory quality tasks** (ergonomic refactoring + commenting) — see Step 8. These don't count against the 1-3 limit.

**Upstream and downstream are everything.** This is the most critical aspect of task planning. A task's upstream dependencies tell an agent "don't start until these are done." Its downstream dependencies tell the system "these are blocked until this completes." Get the dependencies wrong and agents will either: (a) start work that fails because a prerequisite doesn't exist, or (b) sit idle waiting for something that's already done but not wired correctly.

Every task must have its upstream and downstream dependencies explicitly set. No exceptions. Even if a task has no dependencies (it's a starting task), state that explicitly with an empty upstream. An agent should never have to guess what it can start.

**Designed for handoff.** The agent executing a task is not you. It has no memory of this planning conversation. It will read the task's title, description, checklist, references, and dependencies — and that's all it knows. If those fields don't contain enough information for cold-start execution, the task is underspecified.

---

## Reading the inputs

### The objectives (critical input)

The existing objectives are your primary input. For each objective, understand:
- **What outcome it describes** — The title and description define the target state
- **Its checklist** — Success criteria that tasks must collectively satisfy
- **Its parent milestone** — The broader context this objective sits within
- **Its dependencies** — Which objectives must complete first, which come after
- **Its references** — Design docs and specs that inform what tasks are needed

Read the objectives using:
```bash
rex objective list
rex objective get <objective-id>
```

Also read the milestones for broader context:
```bash
rex milestone list
rex milestone get <milestone-id>
```

### From design documents

- **Architecture** — Type definitions, function signatures, and data flow tell you exactly what code needs to be written. Each significant struct, trait, or function cluster may map to a task.
- **Modules** — The file/folder plan tells you where code goes. Tasks should align with module boundaries — a task that touches 5 modules is too broad.
- **Error handling** — Error types and propagation patterns are often their own task within an objective.
- **Library review** — Integration with unfamiliar crates may need a dedicated task if the integration is non-trivial.
- **Integration tests** — Test tasks should reference the test plan so the agent knows exactly what tests to write.

### From onboarding documents

- **Goal and scope** — Keep tasks grounded; don't create tasks for out-of-scope work
- **Success measures** — These inform task-level checklists (the concrete verification points)
- **Existing code** — If code already exists, tasks may involve modification rather than creation

---

## The task planning process

### Step 1: Understand each objective's requirements

For each objective, answer:
- What concrete work must happen for this outcome to become true?
- What artifacts (files, tests, configs) must exist when this objective is complete?
- What's the natural order of that work?

Work from the objective's description, checklist, and referenced design documents. The design documents are particularly important — they contain the specific structs, functions, and module assignments that tell you exactly what to build.

### Step 2: Identify the atomic work items

For each objective, list the concrete actions needed. Then compress ruthlessly into 1-3 tasks.

**How to compress:** If you have 5 candidate tasks, ask which ones are genuinely independent pieces of work versus which ones are just steps within a single coherent task. "Define the struct," "implement the methods," and "add the derives" are not three tasks — they're one task: "Implement the Order type with its methods and derives." An agent writing the struct will naturally add the derives and methods in the same session.

Common task patterns:
- **Implement core types and logic** — The central code for an objective (structs, functions, core behavior)
- **Implement integration/wiring** — Connecting the core logic to other parts of the system (routes, CLI handlers, database calls)
- **Implement tests** — Unit tests, integration tests, or both for the objective's scope

Not every objective needs all three. Some objectives are purely about wiring (1 task). Some are about complex logic (2 tasks: logic + tests). Match the tasks to the actual work, not to a template.

### Step 3: Size-check each task

**Too small (merge into another task):**
- "Add `#[derive(Debug)]` to the Config struct" — that's a line of code, not a task
- "Create the `models/` directory" — that happens as part of writing the first model
- "Import the serde crate" — that's a dependency of implementing serialization, not a standalone task

**Too big (split or push up to objective):**
- "Implement the entire WebSocket server" — if this involves connection handling, message parsing, state management, and reconnection logic, it's an objective
- "Write all tests for the matching engine" — if the matching engine has 3 objectives, each with its own tests, this doesn't belong in any single objective

**Right-sized (a task):**
- "Implement the OrderBook struct with insert, cancel, and best-price methods"
- "Wire the /orders endpoint to the matching engine with request validation"
- "Write integration tests for order submission through the REST API"

Each is completable in one session, produces clear artifacts, and an agent can understand exactly what to do.

### Step 4: Design the dependency graph

This is the most important step. Every task must have its upstream and downstream dependencies explicitly defined.

**Within an objective:**
Tasks within the same objective often depend on each other:
- Implementation before tests (you can't test what doesn't exist)
- Core types before logic that uses them (if split into separate tasks)
- Wiring depends on both the thing being wired and the thing it wires into

**Across objectives within the same milestone:**
If task T2 in objective B needs something produced by task T1 in objective A, add T1 as an upstream of T2. This is the most common source of dependency bugs — tasks that implicitly assume another objective's work is done without declaring the dependency.

**Across milestones:**
Cross-milestone task dependencies should be rare (milestone-level dependencies usually handle this). But if a specific task in milestone 2 depends on a specific task's output from milestone 1, wire it explicitly.

**How to verify your dependency graph:**

1. **The cold-start test.** For each task with no upstream dependencies (a "root" task), ask: "Can an agent start this right now with nothing else completed?" If not, you're missing an upstream.

2. **The completion cascade test.** Walk through tasks in dependency order. After completing each task, check: "Are any downstream tasks now unblocked that shouldn't be? Are any tasks still blocked that should now be free?" Both indicate wiring errors.

3. **The parallel opportunity test.** Look for tasks that have no dependency relationship to each other. These can run in parallel. If everything is serial, ask whether some dependencies are artificial — is task B genuinely blocked by task A, or could they run concurrently?

4. **The orphan test.** Every task should be reachable from a root task via the dependency chain. Orphaned tasks (no upstream, no downstream, disconnected from everything) are a red flag — they're either unnecessary or missing their dependencies.

### Step 5: Define clear completion criteria

Each task gets a checklist — the specific, binary items that prove the task is done. These should be concrete enough that an agent (or a reviewer) can verify them without judgment calls.

Good checklist items:
- "OrderBook::insert handles duplicate order IDs by returning an error"
- "Endpoint returns 400 with a descriptive message for malformed JSON"
- "Integration test covers the happy path: submit order → match → fill notification"

Bad checklist items:
- "Code is well-structured" — subjective
- "Implementation complete" — circular
- "Handles edge cases" — which ones?

### Step 6: Enforce the 1-3 constraint — or escalate

After planning, count the **implementation** tasks per objective. Integration testing objectives are exempt — they always have exactly 3 tasks as defined in Step 7. The mandatory quality tasks from Step 8 (ergonomic refactoring + commenting) are also exempt — they don't count toward the 1-3 limit. If any standard objective has more than 3 **implementation** tasks, the objective is too broad.

**Do not drop tasks to fit.** If you genuinely need 5 implementation tasks for an objective, those tasks represent real work. The objective must be split.

**The escalation procedure:**

1. Identify which objective has too many tasks
2. Determine how to split it into 2 objectives, each with 1-3 tasks, that preserve the original intent
3. Spawn subagents to perform the restructuring:
   - One agent to split the objective using `rex objective upsert` (creating the new objective, updating the old one, ensuring both are parented to the same milestone)
   - One agent to rewire all upstream/downstream dependencies — every objective and task that pointed to the old objective as upstream or downstream must be checked and updated. The two new objectives must be correctly sequenced relative to each other and to sibling objectives
   - After both complete, verify the dependency graph is intact: no orphaned references, no broken chains, no circular dependencies
4. Verify the parent milestone still has at most 3 objectives. If the split pushed the milestone over 3 objectives, escalate further — the milestone itself must be split. Spawn agents to split the milestone and rewire its upstream/downstream dependencies across the milestone graph
5. Re-evaluate the tasks for the now-smaller objectives — they should fit within 1-3 each

This cascade (task overflow → split objective → possible milestone split) is rare but essential. The constraint exists because plans that balloon in size produce more work than they're worth. Smaller, focused units keep agents productive.

### Step 7: Create tasks for integration testing objectives

Integration testing objectives (those with IDs matching `o-*-integration-testing`) follow a **fixed 3-task template**. Do not freestyle these tasks — use the template exactly. The 3-task structure handles the full lifecycle: write and run tests, triage failures with genuine effort and user escalation when needed, and post-resolution verification.

**All 3 tasks use the `rust-integration-testing` skill on sonnet/high.** This skill knows how to read the design integration test plan, write production-grade tests, and handle the escalation flow.

Read `rex/<project-id>/design/integration-tests.md` before creating these tasks. You need to know which CRITICAL and IMPORTANT tests fall within this objective's scope so the task descriptions can reference them specifically.

#### Task 1: Write and run integration tests

```bash
rex task upsert \
  --id t-<milestone-topic>-integ-run \
  --objective o-<milestone-topic>-integration-testing \
  --title "Write and run integration tests for <milestone-topic>" \
  --description "Read the integration test plan at rex/<project-id>/design/integration-tests.md. Implement all CRITICAL and IMPORTANT tests that cover <milestone-topic>. Use real data, real connections, and real failure modes — no mocks, no synthetic data. Mark tests with #[ignore] so they run separately from unit tests. Run the suite with 'cargo test -- --ignored' and record results. If any tests fail and you can identify the cause, fix the code and re-run. Document the final state clearly: which tests pass, which fail, and the error output for each failure." \
  --agent-model sonnet \
  --agent-effort high \
  --agent-skill rust-integration-testing \
  --add-reference rex/<project-id>/design/integration-tests.md \
  --add-output tests/integration/<milestone-topic>.rs \
  --add-checklist "c1:All CRITICAL tests from the design plan for this scope are implemented" \
  --add-checklist "c2:All IMPORTANT tests from the design plan for this scope are implemented" \
  --add-checklist "c3:Tests use real data and real connections — no mocks" \
  --add-checklist "c4:Test results documented with pass/fail status and error output for failures" \
  --add-upstream <last-task-id-from-each-sibling-objective>
```

The upstream dependencies must include the final task(s) from all non-integration-testing objectives in the same milestone — the code must exist before tests can exercise it.

#### Task 2: Triage failures and escalate if needed

```bash
rex task upsert \
  --id t-<milestone-topic>-integ-triage \
  --objective o-<milestone-topic>-integration-testing \
  --title "Triage integration test failures for <milestone-topic> — fix or escalate" \
  --description "Run all integration tests for <milestone-topic> with 'cargo test -- --ignored'. For each failing test:

STEP 1 — TRY TO FIX IT YOURSELF (MANDATORY):
You MUST make at least 3 genuine, distinct attempts to fix each failure before even considering escalation. Work through these:
  a) Re-read the error message and stack trace carefully — they usually point to the exact issue
  b) Check the production code the test exercises for bugs, logic errors, or missing error handling
  c) Check environmental setup — paths, config, test data format, working directory
  d) Search the codebase for working examples of similar patterns
  e) Try alternative approaches — different data, different setup, different assertion strategy

STEP 2 — CLASSIFY THE FAILURE:
After 3+ genuine attempts, classify each remaining failure:
  - FIXABLE: Code bugs, logic errors, incorrect assertions, missing imports, wrong config paths → fix it
  - REQUIRES USER INPUT: Missing API credentials, geo-blocked services, services behind auth you don't have, infrastructure not provisioned, rate limits you can't work around → escalate

STEP 3 — IF ALL TESTS PASS: Mark this task as complete.

STEP 4 — IF ANY FAILURES REQUIRE USER INPUT: Follow these exact steps:

  1. Write a detailed report to rex/<project-id>/user-support/requested.md:
     - Which test(s) failed and exact error messages
     - What you tried (all 3+ attempts, with what happened each time)
     - What specific action the user must take
     - File paths the user should look at

  2. Run this command:
     rex project update-status user-input not-started

  3. Say EXACTLY in your response: 'This task must remain in-progress. I have escalated to user-support. DO NOT MARK THIS TASK AS COMPLETE.'

WHEN RESUMED AFTER USER-SUPPORT: The user's response will appear in your dispatch prompt under 'User Input (from previous escalation)'. Apply the user's fixes, re-run the failing tests. If they pass now, mark this task complete. If new blockers appear, repeat the escalation." \
  --agent-model sonnet \
  --agent-effort high \
  --agent-skill rust-integration-testing \
  --add-reference rex/<project-id>/design/integration-tests.md \
  --add-output tests/integration/<milestone-topic>.rs \
  --add-checklist "c1:Every failure investigated with at least 3 distinct fix attempts" \
  --add-checklist "c2:All fixable failures resolved and tests pass" \
  --add-checklist "c3:Unresolvable failures escalated with detailed report including all attempts made" \
  --add-checklist "c4:All tests pass before marking this task complete" \
  --add-upstream t-<milestone-topic>-integ-run
```

**Critical:** Replace `<project-id>` in the description with the actual project ID from `rex project get-active`.

#### Task 3: Final verification

```bash
rex task upsert \
  --id t-<milestone-topic>-integ-verify \
  --objective o-<milestone-topic>-integration-testing \
  --title "Final verification: all integration tests for <milestone-topic> pass" \
  --description "This is the final verification step. Re-run ALL integration tests for <milestone-topic> with 'cargo test -- --ignored'. Every CRITICAL and IMPORTANT test must pass. This confirms:
  - Tests from task 1 still pass
  - Fixes from task 2 (including user-support resolutions) hold up
  - No regressions were introduced

If ALL tests pass: This task is complete.

If any tests FAIL: Follow the same process as task 2 — make 3+ genuine fix attempts, then escalate if truly stuck. Write to rex/<project-id>/user-support/requested.md, run 'rex project update-status user-input not-started', and say 'DO NOT MARK THIS TASK AS COMPLETE.' The cycle repeats until all tests pass." \
  --agent-model sonnet \
  --agent-effort high \
  --agent-skill rust-integration-testing \
  --add-reference rex/<project-id>/design/integration-tests.md \
  --add-output tests/integration/<milestone-topic>.rs \
  --add-checklist "c1:All CRITICAL integration tests pass" \
  --add-checklist "c2:All IMPORTANT integration tests pass" \
  --add-checklist "c3:cargo test -- --ignored runs clean for all tests in scope" \
  --add-upstream t-<milestone-topic>-integ-triage
```

#### Key points for the planning agent

- **The 3 tasks form a strict linear chain:** `integ-run` → `integ-triage` → `integ-verify`. Wire upstream/downstream accordingly.
- **Task 1's upstream** connects to the last task(s) in the milestone's other objectives — the code (and its ergonomic refactoring from Step 8) must exist before integration tests run.
- **Replace `<project-id>`** in task descriptions with the actual project ID from `rex project get-active`.
- **Task 2 is where escalation lives.** Its description is intentionally verbose with exact CLI commands and exact phrasing because the executing agent has no context about the rex harness — it only knows what the task description tells it.
- **Task 3 runs even if task 2 didn't escalate.** Quick re-run confirmation if everything passed, substantive verification if escalation occurred.
- **"DO NOT MARK THIS TASK AS COMPLETE"** — the operator's Step 8 checks for this. Without it, the operator marks the task complete and the rework cycle breaks.
- **The rework mechanism works natively:** When a task stays `in-progress` and user-support is activated → operator stops → next run handles user-support (user provides input) → following run: `rex task next` returns the in-progress task at Tier 0 (highest priority) → operator dispatches agent with user's input in the prompt → agent picks up where it left off.

---

### Step 8: Add mandatory quality tasks to code-producing objectives

Every standard objective that produces code (i.e., has tasks whose outputs include `.rs` files or similar code artifacts) gets two additional mandatory tasks: **ergonomic refactoring** before integration tests, and **commenting** after integration tests pass. These ensure consistent code quality across the entire project — ergonomics and commenting are never skipped or forgotten because they're baked into the plan itself.

**Skip both tasks** if the objective is purely configuration, documentation, planning, or other non-code work. The test is simple: look at the `--add-output` flags on the objective's implementation tasks. If none of them produce code files, skip this step for that objective.

These quality tasks do **not** count against the 1-3 implementation task limit. An objective may have up to 5 total tasks: 1-3 implementation + 1 ergonomics + 1 commenting.

#### Task A: Ergonomic Refactoring (BEFORE integration tests)

This task is the last one in the objective before integration tests begin. It reviews all code produced by the objective for idiomatic Rust style without changing behavior.

```bash
rex task upsert \
  --id t-<objective-topic>-ergonomics \
  --objective o-<parent-objective-id> \
  --title "Ergonomic refactoring for <objective-topic>" \
  --description "Review all code produced by this objective's tasks for idiomatic Rust style, readability, and ergonomics. Focus on type signatures, naming, module organization, unnecessary verbosity, and missed opportunities for cleaner expression. Do not change behavior — only improve how the code reads. Run cargo check and cargo test --lib after changes to ensure nothing broke." \
  --agent-model sonnet \
  --agent-effort high \
  --agent-skill rust-ergonomic-refactoring \
  --add-reference <output-files-from-objective-tasks> \
  --add-output <same-output-files> \
  --add-checklist "c1:All code produced by this objective reviewed for idiomatic Rust patterns" \
  --add-checklist "c2:No behavioral changes introduced" \
  --add-checklist "c3:cargo check passes" \
  --add-checklist "c4:cargo test --lib passes" \
  --add-upstream <last-implementation-task-id> \
  --add-downstream t-<milestone-topic>-integ-run
```

**Dependency wiring:**
- **Upstream:** The last implementation task in this objective (the one that finishes the code)
- **Downstream:** The first integration test task (`t-<milestone-topic>-integ-run`) — integration tests should run against ergonomically clean code
- **References and outputs:** Use the same output files from the objective's implementation tasks, since the refactoring modifies them in place

#### Task B: Commenting (AFTER integration tests pass)

This task runs after all integration tests pass, adding consistent documentation to the code that's now been verified as correct and well-structured.

```bash
rex task upsert \
  --id t-<objective-topic>-comments \
  --objective o-<parent-objective-id> \
  --title "Add comments to code produced by <objective-topic>" \
  --description "Add consistent, minimal comments to all files produced or significantly modified by this objective's tasks. Follow commenting conventions: module-level //! comments on every file, /// doc comments only where the name and signature don't tell the full story, minimal inline comments. When in doubt, leave it out." \
  --agent-model sonnet \
  --agent-effort high \
  --agent-skill rust-commenting \
  --add-reference <output-files-from-objective-tasks> \
  --add-output <same-output-files> \
  --add-checklist "c1:All files produced by this objective have appropriate module-level //! comments" \
  --add-checklist "c2:Public items have /// doc comments where name and signature are insufficient" \
  --add-checklist "c3:No excessive or redundant comments added" \
  --add-upstream t-<milestone-topic>-integ-verify
```

**Dependency wiring:**
- **Upstream:** The final integration test verification task (`t-<milestone-topic>-integ-verify`) — comments are added only after tests confirm the code is correct
- **References and outputs:** Same output files from the objective's implementation tasks

#### Key points for quality tasks

- **One ergonomics task and one commenting task per code-producing objective.** If a milestone has 2 code-producing objectives, that's 2 ergonomics tasks (both upstream of `integ-run`) and 2 commenting tasks (both downstream of `integ-verify`).
- **The ergonomics task replaces what the `rust-team-coordinator` would have done internally.** The coordinator no longer runs its own polish/comment phases — that responsibility has moved here to ensure it always happens.
- **Task 1 of the integration testing objective** (`integ-run`) should list ALL ergonomics tasks as upstream — not just the implementation tasks. Update the upstream list accordingly: `--add-upstream t-<obj-A>-ergonomics --add-upstream t-<obj-B>-ergonomics ...`
- **Commenting tasks can run in parallel** across objectives since they touch different files. There's no dependency between `t-<obj-A>-comments` and `t-<obj-B>-comments`.

---

## Writing the tasks using the CLI

Once you've planned all tasks (1-3 per standard objective, plus the fixed 3-task template for integration testing objectives per Step 7), write them using the rex CLI. **Do not write planning.json directly.**

### Task creation

```bash
rex task upsert \
  --id t-<objective-topic>-<action> \
  --objective o-<parent-objective-id> \
  --title "Clear, actionable description of the work" \
  --description "What to build, where it goes, what it integrates with. Include enough context for an agent to start cold." \
  --agent-model <opus|sonnet> \
  --agent-effort <high|max> \
  --agent-skill <skill-name> \
  --add-reference design/architecture.md \
  --add-reference design/modules.md \
  --add-output src/module/file.rs \
  --add-output tests/integration/test_file.rs \
  --add-checklist "c1:First completion criterion" \
  --add-checklist "c2:Second completion criterion" \
  --add-upstream t-<dependency-task-id> \
  --add-downstream t-<dependent-task-id>
```

The parent objective's `tasks` list is automatically updated.

### ID conventions

Use consistent, readable IDs that encode the hierarchy:
- `t-<objective-topic>-<action>` (e.g., `t-matching-impl`, `t-matching-tests`, `t-api-routes-wiring`)
- The objective topic prefix groups related tasks visually
- The action suffix describes what the task does

### Dependency wiring

Wire dependencies as you create tasks. Create upstream tasks first so downstream tasks can reference them.

```bash
# Task with upstream dependency
rex task upsert \
  --id t-matching-tests \
  --objective o-core-matching \
  --title "Write integration tests for order matching" \
  --description "..." \
  --add-upstream t-matching-impl

# Cross-objective dependency
rex task upsert \
  --id t-api-routes \
  --objective o-api-layer \
  --title "Wire REST endpoints to matching engine" \
  --description "..." \
  --add-upstream t-matching-impl \
  --add-upstream t-api-types
```

### References and outputs

- **References** — Point to the specific design documents an agent needs to execute this task. Don't just reference the top-level architecture doc — point to the specific section or module plan. An agent should be able to read the references and know exactly what structs to create, what function signatures to use, and where the code goes.
- **Outputs** — List the specific files this task produces. This helps downstream tasks know exactly what to expect and where to find it.

### Agent assignment

Every task must be assigned an agent configuration — the skill, model, and effort level that determines which specialist handles it and how hard they think. This is set via `--agent-model`, `--agent-effort`, and `--agent-skill` flags on `rex task upsert`.

**This is mandatory.** Every task you create must have an agent assigned. Without it, the task falls back to a generic default during execution, which wastes resources on simple tasks and under-powers complex ones.

#### Built-in Rust skills reference

Use these `rust-*` skills for standard Rust development work. **Only assign non-`rex-*` skills** — the `rex-*` skills are for the rex framework's own phases.

| Task type | Skill | Model | Effort | When to use |
|-----------|-------|-------|--------|-------------|
| Planning & architecture decisions | `rust-planning-and-architecture` | opus | max | Tasks that require choosing between approaches, designing data structures, evaluating concurrency strategies, or making significant structural decisions. **The only task type that warrants opus/max.** |
| Complex implementation (new modules, core logic, state machines, multi-file features) | `rust-team-coordinator` | sonnet | high | The default for any substantial implementation work. It triages internally and orchestrates exploration → architecture → implementation → testing → polish. |
| Integration testing | `rust-integration-testing` | sonnet | high | Writing integration tests that exercise real data flows, real connections, and real failure modes. |
| Ergonomic refactoring | `rust-ergonomic-refactoring` | sonnet | high | Cleaning up code for idiomatic style, readability, and ergonomics — especially when touching multiple files or modules. |
| Unit testing | `rust-unit-testing` | sonnet | high | Writing focused unit tests for specific functions, methods, or modules. |
| Comments & documentation | `rust-commenting` | sonnet | high | Adding or updating comments on existing code. |
| Error handling | `rust-errors-management` | sonnet | high | Defining error types, replacing unwraps, setting up thiserror-based error propagation. |
| Simple/straightforward implementation | `rust-developing` | sonnet | high | Small, well-defined tasks where the design is already decided — a single function, a derive macro addition, a config struct, wiring glue code. No design decisions needed. |
| Code exploration | `rust-exploration-and-planning` | sonnet | high | Understanding an existing codebase area before working on it. Typically an upstream of an implementation task. |

#### Custom project skills

During onboarding, custom skills may have been created for project-specific specialist work (e.g., a domain-specific skill for financial calculations, protocol parsing, etc.). Check the onboarding `skill-building.md` output for any custom skills that were created. Assign these to tasks that match their domain. Use sonnet/high for all domain work — reserve opus/max only for planning and architecture decisions.

#### How to decide

1. **Does it require planning or architecture decisions?** (choosing between approaches, designing data structures, evaluating concurrency strategies) → `rust-planning-and-architecture` on opus/max. This is the **only** task type that uses opus.
2. **Is it complex, multi-file implementation work?** → `rust-team-coordinator` on sonnet/high. It triages internally and won't over-engineer simple work.
3. **Is it a focused specialist task?** (tests, comments, error types, refactoring, exploration) → Use the matching specialist skill on sonnet/high.
4. **Does it match a custom project skill?** → Use that skill on sonnet/high.

#### Example with agent flags

```bash
# Complex implementation — rust-team-coordinator on sonnet/high
rex task upsert \
  --id t-matching-impl \
  --objective o-core-matching \
  --title "Implement the OrderBook with insert, cancel, and match methods" \
  --description "..." \
  --agent-model sonnet \
  --agent-effort high \
  --agent-skill rust-team-coordinator \
  --add-reference design/architecture.md \
  --add-output src/matching/orderbook.rs

# Simple commenting task — rust-commenting on sonnet/high
rex task upsert \
  --id t-matching-comments \
  --objective o-core-matching \
  --title "Add comments to the matching engine module" \
  --description "..." \
  --agent-model sonnet \
  --agent-effort high \
  --agent-skill rust-commenting \
  --add-reference src/matching/orderbook.rs \
  --add-output src/matching/orderbook.rs \
  --add-upstream t-matching-impl

# Integration tests — rust-integration-testing on sonnet/high
rex task upsert \
  --id t-api-integration-tests \
  --objective o-api-layer \
  --title "Write integration tests for order submission through the REST API" \
  --description "..." \
  --agent-model sonnet \
  --agent-effort high \
  --agent-skill rust-integration-testing \
  --add-reference design/integration-tests.md \
  --add-output tests/integration/order_api_test.rs \
  --add-upstream t-api-routes
```

---

## Writing output files

If you were given an output file path, write a summary document:

```markdown
# Task Plan

**Date:** YYYY-MM-DD

## Overview
Brief description of the task structure and dependency strategy.

## Tasks by Objective

### Objective: [Title] (`o-id`) — Milestone: [Title] (`m-id`)

#### 1. [Task Title] (`t-id`)
**Description:** What this task involves
**Agent:** skill-name / model / effort
**Upstream:** t-dependency-1, t-dependency-2 (or "none — root task")
**Downstream:** t-dependent-1
**Outputs:** src/module/file.rs
**Checklist:**
- [ ] Completion criterion 1
- [ ] Completion criterion 2

(Repeat for each task)

(Repeat for each objective)

## Dependency Graph
A textual or mermaid representation of the full task dependency chain, showing which tasks can run in parallel and which are serial.

## Execution Order
The recommended order for agents to pick up tasks, based on the dependency graph. Group tasks that can run in parallel.

## Key Decisions
Why tasks were scoped this way, notable dependency choices, and any escalation decisions that were made.
```

---

## What done looks like

You're done when:
1. Every standard objective has 1-3 implementation tasks (if any needed more, it was split first). Every integration testing objective has exactly 3 tasks following the template from Step 7.
2. Every code-producing objective has its two mandatory quality tasks from Step 8: an ergonomic refactoring task (before integration tests) and a commenting task (after integration tests)
3. All tasks have been created via the CLI using `rex task upsert`
4. Every task has its upstream and downstream dependencies explicitly wired — no implicit dependencies
5. Each task has a meaningful checklist with concrete, verifiable completion criteria
6. Each task's references point to the specific design documents an agent needs for cold-start execution
7. Each task's outputs list the specific files it will produce
8. Every task has an agent assigned via `--agent-model`, `--agent-effort`, and `--agent-skill` — no task left without an agent config
9. The dependency graph has been verified: no orphans, no broken chains, no missing upstreams. Specifically: ergonomics tasks are upstream of `integ-run`, and commenting tasks are downstream of `integ-verify`
10. Any requested output files have been written
