---
name: rex-onboarding-success-measures
description: Define measurable, verifiable success criteria for the project during rex onboarding — things agents can check before handing off to UAT. Use this skill when the rex onboarding process reaches the "success-measures" step, when the user wants to define how to verify the project works correctly. Also trigger when the user says things like "how do we know it works", "what should we test for", "define success criteria", or "what does correct look like."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Success Measures

You help users define concrete, measurable criteria that agents can verify *before* the project goes to the user for acceptance testing. UAT is about whether the user is happy with the result. Success measures are about whether the thing actually works correctly.

You'll be told where to write the output (a file path like `onboarding/success-measures.md`). If input files are provided, read them first for context. Then work with the user and write the final document to the output path.

---

## What success measures are

Success measures are things an agent can check programmatically or through structured verification. They answer: "does this work?" before asking "does the user like it?"

Examples:

- **Functional** — "The CLI parses all valid input formats without error." "The API returns the correct response for these 5 test cases." "Importing a 10k-row CSV completes without data loss."
- **Performance** — "Responses return in under 200ms for typical queries." "Processing 1M records takes under 30 seconds."
- **Correctness** — "The output matches the reference implementation for these inputs." "The calculation produces results within 0.01% of the known answers."
- **Reliability** — "The system handles malformed input without crashing." "Concurrent requests don't produce race conditions."
- **Build health** — "All tests pass." "No compiler warnings." "Clippy runs clean."

These are distinct from UAT (which is the user's subjective assessment) and from known risks (which are things that might go wrong).

---

## How to run the conversation

Ask the user what "working correctly" means for this project. Probe for specific, verifiable criteria:

- "If an agent finishes building this, what should it check before calling it done?"
- "What would a broken version look like — what are the failure modes you care about?"
- "Are there specific inputs and expected outputs you can describe?"
- "Does performance matter? Are there thresholds?"
- "Are there edge cases that absolutely must work?"

Help the user be concrete. "It should be fast" becomes "sub-200ms for queries under 1000 rows." "It should handle errors" becomes "returns a clear error message for invalid JSON input instead of panicking."

If the user has reference data, known-correct outputs, or benchmark targets, capture those — they're gold for automated verification.

---

## Writing the output

This document tells agents exactly what to verify before handoff. Each measure should be specific enough that an agent can write a test or run a check against it.

```markdown
# Success Measures

**Date:** YYYY-MM-DD

## Measures

### 1. [Measure name]
- **What:** what to verify
- **How:** how an agent can check this (test, benchmark, command, comparison)
- **Pass criteria:** what "success" looks like specifically
- **Priority:** must-pass or nice-to-have

### 2. [Measure name]
...

## Reference Data
Any known-correct outputs, benchmark targets, or test fixtures the user provided.

## Context
How these were discussed — what the user emphasized, any reasoning behind specific thresholds, what "working" means to them.
```

Write to the output path you were given (relative to the project's rex directory).
