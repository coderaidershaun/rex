---
name: rex-plan-bdd
description: Write Given-When-Then behavior scenarios. Concrete, observable examples of how the system must behave. Use when user wants BDD scenarios, says "write scenarios", "given/when/then", "behavior examples", or pipeline orchestrator dispatches `rex-plan-bdd` step.
disable-model-invocation: false
user-invocable: true
---

# rex-plan-bdd

Turn intent + architecture → concrete observable behavior. Given-When-Then. One scenario, one behavior.

I/O contract lives in `rex-utils-task-request`. Read inputs it gives you. Write outputs to paths it gives you. Don't assume shape — read envelope first.

## What good BDD is

- **Observable.** Outcome a user, operator, or downstream system can see. Not internal state. Not function calls.
- **Concrete.** Specific values, specific events, specific outcomes. "Sequence 1052 arrives" beats "a delta arrives".
- **Independent.** Each scenario stands alone. No "continues from previous scenario".
- **Domain-language.** Reuse project's ubiquitous language. No new synonyms. No impl jargon.
- **One behavior.** If `When` needs more than one event, split. If `Then` covers two unrelated outcomes, split.
- **Failure-aware.** Cover golden path AND edge cases AND failure modes AND ops scenarios (kill switch, disconnect, replay, recovery).

## Scenario shape

```md
## Feature — <capability>
Why this exists: <1-line business reason>.

### Scenario — <specific behavior>
Given <state>
When  <single event/action>
Then  <observable outcome>
And   <additional outcome>
```

## Process

1. **Orient.** Skim CONTEXT.md if present. Reuse domain vocab. Respect prior ADRs.
2. **Identify scope.** Single bounded context (subsystem) vs across contexts (platform). Cross-context scenarios trace flow through multiple owners — strategy → risk → OMS → venue.
3. **Draft scenarios.** Start golden path. Then edges. Then failures. Then ops.
4. **Quiz user.** Confirm coverage. Ask: which scenarios missing? Any scenarios mixing two behaviors? Any leaking impl detail?
5. **Iterate.** Tighten language until each scenario is unambiguous to a domain reader.
6. **Publish.** To path given by task envelope.

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| `When the function is called` | Impl detail | Describe the event the user/system causes |
| `Then the database is updated` | Internal state | Describe what becomes externally observable |
| `Given the system is initialized` | Vague | State the specific precondition |
| `When X happens and Y happens` | Two events in `When` | Split into two scenarios |
| `Then A and B and C and D` | Behavior bundle | Split or reduce to truly-coupled outcomes |
| `Given some valid input` | Not concrete | Specific values, specific shape |

## Anchor examples

Subsystem (market data gap recovery):

> Given a live order book at sequence 1050, when a delta with sequence 1052 is received, then a gap is detected, and the book enters recovery, and a snapshot reload is requested.

Subsystem (trade normalization):

> Given a valid Binance trade message, when the normalizer receives it, then a canonical Trade v1 event is emitted.

Cross-context platform (risk gating):

> Given a strategy proposes an oversized order, when risk evaluates it, then the order is rejected, and no venue submission occurs, and a rejection reason is recorded.

Cross-context platform (typical scenario set):

- approved strategy signal → venue-routed order
- oversized order → rejected by risk, no venue submission
- fill received → position updated + audit trail entry
- kill switch invoked → trading halts across active sessions
- venue disconnect → safe degradation, no crash

## Hand-off

Scenarios are the bridge from architecture to code:

- Each scenario → acceptance criterion on a tracer-bullet slice (`rex-plan-issue-writing`).
- Each scenario → integration / contract / unit test name (`rex-code-tests-*`).
- Scenarios that express invariants → fitness functions (`rex-code-tests-fitness-functions`).

Name scenarios so this hand-off is obvious. Ambiguous scenario name = ambiguous test name = ambiguous slice.
