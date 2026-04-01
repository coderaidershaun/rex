---
name: rex-design-rust-errors
description: Design a comprehensive error handling plan for a Rust project during the rex design phase. Use this skill when the rex design process reaches the "error-handling" step, when an error management strategy needs to be planned before implementation begins, or when the user wants to design how errors should flow through their Rust project. Also trigger when the user says things like "plan the errors", "design error handling", "how should errors work", or "figure out the error types." This skill reads onboarding inputs (goal, scope, libraries, existing code) and produces a concrete error handling plan — error types, variants, propagation strategy, and crate choices — written to the output path.
disable-model-invocation: false
user-invocable: false
---

# Design: Rust Error Handling Plan

You produce a concrete error handling plan for a Rust project. You read the onboarding inputs to understand what's being built, what libraries are involved, and whether there's existing code — then you design the error types, propagation strategy, and file layout that the implementation agents will follow.

You'll be told where to write the output (a file path like `design/error-handling.md`) and given input files to read for context. Read them all before designing anything. Then write the plan to the output path.

This is a first pass — you're establishing the error architecture that implementation agents will build from. Get the structure right; the exact variant names and messages can evolve during implementation.

---

## Your default position: thiserror

You use `thiserror` for error types in virtually every Rust project. It derives `std::error::Error` and `Display` with minimal boilerplate while keeping errors as typed enums — not opaque strings.

`anyhow` has a narrow role: quick prototypes, one-off scripts, or application-level code where the caller never needs to match on specific error variants. If the project is a library, a service with structured error responses, or anything where callers need to distinguish failure modes — `thiserror` is the answer. Don't reach for `anyhow` just because it's "easier." The cost of losing type information compounds as the project grows.

When you see both `thiserror` and `anyhow` together: `thiserror` for defining error types, `anyhow` for the top-level application glue (e.g., `main()` returns `anyhow::Result`). This is a legitimate pattern for CLIs and binaries, but only recommend it when the project genuinely benefits from `anyhow`'s ergonomics at the outermost layer.

---

## Reading the inputs

Before designing anything, read every input file you're given. You're looking for:

### From the goal and scope
- What the project does — this tells you what failure domains exist (network? parsing? filesystem? user input? domain validation?)
- Who it's for — a library needs public error types that callers can match on; a CLI can be more relaxed
- What's in scope vs out — don't design error types for features that aren't being built

### From libraries and SDKs
- Which crates are being used — each brings its own error types that need to integrate with yours
- `tokio` / `async-std` → async error propagation considerations
- `serde` / `serde_json` / `toml` → deserialization error wrapping
- `reqwest` / `hyper` → network error types
- `sqlx` / `diesel` / `rusqlite` → database error types
- `clap` → CLI argument validation (usually handled by clap itself, but custom validation errors may be needed)
- `tracing` / `log` → how errors get logged (not error types per se, but affects what context to carry)

### From existing code (if refactoring)
- What error handling exists today — unwraps, string errors, bare panics, existing error enums
- Whether `thiserror` is already in use — if yes, you're extending; if no, you're introducing it
- How bad the current situation is — this determines whether error handling is a standalone workstream or woven into other implementation tasks

### From known risks and success measures
- Reliability expectations — high-reliability systems need richer error context
- Debugging requirements — if diagnosability is a stated concern, lean toward more context in error variants

---

## Designing the error plan

### Step 1: Identify the failure domains

Every project has a set of things that can go wrong. Group them into domains:

- **I/O** — file operations, network calls, database queries
- **Parsing** — deserialization, format conversion, data extraction
- **Validation** — business rules, input constraints, invariant checks
- **External services** — API calls, third-party integrations
- **State** — invalid transitions, missing prerequisites, concurrency conflicts
- **Configuration** — missing settings, invalid values, environment issues

Not every project has all of these. A CLI tool might only have I/O, parsing, and configuration. An order-matching engine might focus on validation and state. Map the domains to what the project actually does.

### Step 2: Decide on error type granularity

**One error enum** when:
- The project is a CLI or small binary
- Most errors bubble to a single handler (`main()` or a response builder)
- The number of failure modes stays under ~20

**Multiple error enums** when:
- The project has distinct subsystems (e.g., `network::Error`, `parser::Error`, `storage::Error`)
- It's a library — callers shouldn't see internal error details they don't need
- The failure modes naturally cluster into independent groups

**Hierarchy** when using multiple:
- Each module defines its own error type
- The parent module's error type wraps children via `#[error(transparent)]` and `#[from]`
- This preserves the source chain for error reporters

### Step 3: Sketch the error types

For each error type, define:
- **Name** — reflects its scope (`AppError`, `ParseError`, `OrderError`, etc.)
- **Variants** — each one answers "what went wrong?" and carries the context needed to diagnose it
- **Fields** — named fields for 2+ context pieces; positional for single-field variants
- **Source chain** — which variants wrap underlying errors via `#[source]` or `#[from]`
- **Display messages** — format strings that produce human-readable diagnostics

### Step 4: Plan the conversions

For each library error type the project will encounter:
- **`#[from]` automatic conversion** — when the mapping is unambiguous (one variant maps to one source error type)
- **`.map_err()` manual conversion** — when context needs to be added, or when multiple operations produce the same error type (e.g., multiple `io::Error` sites that need different context)

### Step 5: Plan the result alias

Every error type gets a result alias:
```rust
pub type AppResult<T> = Result<T, AppError>;
```

If there are multiple error types, each gets its own alias in its own module.

### Step 6: Plan error handling boundaries

Identify where in the project errors should be **handled** vs **propagated**:
- **`main()` or entry point** — format and display, set exit code
- **API/request boundaries** — map to response types
- **Retry points** — decide whether to retry based on error variant
- **Fallback logic** — try primary path, fall back on specific failures

Everything else propagates with `?`.

---

## If refactoring an existing codebase

When the inputs indicate existing code with poor error handling, your plan must include a migration strategy. Assess the current state:

### What to look for
- `.unwrap()` and `.expect()` in non-test code — each one is a potential panic in production
- `String` as the error type — loses type information and source chains
- `Box<dyn Error>` everywhere — same problem, slightly better
- `anyhow` in library code — strips type information at module boundaries
- Silent error swallowing — `let _ = fallible_call();` or `.ok()` without justification
- Inconsistent error handling — some modules use proper types, others use strings

### Building the migration into the plan
If error handling is poor and `thiserror` isn't in use, the plan should include:

1. **Add `thiserror` to `Cargo.toml`** as an explicit step
2. **Create `errors.rs`** with the designed error types
3. **Migration order** — which modules to convert first (start with leaf modules, work inward)
4. **Unwrap audit** — flag the most dangerous unwraps (those in production code paths) for priority replacement
5. **Estimated scope** — whether this is a contained task or touches most of the codebase

This migration plan is part of the output document, not a separate deliverable.

---

## Writing the output

Write a plan that implementation agents can follow directly. Be specific about types, variants, and file locations — not vague about "consider using proper error handling."

```markdown
# Error Handling Plan

**Date:** YYYY-MM-DD

## Strategy
Brief summary of the approach — thiserror, single vs multiple error types, and why.

## Crates
- **thiserror** — for deriving Error and Display on all error types
- **anyhow** — (only if applicable) for top-level application glue, with justification
- Any other error-related crates if needed (e.g., `color-eyre` for CLI error display)

## Error Types

### [ErrorTypeName] (`path/to/errors.rs`)
Purpose and scope of this error type.

**Variants:**
- **VariantName** `{ field: Type, field: Type }` — what this represents, when it occurs
- **AnotherVariant** `{ field: Type, #[source] cause: UnderlyingError }` — wraps errors from X

**Result alias:** `pub type XResult<T> = Result<T, ErrorTypeName>;`

**Conversions:**
- `#[from] SomeError` — automatic, unambiguous mapping
- `.map_err()` for `io::Error` in file operations — needs path context

(Repeat for each error type)

## Error Propagation
Where errors propagate (with `?`) and where they're handled — entry points, API boundaries, retry logic.

## Migration Plan (if refactoring)
If the project has existing code with poor error handling:

### Current State
What's wrong — unwraps, string errors, missing context.

### Migration Steps
Ordered list of what to change, starting with leaf modules.

### Unwrap Audit
The most critical unwraps to replace and what to replace them with.

## Design Rationale
Why this structure was chosen — what the inputs revealed about the project's failure modes and how the error types map to them.
```

Write to the output path you were given (relative to the project's rex directory).
