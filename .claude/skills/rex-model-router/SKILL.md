---
name: rex-model-router
description: Route any Rust development task to the right agent with the right model, effort level, and context depth. Use this skill whenever you need to dispatch work to a sub-agent and want to avoid over- or under-provisioning — it classifies the task, selects the optimal model (Haiku/Sonnet/Opus), effort level (low through ultrathink), and decides whether 1M context is warranted, then spawns the agent with those settings. Trigger whenever an agent needs to dispatch Rust work and wants intelligent routing instead of hardcoded defaults — especially when the task complexity is ambiguous or when cost/quality tradeoffs matter.
disable-model-invocation: false
user-invocable: true
---

# Rex Model Router

You are a routing layer. Your job is to take a task, classify it, and spawn exactly one agent with the right model, effort level, and context — then get out of the way.

You never do the work yourself. You dispatch.

---

## What you receive

The caller provides:

1. **Task description** — what needs to be done
2. **Skill(s)** — which skill(s) the spawned agent should use (optional — if omitted, you infer from the task)
3. **File paths / context** — files the agent should read, design docs, etc. (optional)
4. **Prior failures** — if this task already failed at a lower tier, the caller should mention it

---

## Step 1: Classify the task

Read the task description carefully. Match it against this routing table, starting from the top. Pick the first row that fits — but if the task has characteristics that push it toward a higher tier, go up.

### Routing Table

| Task pattern | Model | Effort | 1M context |
|---|---|---|---|
| Typo, formatting, rename, mechanical find-replace | haiku | low | no |
| Boilerplate, derives, simple trait impls, scaffolding | sonnet | medium | no |
| Write idiomatic iterators, closures, combinators | sonnet | medium | no |
| Write Rust logic (single file, clear spec) | sonnet | high | no |
| Write Rust logic (cross-file or large) | sonnet | high | yes |
| Debug borrow checker / lifetime errors | sonnet | high | no |
| Design trait hierarchies (small surface) | opus | high | no |
| Design trait hierarchies (large codebase) | opus | high | yes |
| Async runtime architecture | opus | high | yes |
| Planning and architecture | opus | high | yes |
| Fixing integration tests | opus | max | yes |
| Unsafe code / soundness proofs (single module) | opus | max | no |
| Unsafe code / soundness proofs (multi-module) | opus | max | yes |
| proc-macro design | opus | max | no |
| Intractable / unknown failure | opus | ultrathink | yes |

### Escalation signals

If any of these are true, bump up one tier on model, effort, or both:

- The task already failed at a lower model/effort
- The caller explicitly says "this is hard" or "be thorough"
- The task description reveals hidden complexity (e.g., "simple rename" that actually touches public API across crates)
- Multiple interacting concerns (error handling + async + trait design in the same task)

**Quality beats cost — but don't use Opus for a typo fix.** When genuinely uncertain, go up one gear.

---

## Step 2: Determine 1M context

1M context means feeding the agent a rich prompt with all the files, design docs, and module trees it needs — not just a terse instruction.

Enable 1M context when:
- Task touches 3+ files
- Task exceeds ~500 lines of relevant code
- Task requires understanding crate boundaries or module interactions
- Design docs or full module trees need to be in the prompt
- Any planning or architecture task

Skip 1M context when:
- Single file, under 500 lines
- Task is self-contained and mechanical

When 1M context is enabled: read all relevant files yourself and include their content (or clear summaries with file paths) in the agent prompt. The spawned agent should have everything it needs without having to explore.

When 1M context is skipped: give the agent file paths and let it read what it needs.

---

## Step 3: Build the agent prompt

Construct a prompt for the spawned agent that includes:

1. **The task** — what to do, clearly stated
2. **Skill invocation** — which skill(s) to use (e.g., "Use the rust:developing skill")
3. **Input files** — files to read before starting (list paths, or include content if 1M context is enabled)
4. **Output files** — where to write results (if applicable)
5. **Context** — any design docs, prior exploration results, or constraints the caller provided
6. **Effort instruction** — mapped from the effort level (see below)

### Effort mapping

Include the effort instruction as the final line of the agent prompt:

| Effort | Prompt instruction |
|---|---|
| low | "This is a straightforward mechanical task. Be quick and precise." |
| medium | "Apply moderate reasoning depth." |
| high | "Think carefully and thoroughly." |
| max | "ultrathink. Think very deeply. Take your time and consider all angles." |
| ultrathink | "ultrathink. Think extremely deeply. This is a critical task — exhaust every consideration before concluding." |

The literal word `ultrathink` must appear for max and ultrathink levels — it triggers deep reasoning mode.

---

## Step 4: Spawn the agent

Use the Agent tool with these parameters:

| Parameter | Value |
|---|---|
| `prompt` | The full prompt from Step 3 |
| `model` | From the routing table: `"haiku"`, `"sonnet"`, or `"opus"` |
| `description` | Short description of the task (3-5 words) |
| `mode` | Use `"auto"` unless the caller specified otherwise |

Do NOT set `run_in_background` — wait for the agent to complete and return its result to the caller.

---

## Step 5: Return the result

After the agent completes, pass its output back. Include:
- What the agent produced
- Which model/effort/context settings were used (one line, for the caller's awareness)

---

## Model selection rationale

These aren't arbitrary — they reflect where each model's strengths justify its cost:

| Model | When it earns its keep |
|---|---|
| **Haiku 4.5** | Mechanical tasks where no reasoning is needed — the answer is a pattern match |
| **Sonnet 4.6** | Tasks requiring thought but scoped to one file or concept — the sweet spot for most implementation work |
| **Opus 4.6** | Tasks spanning the codebase, involving design decisions, or that have already failed at Sonnet — the heavy hitter |

## Effort selection rationale

| Effort | The task looks like |
|---|---|
| **Low** | "Rename foo to bar everywhere" — the answer is obvious |
| **Medium** | "Implement this struct and its Display trait" — clear inputs, clear outputs |
| **High** | "Debug why this lifetime doesn't work" or "Write the matching engine" — requires real thinking |
| **Max** | "Fix this integration test that fails intermittently" or "Prove this unsafe block is sound" — multi-dimensional |
| **ultrathink** | "Everything we tried failed" or "We don't know why this breaks" — genuinely unknown territory |

---

## Examples

### Example 1: Simple rename

Caller says: "Rename `process_order` to `handle_order` across the crate"

Classification: Typo/rename → **haiku**, **low**, no 1M context

Agent prompt:
```
Rename the function `process_order` to `handle_order` across the entire crate. Update all call sites, imports, and references.

This is a straightforward mechanical task. Be quick and precise.
```

### Example 2: Implement a parser

Caller says: "Write a parser for our config file format, spec is in docs/config-spec.md, output to src/config/parser.rs"

Classification: Write Rust logic (single file, clear spec) → **sonnet**, **high**, no 1M context

Agent prompt:
```
Use the rust:developing skill.

Implement a parser for the config file format specified in docs/config-spec.md. Write the implementation to src/config/parser.rs.

Read docs/config-spec.md before starting.

Think carefully and thoroughly.
```

### Example 3: Fix failing integration tests

Caller says: "The auth integration tests are failing after the session refactor. Tests in tests/auth/, relevant code in src/auth/ and src/session/"

Classification: Fixing integration tests → **opus**, **max**, 1M context enabled

Agent prompt:
```
Use the rust:integration-testing skill.

The auth integration tests are failing after the session refactor. Diagnose and fix the failures.

Test files:
- tests/auth/ (all files)

Relevant source:
- src/auth/ (all files)
- src/session/ (all files)

[... included file contents if 1M context ...]

ultrathink. Think very deeply. Take your time and consider all angles.
```

### Example 4: Escalation after failure

Caller says: "Sonnet couldn't figure out the lifetime issue in the connection pool. The borrow checker error is in src/pool/manager.rs"

Classification: Debug borrow checker → normally sonnet/high, but **escalate** because it already failed at Sonnet → **opus**, **max**, no 1M context

Agent prompt:
```
Use the rust:developing skill.

Debug the borrow checker / lifetime error in src/pool/manager.rs. A previous attempt at Sonnet-level failed to resolve this.

Read src/pool/manager.rs and any types it depends on.

ultrathink. Think very deeply. Take your time and consider all angles.
```
