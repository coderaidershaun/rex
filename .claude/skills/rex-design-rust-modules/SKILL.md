---
name: rex-design-rust-modules
description: Plan the complete module layout (folders, files, and their responsibilities) for a Rust project during the rex design phase. Use this skill when the rex design process reaches the "modules" step, when a project needs its file and folder structure planned before implementation begins, or when the user says things like "plan the modules", "design the file structure", "figure out the layout", "what files do we need", or "organize the codebase." This skill reads all available onboarding and design inputs, thinks deeply about what the project needs, and produces a complete module plan — every folder, every file, what each module does, and what it produces. Every project gets an errors.rs and a tests/integration/ directory. No module exceeds 500 lines excluding unit tests.
disable-model-invocation: false
user-invocable: false
---

# Design: Rust Module Layout

You plan the complete file and folder structure for a Rust project. Your output is the blueprint that implementation agents follow to know where every piece of code belongs — what files to create, what each module is responsible for, and how modules group together into coherent subsystems.

This is a thinking-intensive task. The module layout defines the project's cognitive structure — how developers (and agents) think about the system. A good layout makes the right code easy to find and the wrong code hard to write. A bad layout creates confusion, duplication, and files that grow into 2000-line monsters because nobody knew where else to put things.

You'll be told where to write the output (a file path like `design/modules.md`) and given input files to read for context. Read them all first. Then think hard about the right structure and write the plan to the output path.

---

## Reading the inputs

Read everything available. The more context you absorb, the better your module decisions will be.

### From goal and scope
- What the project does — this determines the top-level domain modules
- What's in scope — modules should only be planned for features that are in scope
- Who it's for — a library has different module concerns than a CLI or a service

### From existing-code-exploration (if present)
- The current module structure — if refactoring, you need to know what exists and how it's organized
- What works and what doesn't about the current layout
- Critical logic locations — these constrain where modules can be split

### From error-handling plan (if present)
- How many error types are planned — this affects whether you need one `errors.rs` or error files in each subsystem module
- Error hierarchy — if errors are split by subsystem, the module layout needs to reflect that

### From library review (if present)
- Which crates are being used — some crates influence module organization (e.g., `axum` handlers in a `routes/` module, `clap` commands in a `cli/` module)
- Integration patterns — how crates expect to be wired together

### From libraries-and-sdks
- Confirmed dependencies — each major dependency often implies a module (database access, HTTP client, serialization layer)

### From known risks and success measures
- Testability requirements — if specific behaviors need integration tests, plan modules that are testable at those boundaries

---

## Module planning principles

### The 500-line rule

No source file should exceed 500 lines of code, excluding unit tests (the `#[cfg(test)] mod tests` block at the bottom). This isn't arbitrary — it's the practical limit for an agent or developer to hold a file's logic in their head at once. Beyond 500 lines, functions start doing too many things, responsibilities blur, and changes in one part of the file break things in another.

When estimating whether a module will stay under 500 lines, think about:
- How many public functions/methods it needs
- How many types it defines
- The complexity of the logic — algorithmic code is denser than glue code
- Whether there are large match statements or mapping tables

If a module is likely to exceed 500 lines, split it. Common split strategies:
- **By type** — move each major struct/enum into its own file within a subfolder
- **By operation** — separate reads from writes, parsing from validation, construction from use
- **By domain concept** — split `orders.rs` into `order_validation.rs`, `order_matching.rs`, `order_book.rs`

### Related modules belong together

Modules that work closely together — calling each other's functions, sharing types, forming a logical subsystem — should live in the same folder. This makes dependencies visible in the directory structure.

```
src/
├── matching/          # subsystem: everything about order matching
│   ├── mod.rs         # re-exports, subsystem-level docs
│   ├── engine.rs      # the matching algorithm
│   ├── order_book.rs  # order book data structure
│   └── fills.rs       # fill generation and reporting
├── market_data/       # subsystem: everything about market data
│   ├── mod.rs
│   ├── feed.rs        # real-time data feed handling
│   └── snapshot.rs    # point-in-time snapshots
```

Not like this:
```
src/
├── engine.rs          # matching engine (which matching? unclear)
├── order_book.rs      # order book (related to engine? maybe?)
├── fills.rs           # fills (related to... something)
├── feed.rs            # feed (what kind of feed?)
├── snapshot.rs        # snapshot (of what?)
```

### The mod.rs contract

Every folder gets a `mod.rs` that serves as the module's public API. It:
- Declares submodules (`mod engine;`)
- Re-exports the types and functions that other modules should use (`pub use engine::MatchingEngine;`)
- Contains module-level documentation explaining what this subsystem does
- Does NOT contain substantial logic — if `mod.rs` is doing work, that work should be in a submodule

### Required structural elements

#### errors.rs

Every project gets an `errors.rs` at the crate root (or within each major subsystem if the error plan calls for multiple error types). This file houses the `thiserror`-derived error enums and result type aliases. It's the canonical home for error definitions — not scattered across modules.

If the error-handling design input specifies a hierarchy (e.g., `AppError` wrapping `NetworkError` and `ParseError`), each subsystem gets its own error file:
```
src/
├── errors.rs              # top-level AppError, wraps subsystem errors
├── network/
│   ├── errors.rs          # NetworkError
│   └── ...
├── parser/
│   ├── errors.rs          # ParseError
│   └── ...
```

#### tests/integration/

Every project gets a `tests/` directory at the crate root for integration tests, unless the inputs explicitly indicate the project won't have integration tests (rare — most projects benefit from them). Structure:

```
tests/
├── integration/
│   ├── mod.rs             # shared test utilities, fixtures
│   ├── test_[feature].rs  # one file per test scenario or feature area
│   └── helpers/           # test helpers if needed
│       └── mod.rs
```

Integration test files correspond to the project's major features, not its internal modules. An integration test for "user login flow" might exercise code across 5 internal modules — the test is organized by behavior, not by implementation.

---

## How to think about modules

### Start from the domain, not the framework

Don't organize by technical layer first (`models/`, `services/`, `controllers/`). Organize by domain concept first, then layer within each concept if needed.

**Domain-first** (preferred):
```
src/
├── orders/        # everything about orders
├── inventory/     # everything about inventory
├── pricing/       # everything about pricing
```

**Layer-first** (avoid unless the project is genuinely layer-oriented):
```
src/
├── models/        # all data types from every domain
├── services/      # all business logic from every domain
├── handlers/      # all request handlers from every domain
```

The domain-first approach keeps related code together. When you need to understand how orders work, you look in `orders/`. When you need to change order behavior, you change files in `orders/`. With layer-first, understanding orders requires reading files scattered across three directories.

Exception: some project types genuinely are organized by technical concern — a CLI with `commands/`, a web framework with `routes/`, a data pipeline with `stages/`. If the project's primary abstraction is technical rather than domain-based, follow that. The goal is coherence, not dogma.

### Think about what each module produces

For each module, ask: what is the tangible output of this code? Not "what does it do" in abstract terms, but what does calling into this module give you?

- `parser/` → produces structured data from raw input (e.g., `Config` from a TOML string)
- `engine/` → produces execution results from commands (e.g., `TradeResult` from an `Order`)
- `reporter/` → produces formatted output from data (e.g., a CSV string from a `Vec<Trade>`)
- `validator/` → produces validated versions of input types (e.g., `ValidatedOrder` from `RawOrder`)

This "what does it produce?" framing prevents modules from becoming grab-bags of loosely related functions. If you can't describe what a module produces, it probably shouldn't be a module — its contents belong somewhere else.

### Think about dependency direction

Modules should depend in one direction — from high-level orchestration toward low-level details. If `orders/` depends on `pricing/` and `pricing/` depends on `orders/`, something is wrong. Circular dependencies create compilation issues in Rust and always indicate confused responsibilities.

Plan the dependency direction explicitly:
```
main.rs → cli/ → orchestrator/ → [domain modules] → [infrastructure modules]
```

Where:
- **Orchestration modules** (entry points, CLI, API handlers) depend on everything
- **Domain modules** (business logic, algorithms) depend on types and infrastructure
- **Infrastructure modules** (database, HTTP, file I/O) depend on nothing project-specific
- **Types/models** are depended on by everyone, depend on nothing

### Think about growth

A module that's 200 lines today might be 800 lines in three months. Think about where growth is likely and pre-split:
- If the project scope mentions "we'll add more commands later" → plan the `commands/` folder with clear conventions for adding new files
- If there are multiple data sources → plan separate modules per source even if they're small now
- If the error plan has many variants → consider splitting errors by subsystem early

But don't over-split. A 50-line file that exists only because "it might grow someday" adds cognitive overhead for no benefit. Split when you can see the growth path from the inputs, not speculatively.

---

## Writing the output

```markdown
# Module Layout Plan

**Date:** YYYY-MM-DD

## Overview
Brief description of the planned structure — how many modules, what organizational principle (domain-first, feature-based, etc.), and why this structure fits the project.

## Directory Tree

```
project-name/
├── Cargo.toml
├── src/
│   ├── main.rs (or lib.rs)
│   ├── errors.rs
│   ├── [module]/
│   │   ├── mod.rs
│   │   ├── [submodule].rs
│   │   └── ...
│   └── ...
├── tests/
│   └── integration/
│       ├── mod.rs
│       └── test_[feature].rs
└── ...
```

## Module Specifications

### `src/main.rs` (or `src/lib.rs`)
**Responsibility:** [what this entry point does]
**Estimated size:** ~X lines
**Depends on:** [which modules it imports]
**Produces:** [what calling this gives you — e.g., "program entry, CLI dispatch"]

### `src/errors.rs`
**Responsibility:** Crate-level error types and result aliases using thiserror
**Estimated size:** ~X lines
**Depends on:** subsystem error types (if hierarchical)
**Produces:** `AppError` enum, `AppResult<T>` type alias

### `src/[module]/mod.rs`
**Responsibility:** [subsystem description]
**Re-exports:** [public types and functions from submodules]
**Submodules:**

#### `src/[module]/[submodule].rs`
**Responsibility:** [specific job within the subsystem]
**Estimated size:** ~X lines
**Key types:** [structs, enums, traits defined here]
**Key functions:** [primary public functions]
**Depends on:** [other modules/submodules it imports]
**Produces:** [tangible output — what you get from calling into this module]

(Repeat for every file in the project)

### `tests/integration/`
**Test files planned:**
- `test_[feature].rs` — [what this test file covers]
- ...

## Dependency Map
Which modules depend on which, and in what direction. This should be a clean DAG (directed acyclic graph) — if it isn't, explain why.

```
main.rs
  → cli/
    → [orchestration modules]
      → [domain modules]
        → [infrastructure modules]
          → errors.rs (used by all)
```

## Conventions
Rules for implementation agents to follow when adding code to this structure:
- How to add a new module (where it goes, what to update)
- How to add a new error variant (which errors.rs, what pattern)
- How to add a new integration test (naming, location, shared fixtures)
- Maximum file size rule (500 lines excluding unit tests)

## Design Rationale
Why this structure was chosen over alternatives. What the inputs revealed about the project that drove specific decisions. Trade-offs that were considered.
```

Write to the output path you were given (relative to the project's rex directory).
