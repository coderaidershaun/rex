---
name: rex-design-rust-existing-code-exploration
description: Perform a deep, forensic exploration of an existing Rust codebase during the rex design phase — producing a structured breakdown that captures not just architecture but the critical logic, invariants, and subtle behaviors that LLMs typically miss. Use this skill when the rex design process reaches the "existing-code-exploration" step, when a codebase needs to be thoroughly understood before refactoring or extending, or when the user says things like "analyze the existing code", "understand how this works", "map out the codebase", "explore the code before we start", or "figure out what this code actually does." This skill goes far beyond surface-level structure — it traces every important code path, documents hidden invariants, captures ordering dependencies, and produces mermaid diagrams showing exactly how everything connects. If important logic is missed, implementation agents will build on a false understanding and the project fails.
disable-model-invocation: false
user-invocable: false
---

# Design: Existing Code Exploration

You perform a forensic analysis of an existing Rust codebase. Your job is to produce a reference document so thorough that any agent reading it can implement changes to this codebase without introducing regressions, violating invariants, or misunderstanding how things actually work.

This is the most important design step when working with existing code. Every downstream agent — architecture, error handling, implementation, testing — depends on your output being complete and accurate. If you miss critical logic, those agents build on a false foundation. The project doesn't just lose time; it produces subtly wrong code that passes tests but fails in production.

You'll be told where to write the output (a file path like `design/existing-code-exploration.md`) and given input files to read for context. Read them all first. Then explore the codebase exhaustively and write your findings to the output path.

---

## Reading the inputs

### From existing-code (primary input)
- Where the code lives (paths, repositories)
- What language/framework it uses
- Current state (working, broken, prototype, production)
- What's being refactored and why
- What must be preserved (APIs, contracts, behaviors)

### From goal and scope
- What the project intends to do with this code — are we refactoring it, extending it, wrapping it, replacing parts of it?
- What's in scope vs out — don't spend time analyzing modules that won't be touched

### From libraries-and-sdks
- What dependencies the existing code uses — you'll need to understand how they're integrated

### From user-expertise
- Domain context that explains *why* the code does certain things — business rules, regulatory requirements, performance constraints

---

## The three-pass exploration

You explore in three passes, each at a different resolution. Don't skip passes or combine them — each one catches things the others miss.

### Pass 1: The big picture

Read the codebase at the structural level. Understand what it is, how it's organized, and what the key abstractions are.

**What to examine:**
- `Cargo.toml` — workspace structure, dependencies, feature flags
- `src/lib.rs` or `src/main.rs` — module hierarchy, re-exports, entry points
- Directory layout — how code is organized (by domain? by layer? by feature?)
- README, docs, comments at module level — stated intentions vs actual structure

**What to produce:**
- A plain-language summary of what the codebase does and how it's organized
- The key abstractions — the 5-10 types/traits/modules that define how the system thinks
- The execution flow — how data enters the system, what happens to it, how it exits

**Use /mermaid-diagrams to create:**
- A **module dependency diagram** (flowchart) showing which modules depend on which
- A **data flow diagram** (sequence diagram or flowchart) showing how data moves through the system from entry to exit

### Pass 2: The connections

Trace how the key abstractions from Pass 1 connect to each other. This is where you map the relationships that make the system work as a whole, not as isolated modules.

**What to examine:**
- Trait implementations — which types implement which traits, and what behavior that gives them
- Type conversions — `From`/`Into` impls, `.into()` chains, serialization boundaries
- Error propagation — how errors flow upward, what context gets added or lost at each layer
- State management — where mutable state lives, who can modify it, what locks or patterns protect it
- Cross-module function calls — the actual call graph between modules (not just the dependency graph)

**What to produce:**
- A detailed map of how types relate to each other — not just "A uses B" but "A creates B via `B::from_raw()`, passes it to C's `process()` method, which returns a `Result<D, CError>` that A converts to `AError::Processing` via `map_err`"
- Trait-implementation inventory: every trait and every type that implements it, with file locations

**Use /mermaid-diagrams to create:**
- A **class diagram** showing the key types, their fields, their trait implementations, and their relationships
- A **sequence diagram** for the primary execution path — the most important thing the code does, traced call-by-call

### Pass 3: The critical details

This is the pass that separates useful documentation from dangerous documentation. Here you look for the things that an LLM scanning the code would typically miss — the subtle logic that makes the system actually work correctly.

**What LLMs typically miss (and you must not):**

#### Ordering dependencies
Code that must execute in a specific order but where nothing in the type system enforces it:
- Initialization sequences — "the connection pool must be created before the worker threads start"
- Cleanup sequences — "channels must be closed before the runtime shuts down"
- Processing pipelines — "validation must happen before normalization because normalization assumes valid input"

#### Implicit invariants
Correctness properties that are maintained by convention, not by the type system:
- "This vec is always sorted after insertion" (but nothing prevents unsorted access)
- "This field is None before init and always Some after" (but it's not encoded as a state machine)
- "These two maps always have the same keys" (but they're separate data structures)
- "This counter never exceeds MAX_CONNECTIONS" (but it's a plain u32, not a bounded type)

#### Hidden side effects
Functions that do more than their signature suggests:
- A `get_*` method that also updates a cache or timestamp
- A `validate()` that also normalizes the input
- A `Drop` implementation that sends a network request or writes to disk
- A `Clone` implementation that's not just a bitwise copy

#### Numeric precision and boundary behavior
- Integer overflow handling — is it wrapping, saturating, or will it panic?
- Floating-point comparisons — are they using epsilon checks or exact equality?
- Boundary conditions — what happens at 0, at MAX, at empty collections?
- Index arithmetic — off-by-one patterns, inclusive vs exclusive ranges

#### Concurrency patterns
- Which data is shared across threads and how it's protected
- Lock ordering conventions (to prevent deadlocks)
- Channel usage patterns — bounded vs unbounded, what happens when full
- Atomic operation ordering — `Relaxed` vs `Acquire`/`Release` vs `SeqCst` and why

#### Error handling subtleties
- Errors that are silently swallowed (`let _ = ...` or `.ok()`)
- Error recovery that changes system state (partial rollbacks)
- Panics hidden in `.unwrap()` or `.expect()` in production paths
- Error messages that carry security-sensitive information

#### Configuration and magic values
- Hardcoded constants with domain significance ("why is this 42?")
- Configuration defaults that affect behavior in non-obvious ways
- Feature flags that change code paths substantially
- Environment variable dependencies

#### Unsafe code
- Every `unsafe` block — what invariant it relies on and what could violate it
- FFI boundaries — what assumptions are made about external code
- Raw pointer usage — lifetime and aliasing assumptions

**For each critical detail found, document:**
1. **What** — the specific code and where it is (file:line)
2. **Why it matters** — what breaks if this is violated or changed
3. **How to preserve it** — what an implementation agent needs to do (or avoid) to keep this working

---

## File inventory

As you explore, maintain a complete list of every source file you examined. This serves two purposes:
1. It proves coverage — reviewers can check whether important files were missed
2. It gives implementation agents direct links to the code

For each file, note:
- Path
- What it contains (one line)
- Whether it contains critical logic from Pass 3

---

## Writing the output

The output must be structured so that an agent can read only the sections relevant to their task while still getting the full picture. Use clear headings and cross-references.

```markdown
# Existing Code Exploration

**Date:** YYYY-MM-DD
**Codebase:** [name/path]
**Scope:** [what was analyzed and why]

## Executive Summary
2-3 paragraphs: what this codebase does, how it's organized, and the most important things to know before touching it. An agent reading only this section should understand the system well enough to ask the right questions.

## Architecture Overview

### What the system does
Plain-language description of the codebase's purpose and behavior.

### Key abstractions
The core types, traits, and modules that define the system's mental model — what they are, why they exist, and how they relate.

### Module structure
How code is organized, what each module is responsible for, dependency directions.

### Diagrams

#### Module Dependencies
```mermaid
[flowchart showing module relationships]
```

#### Data Flow
```mermaid
[sequence/flowchart showing how data moves through the system]
```

#### Type Relationships
```mermaid
[class diagram showing key types, traits, and their connections]
```

#### Primary Execution Path
```mermaid
[sequence diagram tracing the main operation call-by-call]
```

## Detailed Module Breakdown

### [module-name]
**Files:** `path/to/file.rs` (lines X-Y)
**Purpose:** what this module does
**Key types:** the important structs/enums/traits defined here
**Key functions:** the important functions and what they do
**Dependencies:** what this module imports and from where
**Consumers:** who uses this module

(Repeat for each module in scope)

## Critical Logic

This section is the most important part of this document. Every item here represents logic that, if misunderstood or violated during implementation, will cause subtle bugs.

### Ordering Dependencies
For each:
- **What:** [description]
- **Where:** `file:line`
- **Why it matters:** [what breaks if order changes]
- **Preservation:** [what to do about it]

### Implicit Invariants
For each:
- **What:** [the invariant]
- **Where:** [where it's established/maintained]
- **Why it matters:** [what assumes this invariant holds]
- **Preservation:** [how to keep it intact]

### Hidden Side Effects
For each:
- **What:** [the function and its hidden behavior]
- **Where:** `file:line`
- **Why it matters:** [what depends on this side effect]
- **Preservation:** [what to watch for]

### Numeric and Boundary Behavior
For each:
- **What:** [the specific behavior]
- **Where:** `file:line`
- **Why it matters:** [what happens at boundaries]

### Concurrency Patterns
For each:
- **What:** [the pattern]
- **Where:** [relevant code locations]
- **Why it matters:** [what breaks if violated]

### Error Handling Notes
For each:
- **What:** [the specific behavior]
- **Where:** `file:line`
- **Why it matters:** [consequence of changing it]

### Magic Values and Configuration
For each:
- **What:** [the value and its meaning]
- **Where:** `file:line`
- **Why this value:** [domain reasoning]

### Unsafe Code
For each block:
- **What:** [what the unsafe code does]
- **Where:** `file:line`
- **Invariant relied upon:** [what must be true]
- **What could violate it:** [scenarios to watch for]

## Conventions and Patterns
Patterns that the existing code follows consistently. Implementation agents should follow these unless explicitly decided otherwise during architecture.

- **Naming:** [conventions observed]
- **Error handling:** [patterns used]
- **Testing:** [how tests are structured]
- **Visibility:** [pub vs pub(crate) vs private patterns]
- **Module organization:** [how new code fits in]

## Files Reviewed

| File | Contents | Critical Logic? |
|------|----------|----------------|
| `src/lib.rs` | Module declarations, re-exports | No |
| `src/engine/matching.rs` | Order matching algorithm | Yes — ordering deps, invariants |
| ... | ... | ... |

## Open Questions
Anything discovered during exploration that needs human clarification before implementation can proceed — ambiguous logic, apparent bugs, code that contradicts the stated goal.
```

Write to the output path you were given (relative to the project's rex directory).

---

## Exploration discipline

### Read everything in scope
Don't sample. Don't skim. Read every file that's in scope. If the codebase has 20 source files, read all 20. If it has 200, read the ones within the scope defined by the inputs — but read them completely, not just the first 50 lines.

The cost of reading a file you didn't need is a few seconds of context. The cost of missing a file that contained a critical invariant is a broken implementation.

### Follow every thread
When you see a function call, trait usage, or type conversion you don't fully understand, trace it to its definition. When you see an invariant comment, verify it by reading the code that maintains it. When you see an `unsafe` block, read the surrounding context to understand what guarantees it depends on.

### Be honest about uncertainty
If you can't determine why code does something, say so in the Open Questions section. Don't invent an explanation. An honest "I don't know why this constant is 42" is infinitely more useful than a confident wrong explanation that leads an implementation agent to change it.

### Verify your diagrams
After creating each mermaid diagram, re-read the code it represents and check that the diagram is accurate. A wrong diagram is worse than no diagram — it actively misleads every agent that reads it.
