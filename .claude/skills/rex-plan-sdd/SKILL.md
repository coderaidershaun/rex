---
name: rex-plan-sdd
description: Write specs that formalize the load-bearing parts of a system — canonical schemas, state machines, invariants, event contracts, versioning, operational semantics. Use when user wants spec, says "spec this", "schema", "state machine", "invariants", "contract", or pipeline orchestrator dispatches `rex-plan-sdd` step.
disable-model-invocation: false
user-invocable: true
---

Spec-driven development. Pin down the parts of the system that everything else relies on. Precise, versioned, authoritative.

I/O contract lives in `rex-utils-task-request`. Read inputs it gives you. Write outputs to paths it gives you. Don't assume shape — read envelope first.

## What good SDD is

- **Authoritative.** Spec is the truth. Code conforms to spec, not the reverse.
- **Versioned.** Every canonical type carries a version (`Trade v1`, `OrderIntent v2`). Breaking changes bump the version.
- **Precise.** Every field has a type, every state has legal transitions, every invariant has a clear violation case.
- **Domain-language.** Names match project vocab (CONTEXT.md). No new synonyms. Same word = same meaning everywhere.
- **Behavioral, not implementational.** Specifies what holds, not how it's coded. No file paths, no module names, no language-specific types.
- **Testable.** Every invariant becomes a fitness function. Every state machine becomes a unit test. Every schema becomes a contract test.

## What gets specified

| Kind | Examples |
|------|----------|
| Canonical schemas | `Trade v1`, `OrderBookSnapshot v1`, `OrderIntent v1`, `Fill v1`, `RiskDecision v1` |
| State machines | Order book lifecycle, order lifecycle, connection lifecycle |
| Invariants | Sequence numbers strictly increase, deltas never apply before snapshot, rejected orders never reach venue |
| Event contracts | Required fields, ordering guarantees, idempotency keys, retry semantics |
| Versioning rules | When to bump, deprecation policy, dual-write windows |
| Operational semantics | Reconnect policy, gap recovery, replay determinism, persistence ordering |

## Canonical discipline

"Canonical" = the agreed common form across multiple representations.

- Use it when multiple sources / representations exist AND one form is the agreed standard. (Binance trade vs Coinbase trade → canonical `Trade v1`.)
- Don't use it for: a private helper type, a one-off transformation, an unstable intermediate, anything that isn't shared.
- Distinguish raw (source bytes) vs canonical (normalized shared shape) vs authoritative (truth-owner). They are not the same thing.

## Spec shape

```md
## <TypeName> v<N>
Purpose: <1-line why this exists>.
Owner: <bounded context>.

### Fields
- `field_name` : <type> — <meaning + units + constraints>.
- ...

### Invariants
- <statement that must always hold>.
- ...

### Versioning
- Compatible with v<N-1>: <yes/no, with notes>.
- Breaking change rules: <when to bump>.
```

```md
## <Aggregate> lifecycle
States: <S1>, <S2>, <S3>.
Initial: <S1>. Terminal: <Sn>.

### Legal transitions
- <S1> --<event>--> <S2> [guard: <condition>]
- ...

### Forbidden transitions
- <S2> --*--> <S1> (irreversible)
- ...
```

## Process

1. **Orient.** Skim CONTEXT.md, prior specs, ADRs touching the area. Reuse vocab.
2. **Pick the load-bearing things.** The 3–10 types / state machines / invariants that everything else depends on. Skip incidental types.
3. **Draft schema first, then state, then invariants.** Schema gives shared vocab. State gives lifecycle. Invariants are the never-violate rules.
4. **Stress-test.** For each invariant ask: how would this fail? What violates it? Does the spec prevent it or just discourage it?
5. **Quiz user.** Confirm versioning rule. Confirm raw vs canonical vs authoritative split. Surface ambiguity.
6. **Publish.** To path given by task envelope.

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| `Trade` (no version) | Future breaking change has nowhere to go | `Trade v1` |
| `price: number` | No precision, no unit | `price: decimal, scaled by quote_currency_decimals` |
| "Trade is normalized data" | Vague — what's the rule? | Field-by-field shape + invariants |
| State machine w/ implicit "back to start" | Transitions undocumented = bugs | List every legal transition explicitly |
| `Fill` defined in two places | Two truths = drift | One canonical definition; everywhere else references it |
| "Use canonical form" w/o defining it | Word-as-magic | Define canonical shape; mark as canonical only if it actually is |

## Anchor examples

Subsystem (market data):

> `Trade v1` — fields: `venue`, `instrument`, `venue_symbol`, `price`, `quantity`, `event_time`, `ingest_time`, `sequence?`. Invariant: `price > 0`, `quantity > 0`, `event_time <= ingest_time`.

> Order book lifecycle states: `Uninitialized`, `LoadingSnapshot`, `Live`, `Recovering`. Legal: `Uninitialized → LoadingSnapshot → Live`. `Live → Recovering` on gap. `Recovering → LoadingSnapshot → Live` on recovery.

Platform (trading):

> `OrderIntent v1` — fields: `intent_id`, `strategy_id`, `instrument`, `side`, `quantity`, `limit_price?`, `time_in_force`. Invariant: `quantity > 0`. Versioning: bump on any field rename, removal, or semantic change.

> `RiskDecision v1` — variants: `Approved { decision_id, intent_id }`, `Rejected { decision_id, intent_id, reason_code, detail }`. Invariant: every `OrderIntent` produces exactly one `RiskDecision`.

## Hand-off

Specs are the foundation everything else stands on:

- Schemas → contract tests (`rex-code-tests-contract-seams`).
- Invariants → fitness functions (`rex-code-tests-fitness-functions`).
- State machines → unit tests of transitions (`rex-code-tests-unit-testing`).
- Event contracts → integration tests against real systems (`rex-code-tests-integration-testing`).
- Schemas + states inform DDD context boundaries (`rex-plan-ddd`) and C4 component design (`rex-plan-c4-architecture`).
- Behaviors implied by state machines → BDD scenarios (`rex-plan-bdd`).

If a spec is right, the rest of the pipeline writes itself. If it's wrong, everything downstream rots.
