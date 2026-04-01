---
name: rex-design-rust-integration-tests
description: Plan the integration test strategy for a Rust project during the rex design phase — identifying every production failure mode, classifying tests as CRITICAL/IMPORTANT/NICE-TO-HAVE, and specifying real-world data flows. Use this skill when the rex design process reaches the "integration-tests" step, when the project needs its integration test plan designed before implementation, or when the user says things like "plan the integration tests", "what tests do we need", "how do we test this end-to-end", "figure out what could break in production", or "design the test strategy." This skill is paranoid about production failures — it finds every boundary crossing, every data flow, every place the system touches the real world, and plans tests that prove those paths work with real data, not synthetic fixtures.
disable-model-invocation: false
user-invocable: false
---

# Design: Integration Test Plan

You plan integration tests that prevent production failures. Not unit tests dressed up with a database connection — real integration tests that prove the system works when all the pieces are wired together, with data that looks like what production actually sends.

This is the final safety net. Unit tests verify that individual functions compute correctly. Integration tests verify that the system *works*. They catch the failures that only emerge when real components interact: serialization mismatches, connection exhaustion, timeout cascading, data encoding surprises, race conditions under actual concurrency, and the hundred other things that work perfectly in isolation and break catastrophically when combined.

If the integration tests pass, the system works. If they don't, it doesn't. There is no middle ground.

You'll be told where to write the output (a file path like `design/integration-tests.md`) and given input files to read for context. Read them all. Then think about every way this system can fail in production, and plan the tests that would have caught each failure.

---

## The fundamental rule: no synthetic data

Integration tests do not use synthetic data. That is what unit tests are for.

Synthetic data — `test_user_1`, `foo@example.com`, `{"key": "value"}` — proves nothing about production. Production data has unicode in usernames, email addresses with plus signs and dots, JSON with nested nulls and unexpected fields, timestamps in wrong timezones, numbers at the boundaries of integer ranges, and strings that happen to contain SQL injection patterns just because someone's last name is O'Brien.

Every integration test you plan specifies data that mirrors what production actually looks like:
- If the system processes orders, the test uses an order with realistic field values — a price with 8 decimal places because that's what the exchange sends, a symbol with a dot in it because some assets have compound tickers, a quantity of zero because someone submitted a market order with no fill
- If the system reads configuration, the test uses a config file with every quirk you can anticipate — comments, trailing whitespace, values that look like numbers but are strings, environment variable references
- If the system handles API responses, the test uses actual response shapes from the API documentation, including error responses, rate limit responses, and malformed responses you've seen in the wild

When real production data isn't available, use data *shaped like production data* — same encoding, same edge cases, same structure. The point is to test the system against the mess of reality, not against a tidy abstraction of reality.

---

## Reading the inputs

Read everything. Integration tests sit at the intersection of all design decisions — you need the full picture.

### From goal and scope
- What the system does end-to-end — this defines the primary integration test paths
- Who uses it — different users create different data patterns

### From architecture
- The type hierarchy — where data transforms happen and what can go wrong at each transform
- Trait boundaries — these are the seams where mocking happens in unit tests and where *real implementations* must be tested in integration tests
- Library integrations — every external crate that touches I/O is a potential failure point

### From module layout
- Module boundaries — cross-module calls are integration test targets
- The `tests/integration/` directory structure — your planned tests map to files here

### From error-handling plan
- Every error variant — for each one, ask: "Is there an integration test that proves this error is returned correctly when the real system encounters this failure?" If not, plan one
- Error propagation paths — integration tests should verify errors bubble up correctly through the full stack, not just one module

### From existing-code-exploration (if refactoring)
- Critical invariants — integration tests must verify these survive the refactor
- Hidden side effects — integration tests must verify these still happen (or stop happening, if that's the intent)
- Production incident history — if the inputs mention past failures, those are automatic CRITICAL test cases

### From library review
- External crate behaviors — libraries have their own failure modes. `reqwest` times out. `sqlx` has connection pool limits. `serde` has deserialization edge cases. Plan tests that exercise these
- Version-specific behaviors — if the library review flagged version changes or breaking changes, test those boundaries

### From success measures
- Every measurable success criterion should have at least one integration test backing it
- Performance requirements — if "responds within 100ms" is a success measure, there should be an integration test that measures response time under realistic load

### From known risks
- Every risk flagged during onboarding should map to at least one integration test that would catch it

---

## Finding the failure modes

For every boundary the system crosses, ask: "What goes wrong here in production?"

### Network boundaries
- **Connection refused** — the remote service is down
- **Timeout** — the remote service is slow (not down, just slow — the worst kind of failure because retries make it worse)
- **Partial response** — the connection drops mid-transfer
- **TLS failures** — certificate expired, hostname mismatch, intermediate CA not in the chain
- **DNS failures** — resolution fails, returns stale IP
- **Rate limiting** — the remote service returns 429 and expects backoff
- **Authentication expiry** — the token was valid when the process started and expired during a long-running operation

### Data boundaries
- **Encoding** — UTF-8 with BOM, latin-1 masquerading as UTF-8, null bytes in strings, surrogate pairs
- **Serialization** — the JSON the API actually sends vs what the docs say it sends (they are never identical)
- **Schema drift** — the database has columns that the code doesn't know about, or the code expects columns that don't exist yet
- **Numeric precision** — floating-point comparisons, integer overflow at boundaries, decimal precision loss during conversion
- **Timestamps** — timezone-naive vs timezone-aware, leap seconds, DST transitions, `0000-00-00` dates in legacy data

### Concurrency boundaries
- **Race conditions** — two requests modifying the same resource simultaneously
- **Lock contention** — connection pool exhaustion under concurrent load
- **Ordering** — events arriving out of order when the code assumes in-order delivery
- **Deadlocks** — multiple resources locked in different orders by different threads

### Filesystem boundaries
- **Permissions** — the file exists but can't be read, or the directory exists but can't be written to
- **Disk space** — write fails mid-operation
- **Path encoding** — unicode in filenames, paths with spaces, symlinks, paths longer than 255 characters
- **Atomicity** — crash during write leaves corrupt file

### Configuration boundaries
- **Missing values** — required config key absent, environment variable not set
- **Invalid values** — config key present but value is wrong type, out of range, or empty string
- **Environment differences** — config that works on the developer's machine but fails in CI or production

---

## Classifying tests

Every test gets exactly one classification. Be honest — a test that's "nice to have" shouldn't be promoted to CRITICAL just to pad the plan. But a test that's CRITICAL must never be demoted to avoid work.

### CRITICAL

Tests that must pass before the system can be considered functional. If any CRITICAL test fails, the system is broken and cannot be deployed.

A test is CRITICAL when:
- It verifies the primary happy path — the thing the system was built to do
- It verifies that the system doesn't corrupt data (data loss is unrecoverable)
- It verifies that the system doesn't fail silently (returning success when it actually failed is worse than crashing)
- It verifies a failure mode that has caused a production incident before
- It verifies that authentication and authorization actually work (not that they exist in the code, but that unauthorized requests are actually rejected by the running system)

Expect 3-8 CRITICAL tests depending on system complexity. If you have more than 10, you're either building a very complex system or classifying too aggressively.

### IMPORTANT

Tests that verify significant functionality but whose failure doesn't make the system fundamentally broken. If an IMPORTANT test fails, there's a real problem that needs fixing before production, but the core system may still be usable.

A test is IMPORTANT when:
- It verifies a secondary path that real users will encounter (not the primary function, but common operations like error recovery, retry behavior, pagination)
- It verifies graceful degradation — what happens when a non-essential dependency is down
- It verifies performance under realistic load (the system works but is it fast enough?)
- It verifies that error messages are actually useful (not that errors exist, but that a human can diagnose the problem from the error)

Expect 5-15 IMPORTANT tests.

### NICE-TO-HAVE

Tests that increase confidence but whose absence doesn't meaningfully increase production risk. These are defense-in-depth — they catch unlikely scenarios or verify behaviors that are already implicitly tested by higher-priority tests.

A test is NICE-TO-HAVE when:
- It verifies an edge case that production is unlikely to hit in the near term
- It verifies a behavior that's already partially covered by a CRITICAL or IMPORTANT test
- It verifies cosmetic behavior (logging format, metric names, response header ordering)
- It stress-tests beyond realistic production load

---

## Planning each test

For every test, specify enough detail that the implementation agent can write it without guessing. The implementation agent should never have to ask "but what data do I use?" or "how do I know if this passed?" — your plan answers those questions.

### What to specify for each test

**Name and classification.** A descriptive name that says what production failure this test prevents. Not `test_database_connection` but `test_order_persistence_survives_connection_pool_exhaustion`.

**What production failure this prevents.** One sentence describing the real-world scenario this test guards against. This is the "why" — it justifies the test's existence and its classification. Example: "Prevents the scenario where a burst of concurrent order submissions exhausts the connection pool, causing orders to be silently dropped because the write returns a connection error that the retry logic doesn't handle."

**The real-world data.** Exactly what data this test uses, with specific values that mirror production. Not "a valid order" but "an order with symbol `BTC/USDT`, price `0.00003417` (8 decimal places, common in crypto pairs), quantity `1500000` (large lot), and client_order_id containing a UUID with hyphens."

**Setup requirements.** What real infrastructure this test needs — a running database with a specific schema, a mock server mimicking a specific API, a config file with specific values. Integration tests need real (or realistically mocked) dependencies. Specify which dependencies must be real and which can be test doubles, and why.

**The sequence of operations.** What the test does, step by step. Not just "send an order and check it's persisted" but:
1. Start with a database containing 3 existing orders (to verify the new order doesn't interfere)
2. Submit an order via the same entry point production uses (not by calling an internal function)
3. Wait for the async processing to complete (specify how — polling? channel? timeout?)
4. Query the database directly (not through the application) to verify the order was persisted with correct field values
5. Verify the response matched what the caller would see

**What "pass" means.** The specific assertions. Not "the order is saved" but:
- The order exists in the `orders` table with `status = 'filled'`
- The `created_at` timestamp is within 1 second of the submission time
- The `fill_price` matches the expected value to 8 decimal places
- The response contained a `trade_id` that matches a row in the `trades` table

**What "fail" tells you.** If this test fails, what's the most likely cause? This helps the debugging agent triage quickly. "If this fails, check: (1) connection pool size vs concurrent request count, (2) retry logic in the persistence layer, (3) whether the circuit breaker is tripping too aggressively."

---

## Test infrastructure plan

Integration tests need infrastructure. Plan it explicitly.

### What needs to be real
- Database (if the system uses one) — specify if it needs seeded data, specific schema, or specific version
- File system fixtures — config files, test data files, with exact paths and contents described
- Network services — if the system calls external APIs, specify how to handle them (real sandbox? mock server? recorded responses?)

### What can be a test double
- External services that are expensive, rate-limited, or non-deterministic — but the test double must faithfully reproduce the *failure modes* too, not just the happy path
- Time-dependent operations — use controllable clocks, not `sleep()`

### Test isolation
- How tests are isolated from each other (database transactions? separate databases? cleanup hooks?)
- Whether tests can run in parallel or must be sequential (and why)
- How test state is cleaned up (teardown functions? drop traits? database rollback?)

### CI/CD considerations
- How long the integration test suite should take to run (budget in seconds/minutes)
- What external dependencies CI needs (database container? mock server? credentials?)
- Whether any tests should be skipped in CI and only run in staging (and why)

---

## Writing the output

```markdown
# Integration Test Plan

**Date:** YYYY-MM-DD

## Philosophy
Brief statement of what these tests prove and why they matter for this specific project.

## Failure Mode Analysis
The production failure modes identified, organized by boundary type (network, data, concurrency, filesystem, configuration). For each:
- **Failure:** what goes wrong
- **Impact:** what happens to the user/system
- **Covered by:** which test(s) below catch this

Failure modes not covered by any test should be explicitly listed with a justification for why they're not tested (accepted risk, infeasible to test, covered by monitoring instead).

## Test Infrastructure

### Dependencies
What real/mock infrastructure the test suite needs, how to set it up, and how to tear it down.

### Test Data Strategy
Where test data comes from, how it mirrors production, and how it's managed across test runs.

### Isolation Strategy
How tests are kept independent, whether they can run in parallel, cleanup approach.

## CRITICAL Tests

### `test_name`
**Prevents:** [one-sentence production failure scenario]
**Classification rationale:** [why this is CRITICAL, not IMPORTANT]

**Data:**
[Exact data values with production-realistic specifics]

**Setup:**
[What infrastructure and state must exist before the test runs]

**Sequence:**
1. [Step-by-step operations]
2. ...

**Pass criteria:**
- [Specific assertion]
- [Specific assertion]

**On failure, check:**
- [Likely cause 1]
- [Likely cause 2]

(Repeat for each CRITICAL test)

## IMPORTANT Tests

(Same format as CRITICAL)

## NICE-TO-HAVE Tests

(Same format, but can be briefer on the sequence/assertions since these are lower priority)

## Coverage Map

| Production Failure Mode | Test(s) | Classification |
|------------------------|---------|---------------|
| Connection pool exhaustion | test_concurrent_writes_under_pool_limit | CRITICAL |
| Malformed API response | test_handles_unexpected_json_fields | IMPORTANT |
| ... | ... | ... |

## Uncovered Risks
Failure modes that this test plan does not cover, with justification:
- **[Failure mode]** — [why it's not tested] — [mitigation strategy instead (monitoring, alerting, manual testing)]

## Implementation Order
Which tests to write first. CRITICAL tests first, but within CRITICAL, order by:
1. Tests that are easiest to set up (quick confidence)
2. Tests that cover the most failure modes (highest leverage)
3. Tests that verify the riskiest code paths (highest impact)
```

Write to the output path you were given (relative to the project's rex directory).
