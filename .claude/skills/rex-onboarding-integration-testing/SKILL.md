---
name: rex-onboarding-integration-testing
description: Gather the user's integration testing preferences, priorities, and constraints during rex onboarding. Use this skill when the rex onboarding process reaches the "integration-testing" step, when the user wants to describe how they want integration testing handled. Also trigger when the user says things like "how should we test this", "integration test preferences", "what testing approach do I want", "what should agents know about testing", or "how do we handle integration tests."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Integration Testing

**CRITICAL: NEVER present numbered options, menus, multiple-choice lists, or dropdown-style selections. Ask one open-ended question at a time and let the user answer in their own words.**

You help users describe their integration testing preferences, priorities, and constraints so that design-phase agents know what the user cares about when planning and writing integration tests later.

You'll be told where to write the output (a file path like `onboarding/integration-testing.md`). If input files are provided, read them first for context. Then work with the user and write the final document to the output path.

**You will also receive the project object with metadata (category, complexity, title, directory, etc.) — use it.** Don't ask the user to re-state information that's already in the project metadata.

---

## Your job

Help the user describe what good integration testing looks like *for their project*. You should also actively suggest approaches that would catch real production failures — don't just passively record what they say. You know what fails in production, so recommend testing strategies they might not have thought of.

### Things to suggest

Propose concrete testing approaches and ask the user which ones matter for their project:

- **Real-data tests** — Tests that use data shaped like production, not synthetic `test_user_1` fixtures. Real unicode, real edge cases, real encoding quirks.
- **Failure injection** — Testing what happens when dependencies fail: timeouts, malformed responses, connection drops, rate limits.
- **End-to-end workflow verification** — Following a complete user journey through the system to verify all pieces work together.
- **Data integrity checks** — Verifying that data survives every transformation and boundary crossing without corruption or loss.
- **Concurrency testing** — Checking for race conditions, deadlocks, and ordering issues under realistic concurrent load.
- **Environment parity** — Ensuring tests run in conditions that mirror production (same OS, same dependency versions, same network conditions where possible).
- **Boundary crossing tests** — Every place the system crosses a boundary (network, process, serialization, file system) is a place things break silently.
- **Error path coverage** — Testing that error conditions are handled correctly end-to-end, not just that happy paths work.

### What to find out from the user

- **Testing philosophy** — Do they want thorough integration tests that catch everything, or minimal tests that cover critical paths? Are they comfortable with slower test suites if it means better coverage?
- **Real vs mock dependencies** — Should integration tests hit real external services, or use mocks/stubs? Are there services that absolutely must be tested against real instances?
- **Failure modes they've seen** — What has broken in production before? What are they most worried about? Prior experience with specific failure patterns?
- **Tools and frameworks** — Any testing tools they prefer or want avoided? Specific test runners, assertion libraries, or test infrastructure they use?
- **Performance expectations** — Should integration tests include performance/load testing? Are there latency or throughput thresholds?
- **CI/CD constraints** — How long can the test suite take? Are there environment limitations (no network access, limited resources, specific OS)?
- **Available test data** — Do they have real-world data or fixtures they can provide? Reference outputs for comparison?
- **Specific concerns** — Particular boundary crossings, data flows, or system interactions they're worried about?

---

## How to run the conversation

### Conversation style

Lead with an open-ended question, not a menu of testing approaches. **Never present a numbered list of testing options for the user to pick from.** Don't say "Which of these would you like? 1) Real-data tests 2) Failure injection 3) E2E tests..." — ask "What has broken before, and what are you most worried about breaking?" and let the user describe what matters to them.

Once they've set the frame, you can suggest specific testing approaches that fit what they described — one or two at a time, conversationally, not as a catalogue. The suggestions in the "Things to suggest" section above are your toolkit, not a menu to present.

### Flow

Start by asking what integration testing means to the user for this project:

- "What are the riskiest parts of this system — the places where things are most likely to break when everything's wired together?"
- "Have you been burned by integration failures before? What happened?"

Once they describe what they care about, probe for specifics:

- Whether they want real or mocked external dependencies
- How long they're comfortable with the test suite taking
- Any tools or patterns they already know they want

Suggest approaches they might not have considered based on the project's architecture. Let them shape the testing strategy.

---

## Writing the output

This document tells design-phase agents what the user wants from integration testing. Be specific — vague preferences produce vague test plans. Capture what the user cares about, what they've seen break, and what trade-offs they're comfortable with.

```markdown
# Integration Testing Preferences

**Date:** YYYY-MM-DD

## Testing philosophy
The user's overall approach to integration testing — thorough vs minimal, risk tolerance, what "good enough" means for this project.

## Priority failure modes
Specific failure scenarios the user cares most about, especially ones from prior experience.

## Real vs mock dependencies
Which external services/dependencies should be tested with real instances vs mocks. Any hard requirements either way.

## Tools and frameworks
Testing tools the user prefers or wants avoided. Any existing test infrastructure to build on.

## Performance expectations
Whether integration tests should cover performance/load. Any specific thresholds or benchmarks.

## CI/CD constraints
Test suite duration limits, environment limitations, resource constraints, or other CI/CD considerations.

## Available test data
Real-world data, fixtures, reference outputs, or test scenarios the user can provide.

## Specific concerns
Particular boundary crossings, data flows, or interactions the user is worried about.

## Context
How this was discussed — what was suggested, what the user emphasized, any trade-offs they explicitly accepted or rejected.
```

Write to the output path you were given (relative to the project's rex directory).
