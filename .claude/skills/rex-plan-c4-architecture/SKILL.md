---
name: rex-plan-c4-architecture
description: Communicate architecture at the right resolution using C4 (Context, Containers, Components, Code). Tool for shared reasoning, not domain discovery. Use when user wants C4 diagrams, says "context diagram", "container diagram", "C4", "architecture overview", or pipeline orchestrator dispatches `rex-plan-c4-architecture` step.
disable-model-invocation: false
user-invocable: true
---

# rex-plan-c4-architecture

Communicate architecture at progressive zoom levels. Tool for shared reasoning, not domain discovery — DDD found the boundaries already, C4 expresses them.

I/O contract lives in `rex-utils-task-request`. Read inputs it gives you. Write outputs to paths it gives you. Don't assume shape — read envelope first.

For the actual diagram syntax, defer to `rex-utils-mermaid-diagrams`. This skill is about what to draw and at which level — not how to render it.

## What good C4 is

- **Right level for the audience.** Each level answers one question. Mixing levels confuses readers.
- **Containers are runtime units, not crates.** A container is a deployable / process / runtime boundary. A crate is a compile boundary. One container can be many crates.
- **Bounded context informs container, but doesn't equal it.** Use DDD's context map to choose containers, not to name them by accident.
- **Diagrams are conversation, not contract.** Update when the architecture changes. Stale diagrams are worse than no diagrams.
- **Lower levels are optional.** C1 + C2 are usually enough. C3 only for containers worth zooming. C4 (code) only when the structure is genuinely subtle.

## Levels

| Level | Audience | Question answered | Don't include |
|-------|----------|-------------------|---------------|
| C1 — Context | Anyone (incl. non-engineers) | What surrounds the system? Who uses it? What does it talk to? | Internal services, crates, parsers, state-machine internals |
| C2 — Containers | Engineers, ops, architects | What major runtime units exist? Where does each capability live? | Internal components, classes, code-level concerns |
| C3 — Components | Engineers working in that container | How is one container internally shaped? | Code maps, every type, framework noise |
| C4 — Code | Engineers touching that subtle code | How is one critical component structured? | Anything that isn't load-bearing |

## What each level shows

**C1 — System Context.** System boundary. External actors (users, operators, roles). External systems. Major interaction flows. Single page. One box for the system. Boxes around it for everything outside.

**C2 — Containers.** Runtime units inside the system boundary. Each = one process / service / DB / queue / cache. Arrows = primary calls + data flows. Label every arrow with what flows. Annotate persistence vs compute.

**C3 — Components.** Zoom inside one C2 container. Show its internal building blocks (subsystems, modules, ports, adapters). Stop at "thing a reader can name and find in code".

**C4 — Code.** Optional. Reserve for: state machines, lifecycle logic, port/adapter trait structures, replay sequencing. Skip otherwise.

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| C1 with internal microservices on it | Wrong level — confuses external readers | Move them to C2 |
| C2 = "one box per crate" | Conflates compile-time w/ runtime | One box per runtime unit. Note crate composition separately if needed |
| C3 with every helper class | Reader drowns in detail | Stop at component grain, not symbol grain |
| Unlabelled arrows | Direction known, meaning unknown | Label what flows + protocol if it matters |
| Diagram drifts from code | Becomes lying documentation | Update when topology changes; mark deltas in PR |
| One mega-diagram across all levels | Mixes audiences | One diagram per level. Cross-link them |
| Container named after the team | Conway's law leaking | Name by capability, not by org |

## Process

1. **Orient.** Read PRD (intent), DDD context map (boundaries), spec (contracts). Skim ADRs.
2. **Decide which levels you need.** Default: C1 + C2. Add C3 only for containers that are non-obvious or own critical logic. Add code-level only if a state machine / lifecycle is subtle.
3. **Draw C1.** One system box. External actors + systems around it. Arrows w/ short labels.
4. **Draw C2.** Pick runtime units from DDD bounded contexts + non-domain concerns (stores, buses, ops API). Show major data flows, not every call.
5. **Draw C3 (per chosen container).** Internal components. Stop at named-thing grain.
6. **Draw code-level (optional).** Only the part worth modelling.
7. **Cross-check w/ DDD + spec.** Every bounded context owner should be visible. Every spec'd contract should sit on a labelled arrow.
8. **Publish.** To path given by task envelope. Use mermaid syntax via `rex-utils-mermaid-diagrams` unless task specifies otherwise.

## Anchor examples

Subsystem (market data, C2 containers):

> Collector Gateway → Raw Capture Writer (raw frames) → Normalizer Service (canonical events) → Publisher (consumers). Side branches: Book Engine (consumes normalized deltas, owns book state), Replay Service (reads Raw Store, replays), Ops API (control plane). Stores: Raw Store, Normalized Store.

Subsystem (market data, C3 inside Normalizer Service):

> Message Classifier → Venue Decoder → Canonical Mapper → Validation Engine. Side: Error Quarantine. Output: Publisher Adapter.

Subsystem (market data, C3 inside Book Engine):

> Snapshot Loader, Delta Applicator, Sequence Tracker, Recovery Coordinator, Book State Store. Recovery Coordinator triggered by Sequence Tracker on gap.

Platform (trading engine, C2 containers):

> Market Data, Strategy Runtime, Risk Engine, OMS, Execution Gateway, Fill Processor, Position / Ledger, Control Plane. Stores: Order Store, Fill Store, Position Store. Buses: Market Data Bus, Order Event Bus.

## Container vs crate

- Container = runtime unit. Lives in C4.
- Crate = Rust package boundary. Lives in `Cargo.toml`.
- One container may be many crates (e.g. OMS container = `oms-core` + `oms-api` + `oms-persistence`).
- One crate is rarely many containers — that's a smell.

If a reviewer says "is this a container or a crate", the diagram is unclear. Fix the labelling before you fix the architecture.

## Hand-off

- C2 container topology → infrastructure work (deployment, networking, observability).
- C2 + C3 boundaries → contract test seams (`rex-code-tests-contract-seams`).
- C2 forbidden dependencies → fitness functions (`rex-code-tests-fitness-functions`) — e.g. "Strategy container may not import Venue Connectivity types".
- C3 components → module structure for `rex-plan-issue-writing` slices to target.
- Code-level state machines → unit tests of transitions (`rex-code-tests-unit-testing`).

C4 is the architecture's mirror. If the code stops matching the diagram, one of them is lying. Find out which.
