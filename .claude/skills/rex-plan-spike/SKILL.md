---
name: rex-plan-spike
description: Time-boxed exploration to reduce uncertainty about an external API, library, protocol, or unknown behavior. Output = findings doc + throwaway demo, not production code. Use when user says "spike on", "explore the API", "what does X actually do", "throwaway test to learn", "reduce risk before committing", or pipeline orchestrator dispatches `rex-plan-spike` step.
disable-model-invocation: false
user-invocable: true
---

# Spike

Throwaway exploration. Buy knowledge, not code. Time-boxed.

I/O contract = `rex-utils-task-request`. Read envelope first. Paths from envelope.

## What spike is

- One concrete unknown per spike.
- Output = findings doc. Maybe runnable demo.
- Code = throwaway. Quarantine in `spikes/`, never `src/`, never `tests/`.
- Time-boxed. Stop when budget hits or question answered.

## What spike is not

- Not unit tests (`rex-code-tests-unit-testing`).
- Not integration tests (`rex-code-tests-integration-testing`).
- Not pure-doc research (`rex-plan-research-apis`).
- Not v1 of a feature.

## Process

1. **Frame question.** One sentence. "Does Binance ws send heartbeats during silence?" Not "explore Binance ws".
2. **Time-box.** 30 min / 2 hr / half-day. Write budget down.
3. **Smallest test.** Hardcoded inputs OK. No abstractions.
4. **Run + capture.** Raw logs, payloads, error codes, timings.
5. **Write findings.** What learned. What surprised. What still unknown.
6. **Decide next.** Promote / update ADR / new spike / drop.
7. **Quarantine or delete code.** Never let spike code become load-bearing.

## Findings doc shape

```md
# Spike: <question>

**Time-box:** <budget>. **Spent:** <actual>.
**Date:** <YYYY-MM-DD>. **Owner:** <name>.

## Question
<one sentence>

## Setup
<min repro — versions, creds env, command>

## Findings
- <fact + evidence>. Evidence: <log / payload / link>.

## Surprises
- <contradicted assumption>.

## Still unknown
- <question that emerged>.

## Next step
<promote / ADR / new spike / drop>

## Spike code
<path to quarantine dir, or inline if tiny>
```

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| No time-box | Drags forever | Budget upfront |
| Spike code in `src/` | Throwaway becomes load-bearing | `spikes/` only |
| "Looks fine" findings | Not knowledge | Evidence per fact |
| Spike becomes v1 | Skips design | Extract findings, throw code |
| Multiple unknowns | Tangled | One spike, one question |
| No findings doc | Knowledge dies in branch | Doc = output, code = incidental |

## Anchor examples

> **Q:** Does Binance ws spot stream send keepalive during low-volume hours?
> **Time-box:** 1 hr. **Setup:** connect to `btcusdt@trade`, log frames for 10 min at 03:00 UTC.
> **Findings:** server sends `ping` every 3 min. Client must `pong` within 10 min or disconnect.
> **Next:** promote to integration test. Update ADR-007.

> **Q:** Does Coinbase reject `post_only` orders that cross, or accept-then-cancel?
> **Time-box:** 30 min. **Setup:** sandbox, place crossing `post_only` limit.
> **Findings:** 400 response immediately, body `post_only would take liquidity`. No order created.
> **Next:** update `OrderIntent v1` spec — `post_only` rejection is sync.

## Hand-off

- Architectural finding → ADR update.
- Schema clarification → `rex-plan-sdd`.
- Repeatable check → `rex-code-tests-integration-testing`.
- New unknown → another spike or `rex-plan-research-apis`.

Spike answers question. Findings doc carries knowledge. Code dies.
