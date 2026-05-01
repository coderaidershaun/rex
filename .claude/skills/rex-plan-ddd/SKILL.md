---
name: rex-plan-ddd
description: Discover the domain. Find events, commands, aggregates, bounded contexts, ownership, and ubiquitous language via EventStorming + DDD. Produce a context map. Use when user wants DDD, says "event storm", "bounded contexts", "domain model", "context map", "aggregates", or pipeline orchestrator dispatches `rex-plan-ddd` step.
disable-model-invocation: false
user-invocable: true
---

# rex-plan-ddd

Discover the shape of the domain. Find what happens (events), who decides (commands), where consistency lives (aggregates), where meaning splits (bounded contexts), who owns what.

I/O contract lives in `rex-utils-task-request`. Read inputs it gives you. Write outputs to paths it gives you. Don't assume shape — read envelope first.

## What good DDD is

- **Domain-led.** Vocabulary comes from the business, not the framework. "Order", "Fill", "Position" — not "Entity", "Repository", "Service".
- **Event-first.** Past-tense events are the spine. `OrderRejected`, not "the system rejects an order".
- **Boundaries precede services.** Bounded contexts come before container topology. C4 maps to bounded contexts later — not the other way around.
- **Ubiquitous within a context.** Same word = same meaning inside a bounded context. Across contexts, the same word may legitimately mean different things — that's a translation, not a bug.
- **Aggregates protect invariants.** Consistency boundary, not a data bag. If two things must change atomically, they are inside one aggregate.
- **Ownership is explicit.** Every bounded context has one owner. No shared mutable state across owners.

## What gets produced

| Artefact | Purpose |
|----------|---------|
| Domain event list | Past-tense things that happen, in time order |
| Command list | Imperatives that trigger events |
| Aggregates | Consistency boundaries that own state and emit events |
| Bounded contexts | Clusters of aggregates sharing a language + owner |
| Context map | Relationships between contexts (upstream / downstream, conformist, anti-corruption layer, shared kernel, partnership) |
| Ubiquitous-language entries | Domain terms with definitions, scoped to context |

## EventStorming flow

1. **Chaotic events.** List every domain event you can think of. Past tense. No order yet. Don't filter.
2. **Timeline.** Order events left-to-right by causality + time.
3. **Commands + actors.** What command (and who) caused each event? `SubmitOrderIntent` (Strategy) → `OrderIntentSubmitted`.
4. **Aggregates.** Group events that share consistency. Whatever owns the state that the events mutate.
5. **Bounded contexts.** Cluster aggregates that share a language and an owner. Draw the line.
6. **Context map.** Mark relationships between contexts. Where translation happens, where contracts must hold, where one context conforms to another.
7. **Glossary.** Pin every domain term you used. Context-scoped.

## Context relationship vocabulary

- **Upstream / downstream** — direction of model influence.
- **Conformist** — downstream accepts upstream's model as-is.
- **Anti-corruption layer (ACL)** — downstream translates upstream's model to protect its own.
- **Shared kernel** — small shared model both contexts maintain together. Use sparingly.
- **Partnership** — two contexts evolve coordinated changes together.
- **Customer / supplier** — downstream has influence over upstream's roadmap.

Pick one per relationship. If unsure, default to ACL — most defensive.

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| Events named in present tense (`OrderRejecting`) | Events are past tense | `OrderRejected` |
| One bounded context = one team | Conway's law dressed up as DDD | Bounded context = one shared language. Team alignment is a consequence, not the rule |
| Aggregate that spans contexts | Two languages, two consistency rules → drift | Split. Use events to coordinate |
| Context map with unlabelled arrows | Hides direction of dependency | Label every arrow w/ relationship type |
| Generic terms (`Item`, `Record`, `Manager`) | No domain meaning | Use the business word. If none exists, surface it as an open question |
| "Order" defined twice w/ different fields | Word collision across contexts | Note both definitions, scope to context, document the translation |

## Anchor examples

Subsystem (market data, Order Book State context):

> Events: `VenueConnected`, `SnapshotLoaded`, `DeltaApplied`, `GapDetected`, `RecoveryStarted`, `SnapshotReloaded`, `RecoverySucceeded`.
> Aggregate: `OrderBook` (per instrument).
> Bounded contexts surrounding it: Venue Connectivity, Raw Capture, Normalization, Distribution, Observability & Control.

Platform (trading engine):

> Bounded contexts: Market Data, Strategy, Risk, OMS / Execution, Venue Connectivity, Fill Processing, Position / Ledger, Reference Data, Operations / Control Plane.
> Context map: Strategy → (ACL) → Risk → OMS → Venue Connectivity. Fill Processing → Position / Ledger (downstream, conformist). Operations / Control Plane → all (partnership).

## Process

1. **Orient.** Skim CONTEXT.md, prior context maps, ADRs. Reuse vocab.
2. **Run EventStorming flow.** Steps 1–7 above. Quiz user at each step if domain is unfamiliar.
3. **Stress-test boundaries.** For each bounded context ask: does any aggregate inside need state from another context to enforce its invariants? If yes, the boundary is wrong.
4. **Stress-test ownership.** Two owners on one context = drift incoming. Split or pick one.
5. **Publish.** To path given by task envelope.

## Hand-off

- Bounded contexts → C4 container candidates (`rex-plan-c4-architecture`). Container ≠ context, but contexts inform topology.
- Aggregates + events → spec material (`rex-plan-sdd`) for state machines + event contracts.
- Cross-context flows → BDD platform scenarios (`rex-plan-bdd`).
- Context boundaries → contract test seams (`rex-code-tests-contract-seams`).
- Forbidden cross-context dependencies → fitness functions (`rex-code-tests-fitness-functions`) — e.g. "Strategy may not import Venue Connectivity".

A clean context map is the most reusable artefact in the pipeline. Spend the time.
