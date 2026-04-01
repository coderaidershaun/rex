---
name: rust-errors-management
description: Architect and implement robust Rust error handling using thiserror, dedicated error types, and proper propagation. Use this skill when creating or restructuring error types, when reviewing code for unwrap/expect misuse, when the user says "fix the errors", "add error handling", "clean up unwraps", "make errors useful", "I can't tell where this error came from", or when writing new modules that need to integrate with the project's error system. Also trigger when adding new CLI commands, new modules, or any code that introduces new failure modes that need proper error variants. If you see scattered unwraps, bare panics, or error messages that don't carry context about what went wrong and where — this skill applies.
disable-model-invocation: false
user-invocable: true
---

# Rust Error Management

You design and implement error handling that makes failures diagnosable. When something goes wrong in production, the person reading the error message should be able to trace exactly what happened, where, and why — without adding println debugging or re-running with RUST_LOG=trace.

The core conviction: **errors are data, not strings.** They should be typed, structured, composable, and carry enough context to diagnose the failure from the error alone.

---

## The thiserror Foundation

`thiserror` is the preferred approach for error types in this project. It derives `std::error::Error` and `Display` with minimal boilerplate while keeping errors as real types with real variants — not opaque strings.

### The errors.rs Pattern

Every project (or major module) should have an `errors.rs` that defines its error types and a result alias:

```rust
//! Error types and result alias.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("failed to read config at {path}: {source}")]
    ConfigRead {
        path: String,
        source: io::Error,
    },

    #[error("invalid configuration: {reason}")]
    ConfigInvalid { reason: String },

    #[error("connection to {endpoint} failed after {attempts} attempts")]
    ConnectionFailed {
        endpoint: String,
        attempts: u32,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

pub type AppResult<T> = Result<T, AppError>;
```

Key design choices:
- **Named fields over positional** when there are 2+ pieces of context — `{path}` reads better in error messages than `{0}`, and the field names serve as documentation at the construction site.
- **A result alias** (`AppResult<T>`) so every function signature reads cleanly without repeating the error type.
- **The error type name reflects its scope.** A CLI project might use `CliError`. A parser library might use `ParseError`. A domain-specific module might use `OrderError` or `StreamError`. Pick a name that tells the reader what system this error belongs to.

### When to Use One Error Type vs Several

**One enum for the whole binary** works well when:
- The project is a CLI tool or small service
- Most functions ultimately bubble errors to the same handler (e.g., `main()` or a response builder)
- The number of variants stays manageable (under ~20)

**Multiple error types** make sense when:
- You have distinct subsystems with different failure modes (network vs parsing vs storage)
- A library crate exposes a public API — callers shouldn't see internal error details
- An error type is growing unwieldy and variants cluster naturally into groups

When splitting, each module gets its own error type, and the parent module's error type wraps them:

```rust
// network/errors.rs
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("connection timed out after {0}ms")]
    Timeout(u64),
    #[error("DNS resolution failed for {host}")]
    DnsFailure { host: String },
}

// lib/errors.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Network(#[from] NetworkError),

    #[error(transparent)]
    Parse(#[from] ParseError),
    // ...
}
```

`#[error(transparent)]` delegates both `Display` and `source()` to the inner error, which preserves the full error chain for anyone walking the `.source()` trail.

---

## Context: The Difference Between Useless and Useful Errors

A bare error like `"file not found"` is nearly worthless. The person debugging needs to know *which* file, *why* it was being read, and *what operation* was in progress. Context transforms errors from cryptic messages into diagnostic trails.

### Adding Context with .map_err()

The simplest tool — wrap the low-level error with higher-level meaning:

```rust
fn load_config(path: &Path) -> AppResult<Config> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| AppError::ConfigRead {
            path: path.display().to_string(),
            source: e,
        })?;

    toml::from_str(&contents)
        .map_err(|e| AppError::ConfigInvalid {
            reason: format!("in {}: {e}", path.display()),
        })
}
```

Now the caller sees `"failed to read config at /etc/app/config.toml: No such file or directory"` instead of just `"No such file or directory"`.

### The source Chain

`thiserror`'s `#[source]` attribute (or `#[from]`, which implies `#[source]`) builds the error chain that `std::error::Error::source()` exposes. This matters because error reporters (like the one you write in `main()`) can walk the chain and print every layer:

```rust
fn report_error(err: &dyn std::error::Error) {
    eprintln!("error: {err}");
    let mut source = err.source();
    while let Some(cause) = source {
        eprintln!("  caused by: {cause}");
        source = cause.source();
    }
}
```

Output:
```
error: failed to read config at /etc/app/config.toml
  caused by: No such file or directory (os error 2)
```

This is why `#[from]` conversions are valuable — they wire up the chain automatically. But use them judiciously: a `#[from] io::Error` is fine for a CLI where any I/O error is fatal, but in a library you often want to add context before converting.

---

## The unwrap Problem

Every `.unwrap()` is an implicit assertion: "I guarantee this will never be `None`/`Err` at runtime." When that guarantee is wrong, the program panics with a generic message and no context about what the calling code was trying to do.

### The Real Danger

The problem isn't just that unwrap panics — it's that the panic message is useless for diagnosis:

```
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value:
Os { code: 2, kind: NotFound, message: "No such file or directory" }'
```

Which file? What operation? What was the program trying to accomplish? The stack trace helps if you have debug symbols, but in release builds or production logs, you're blind.

### When unwrap is Acceptable

There are legitimate uses, but they're narrow:

- **After a check that guarantees success**: `if map.contains_key(&k) { map.get(&k).unwrap() }` — though `.get()` returning `Option` usually means you should use `if let` instead.
- **In tests**: Test code is throwaway verification; a panic with a backtrace is fine.
- **Compile-time constants**: `"127.0.0.1".parse::<IpAddr>().unwrap()` — the input is a literal, it will always parse. Even here, `expect("valid IP literal")` is marginally better.
- **Proving invariants in unsafe code**: Sometimes the `unwrap` documents "this is structurally guaranteed" — but even then, `expect("invariant: ...")` communicates the guarantee.

### The Replacement Hierarchy

When you encounter an unwrap in application code, replace it with the most appropriate alternative:

1. **`?` propagation** — when the caller can handle the error. This is the default choice.
2. **`if let` / `match`** — when you need to handle the success and failure cases differently.
3. **`.expect("reason this should never fail")`** — when you genuinely believe it can't fail but want to document why. The message appears in the panic, making diagnosis possible.
4. **`.unwrap_or()` / `.unwrap_or_else()` / `.unwrap_or_default()`** — when there's a sensible fallback value.

The goal: **no unwrap in any code path that could plausibly execute in production**. If an unwrap survives code review, it should have a comment explaining the structural guarantee.

---

## Bubbling Up vs Handling: Where to Draw the Line

Most code should propagate errors upward with `?`. Only a few places in the call stack should *handle* them (decide what to do about the failure). Getting this boundary right is the difference between clean error handling and a mess of try-catch-equivalent spaghetti.

### Propagate by Default

Functions that do work should report failures, not react to them:

```rust
fn process_order(order: &Order, book: &mut OrderBook) -> AppResult<Trade> {
    let validated = order.validate()?;          // propagate validation errors
    let matched = book.match_order(validated)?;  // propagate matching errors
    Ok(matched)
}
```

This function has no idea whether a validation failure should be logged, retried, or shown to a user. That's the caller's decision. By propagating, it stays focused on its own job.

### Handle at Decision Points

Errors should be handled where the program has enough context to make a decision:

- **`main()` or the top-level entry point** — format and display the error, set the exit code.
- **Request handlers / API boundaries** — map errors to HTTP status codes or response messages.
- **Retry loops** — decide whether to retry based on error type.
- **Fallback logic** — try the primary path, fall back to an alternative on specific failures.

```rust
fn main() {
    if let Err(err) = run() {
        report_error(&err);
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let config = load_config(&default_config_path())?;
    let conn = connect(&config)?;
    process(&conn)?;
    Ok(())
}
```

`run()` propagates everything. `main()` handles everything. Clean separation.

### Handling Specific Variants

Sometimes you want to handle one type of error and propagate the rest:

```rust
match load_config(path) {
    Ok(config) => config,
    Err(AppError::ConfigRead { .. }) => {
        eprintln!("Config not found, using defaults");
        Config::default()
    }
    Err(e) => return Err(e),  // propagate everything else
}
```

This is where typed errors pay for themselves — you can match on specific failure modes without parsing strings.

---

## When Panics are Appropriate

Panics have a role, but it's a narrow one. A panic says: "The program has reached a state that violates a fundamental assumption — continuing would produce wrong results or corrupt data."

### Legitimate Panic Scenarios

- **Unrecoverable invariant violations**: An index that should always be in bounds isn't. A data structure's internal state is corrupted. These indicate a programming bug, not a runtime condition.
- **During initialization**: If the program can't set up its essential resources (logger, crypto RNG, core allocator), failing fast is better than limping along.
- **`unreachable!()` arms**: When a match arm or code path is logically impossible based on the program's structure, `unreachable!()` is the right marker — it panics with a clear message if the assumption is ever violated.

### Not Legitimate

- **File not found** — that's a runtime condition, not a bug. Return an error.
- **Network timeout** — transient failures happen. Return an error and let the caller decide.
- **Invalid user input** — the user made a mistake. Return an error with a helpful message.
- **Parse failure on external data** — external data is never trustworthy. Return an error.

The heuristic: **if the failure could happen in a correctly written program running in production, it's an error, not a panic.**

---

## Practical Workflow

When you're asked to set up or restructure error handling:

### 1. Survey the Failure Modes

Read the code and identify every place something can fail:
- I/O operations (files, network, database)
- Parsing and deserialization
- Validation (business rules, invariants)
- External service calls
- State transitions that can be invalid

### 2. Design the Error Type

Group failures into an enum. Each variant should answer: *what went wrong?* and *what context does the debugger need?*

```rust
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("failed to fetch data from {source}: {reason}")]
    FetchFailed { source: String, reason: String },

    #[error("transform '{stage}' failed on record {record_id}")]
    TransformFailed {
        stage: String,
        record_id: u64,
        #[source]
        cause: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("output write failed for batch {batch_id}")]
    WriteFailed {
        batch_id: u64,
        source: io::Error,
    },
}
```

### 3. Wire Up Conversions

Use `#[from]` for automatic conversions where the mapping is unambiguous. Use `.map_err()` where you need to add context:

```rust
// Unambiguous — one io::Error variant, clear mapping
#[error("filesystem error: {0}")]
Io(#[from] io::Error),

// Ambiguous — multiple places produce io::Error, need context
let data = std::fs::read(path).map_err(|e| PipelineError::WriteFailed {
    batch_id,
    source: e,
})?;
```

### 4. Hunt Down unwraps

Search the codebase for `.unwrap()` and `.expect()` in non-test code. For each one:
- If it's in a code path that could fail in production → replace with `?` or proper handling
- If it's truly infallible → consider adding a comment, or switch to `expect("reason")`
- If it's in test code → leave it alone

### 5. Verify the Chain

After wiring up error handling, trace a failure path from a leaf function to the top-level handler. Every layer should add context or pass through transparently. The final error message should tell the full story:

```
error: pipeline failed for batch 42
  caused by: transform 'normalize' failed on record 1337
  caused by: invalid UTF-8 at byte offset 128
```

If any layer swallows context or converts to a bare string, fix it.

---

## Anti-Patterns to Flag

These are the things that erode error quality over time:

- **`anyhow` in library code** — `anyhow::Error` is great for applications but strips type information from library boundaries. Callers can't match on specific failures.
- **`.to_string()` on errors** — converts a typed, chainable error into a flat string. The source chain is lost.
- **Catch-all `Box<dyn Error>`** — sometimes necessary, but if you find yourself boxing errors everywhere, you probably need a proper error enum.
- **Silent swallowing** — `let _ = fallible_call();` or `.ok()` without a comment explaining why the error is intentionally ignored.
- **String-based error matching** — `if err.to_string().contains("timeout")` is fragile. Use typed variants.
- **Overly broad `#[from]` conversions** — if `io::Error` could come from five different operations, a single `#[from] io::Error` loses information about *which* operation failed. Add context with `.map_err()` instead.
