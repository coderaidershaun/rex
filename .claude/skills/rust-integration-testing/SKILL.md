---
name: rust-integration-testing
description: Write high-stakes Rust integration tests that catch real-world production failures — using real data, real connections, and real failure modes. Use this skill when verifying that a system works end-to-end in production conditions, when the user says "integration test", "test against real data", "make sure this works in production", "test the websocket", "test the API connection", or when working on code that interacts with external systems (APIs, databases, websockets, file systems, networks). Also trigger when the user mentions "test for real", "production test", "end-to-end test", or when an agent needs to verify that a feature works beyond unit-level correctness. This skill is the last line of defense before shipping — it must be used with maximum thinking depth and care. Do NOT use this for synthetic/mock-based testing (that belongs to rust:unit-testing).
disable-model-invocation: false
user-invocable: true
---

# Rust Integration Testing

You are the last line of defense before code ships. Your job is to think deeply about how an application breaks in the real world, then write targeted tests that prove it doesn't — using real data, real connections, and real failure modes. Never synthetic data. Never mocks. If it can't be tested against reality, document exactly why.

This skill demands your absolute best thinking. Use extended thinking for deep reasoning about failure modes. Take your time. The quality of what you produce here determines whether the entire project succeeds or fails in production.

---

## Philosophy

Unit tests verify logic. Integration tests verify reality. The gap between "works in isolation" and "works in production" is where projects die. Your tests live in that gap.

- **Real data only.** Synthetic data hides the messy, inconsistent, malformed inputs that production throws at you. Unit tests already cover the clean cases.
- **Think like an adversary.** Before writing a single test, enumerate every way this system could break when real users, real networks, and real data are involved.
- **Quality over quantity.** Five tests that each catch a different class of production failure are worth more than fifty that all test the happy path with slight variations.
- **Short and sharp.** Each test should be readable in under 30 seconds. If it's long, the test is doing too much — split it or simplify.

---

## Before Writing Any Tests

Stop. Think. This is the most important step.

### Step 0: Check for a design integration test plan

Before doing your own failure mode analysis, check if a design-phase integration test plan exists. When working within a rex project, this file is at `rex/<project-id>/design/integration-tests.md`. Read it if it exists.

If the design plan exists, it is your **primary guide** — it contains pre-analyzed failure modes classified as CRITICAL/IMPORTANT/NICE-TO-HAVE, real-world data patterns, test infrastructure requirements, and specific test specifications. Your job is to **implement those tests**, not re-plan them from scratch. Use the design plan's failure modes, data values, and pass criteria directly.

If the design plan doesn't exist (standalone usage or greenfield project), proceed with Step 1 below to do your own failure mode analysis.

### Step 1: Failure Mode Analysis

Before writing code, write a list. For the system under test, enumerate every way it could break in production. Think about:

**Network and connectivity:**
- Connection drops mid-operation
- DNS resolution failures
- TLS handshake failures or certificate issues
- Timeouts (server slow, server gone, network congestion)
- Rate limiting / throttling from external services

**Data integrity:**
- Malformed responses from external APIs
- Schema changes in upstream services
- Encoding issues (UTF-8 edge cases, binary data in text fields)
- Timestamps in unexpected timezones or formats
- Numeric overflow or precision loss in financial data

**State and concurrency:**
- Race conditions between concurrent operations
- Stale connections in connection pools
- Resource exhaustion (file handles, memory, connection limits)
- Partial failures in multi-step operations (step 3 of 5 fails — now what?)

**Authentication and authorization:**
- Expired tokens or credentials
- Permission changes mid-session
- API key rotation

**Environment:**
- Missing environment variables or config
- Disk space exhaustion
- OS-level resource limits

Write this list as a comment block at the top of your test file. It serves as both a planning document and a record of what you considered.

### Step 2: Prioritize

Not everything on the list can or should be tested. Rank by:
1. **Likelihood** — How often could this realistically happen?
2. **Impact** — What breaks if it does happen? Data loss? Silent corruption? Graceful degradation?
3. **Testability** — Can you reliably trigger this in a test? If not, document it but don't force a flaky test.

Pick the top 3-7 failure modes. Those become your tests.

---

## Writing the Tests

### File Location

Integration tests go in the `tests/` directory at the crate root, not inline with the source:

```
project/
├── src/
│   └── ...
├── tests/
│   └── integration/
│       ├── mod.rs           (optional, for shared setup)
│       ├── websocket.rs     (one file per system boundary)
│       └── api_client.rs
```

### Test Structure

```rust
//! Integration tests for [component]
//!
//! Failure modes considered:
//! - [list from Step 1]
//! - [list from Step 1]
//! - [list from Step 1]

use your_crate::whatever;

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn reconnects_after_server_drops_connection() {
    // Setup: connect to real endpoint
    // Act: trigger the failure condition
    // Assert: system recovers correctly
}
```

### Key Rules

1. **Every passing test gets `#[ignore]`**. Integration tests hit real systems and take real time. They must not slow down `cargo test` during development. They run on demand with `cargo test -- --ignored`.

2. **Use real production data.** If you're testing an API client, hit the real API. If you're testing a parser, feed it real-world files. If you need credentials or endpoints, read them from environment variables — never hardcode them.

3. **Keep tests short.** Setup, act, assert. If you need more than ~20 lines per test, the code under test may need a better interface — consider invoking the `/rust:planning-and-architecture` skill to redesign it.

4. **One failure mode per test.** Don't write a test that checks connection handling AND data parsing AND error recovery. Each test targets one specific way things break.

5. **Clean up after yourself.** If your test creates files, connections, or state — tear it down. Use `Drop` guards or explicit cleanup at the end.

### Environment Variables for Real Data

Tests that need external resources should read from env vars and skip gracefully if unavailable:

```rust
fn get_env_or_skip(var: &str) -> String {
    match std::env::var(var) {
        Ok(val) => val,
        Err(_) => {
            eprintln!("Skipping: {} not set", var);
            return String::new();
        }
    }
}
```

Or use a macro pattern:

```rust
macro_rules! require_env {
    ($var:expr) => {
        match std::env::var($var) {
            Ok(val) if !val.is_empty() => val,
            _ => {
                eprintln!("SKIPPED: {} not set — set it to run this integration test", $var);
                return;
            }
        }
    };
}

#[test]
#[ignore]
fn fetches_live_market_data() {
    let api_key = require_env!("EXCHANGE_API_KEY");
    let endpoint = require_env!("EXCHANGE_WS_ENDPOINT");
    // ... test with real credentials
}
```

---

## When Tests Fail and You Can't Fix Them

Sometimes integration tests fail. Before escalating, you MUST make genuine, sustained effort to fix each failure yourself.

### The "try harder" rule: 3 distinct attempts minimum

For each failing test, you must make **at least 3 substantive, distinct attempts** to fix it before classifying it as requiring user input. Giving up after one attempt is unacceptable. Work through these approaches:

1. **Re-read the error carefully.** Error messages almost always point to the exact issue. Read the full stack trace. Check if it's a compile error, runtime panic, assertion failure, or timeout.
2. **Check the code under test.** The failure might be a code bug, not a test bug. Read the production code the test exercises. Look for logic errors, missing error handling, wrong assumptions about data format.
3. **Check environmental setup.** Is the right config loaded? Are paths correct? Is the test data in the right format? Is the test running from the right directory?
4. **Search the codebase for working patterns.** Find similar code or tests that work. What are they doing differently?
5. **Try alternative approaches.** Different test data, different setup sequence, different assertion strategy.

Only classify a failure as requiring user input when the blocker is **genuinely external** to the codebase:
- Missing API credentials the user hasn't provided
- Services behind authentication you don't have access to
- Geo-blocked or IP-restricted endpoints
- Infrastructure not yet provisioned (databases, message queues, etc.)
- Rate limits that can't be worked around
- Platform-specific restrictions (OS, hardware)

### When you've exhausted your attempts

If after 3+ genuine attempts a failure is truly outside your control:

#### Step 1: Write a detailed escalation report

Write to `rex/<project-id>/user-support/requested.md`:

```markdown
# Integration Test Escalation

## Failing Test(s)
- `test_name_here` in `tests/integration/module.rs`

## Error Output
[Exact error message / stack trace]

## What I Tried
1. [First approach — what I did and what happened]
2. [Second approach — what I did and what happened]
3. [Third approach — what I did and what happened]

## Root Cause
[Your assessment — e.g., "EXCHANGE_API_KEY environment variable is not set"]

## What the User Needs to Do
[Specific, actionable steps — e.g., "Set the EXCHANGE_API_KEY env var with a valid API key from the exchange dashboard"]

## Files to Look At
- [relevant file paths]

## Impact if Not Resolved
[What functionality remains unverified]
```

#### Step 2: Activate user-support

```bash
rex project update-status user-input not-started
```

#### Step 3: Tell the operator to keep this task open

Say exactly this in your response: **"This task must remain in-progress. I have escalated to user-support. DO NOT MARK THIS TASK AS COMPLETE."**

This is critical — the operator reads your response and will leave the task in-progress. On the next run, the user provides input, and then this task is resumed with the user's answer.

#### Step 4: Don't delete the failing test

Leave it in place with `#[ignore]`. It represents a real verification that needs to happen — removing it just hides the gap.

---

## When to Invoke Other Skills

**`/rust:planning-and-architecture`** — If your integration tests reveal that the code needs structural changes to be testable or robust (e.g., no retry logic, no timeout handling, hardcoded endpoints), invoke this skill to plan the fix. Only do this when integration testing surfaces a real architectural issue — not for cosmetic improvements.

**User-support escalation** — If failures are outside your control (missing credentials, external service down, user decision needed), follow the escalation steps in "When Tests Fail and You Can't Fix Them" above. Write to `rex/<project-id>/user-support/requested.md`, run `rex project update-status user-input not-started`, and tell the operator not to mark the task complete.

---

## Example: WebSocket Integration Tests

Here's what thorough integration testing looks like for a WebSocket client:

```rust
//! Integration tests for WebSocket client
//!
//! Failure modes considered:
//! - Server drops connection unexpectedly
//! - Server sends malformed JSON
//! - Connection timeout on initial handshake
//! - Server sends messages faster than we can process
//! - Authentication token expires mid-session
//! - Network partition (long disconnect, then reconnect)
//! - Server returns HTTP 429 (rate limited)

#[test]
#[ignore]
fn connects_and_receives_first_message() {
    let url = require_env!("WS_TEST_ENDPOINT");
    let client = WsClient::connect(&url, Duration::from_secs(5)).unwrap();
    let msg = client.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(!msg.is_empty(), "First message should contain data");
}

#[test]
#[ignore]
fn recovers_from_connection_drop() {
    let url = require_env!("WS_TEST_ENDPOINT");
    let client = WsClient::connect(&url, Duration::from_secs(5)).unwrap();

    // Force disconnect
    client.force_close();

    // Should reconnect automatically
    let reconnected = client.wait_for_reconnect(Duration::from_secs(30));
    assert!(reconnected.is_ok(), "Should reconnect within 30s");
}

#[test]
#[ignore]
fn handles_malformed_server_response() {
    // This test requires a test endpoint that can send bad data
    // If not available, document in failing.md
    let url = require_env!("WS_TEST_ENDPOINT_MALFORMED");
    let client = WsClient::connect(&url, Duration::from_secs(5)).unwrap();

    // Client should not panic — should return an error or skip the bad message
    let result = client.recv_timeout(Duration::from_secs(10));
    assert!(result.is_err() || result.unwrap().is_empty());
}
```

Notice: three tests, three different failure modes, all against real endpoints. Short, clear, no test infrastructure.

---

## Checklist Before You're Done

- [ ] Failure mode list written as a comment block in the test file
- [ ] Each test targets one specific production failure mode
- [ ] All tests use real data / real connections (no mocks, no synthetic data)
- [ ] All passing tests marked `#[ignore]`
- [ ] Tests that can't pass (after 3+ fix attempts) are documented in `rex/<project-id>/user-support/requested.md`
- [ ] `rex project update-status user-input not-started` called if blocked on user input
- [ ] Operator told "DO NOT MARK THIS TASK AS COMPLETE" if escalated
- [ ] `cargo test -- --ignored` runs clean for all non-blocked tests
- [ ] No unnecessary code — every line earns its place
