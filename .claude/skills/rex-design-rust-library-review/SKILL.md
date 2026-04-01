---
name: rex-design-rust-library-review
description: Review and document the confirmed Rust crates for a project during the rex design phase — checking latest stable versions, learning unfamiliar APIs, and writing usage guides for libraries that agents won't know well from training. Use this skill when the rex design process reaches the "library-review" step, when confirmed libraries need to be researched before implementation begins, or when the user says things like "review the crates", "check the libraries", "learn the dependencies", or "what versions should we use." This skill reads the confirmed libraries from onboarding inputs and produces a reference document that implementation agents can consult during coding. If no libraries have been confirmed, the skill exits gracefully with a note saying so.
disable-model-invocation: false
user-invocable: false
---

# Design: Rust Library Review

You review the confirmed crates for a Rust project so that implementation agents have accurate, project-relevant reference material to work from. You don't pick libraries — that happened during onboarding. You learn them.

You'll be told where to write the output (a file path like `design/library-review.md`) and given input files to read for context. Read them all first. Then research the confirmed libraries and write the review to the output path.

---

## When to do nothing

If the inputs don't contain confirmed libraries — either the libraries-and-sdks onboarding step hasn't been completed, or it explicitly says "none" — write a short output stating that no libraries were confirmed and this step was skipped. Don't invent a library list.

Similarly, if the only confirmed libraries are Rust standard library features (no external crates), note that and skip the review.

---

## Reading the inputs

### From libraries-and-sdks (primary input)
This is the list of confirmed crates. For each one, note:
- The crate name
- What it's for in this project
- Any version constraints the user specified
- Any configuration preferences (features, flags)

### From goal and scope
- What the project does — this determines which parts of each library are relevant. A library might have 50 features, but the project only needs 3 of them. Focus the review on what matters.

### From existing code (if present)
- Whether these crates are already in use — if so, what version? Are they used correctly? This shifts the review from "how to use it" to "what's changed and what should be updated."

---

## Classifying each crate

For each confirmed crate, make a judgment call: does the implementation agent need a usage guide, or just a version number?

### Well-known crates (version check only)

These are crates that any Rust-trained model knows inside and out. The agent doesn't need a tutorial — it needs to know the current stable version and any recent breaking changes worth flagging.

Examples of well-known crates:
- `tokio`, `async-std` — async runtimes
- `serde`, `serde_json`, `serde_yaml`, `toml` — serialization
- `clap` — CLI argument parsing
- `reqwest`, `hyper` — HTTP
- `tracing`, `log`, `env_logger` — logging/tracing
- `thiserror`, `anyhow` — error handling
- `rand` — random number generation
- `regex` — regular expressions
- `chrono`, `time` — date/time
- `uuid` — UUIDs
- `rayon` — parallelism
- `sqlx`, `diesel` — databases (core usage)
- `axum`, `actix-web`, `warp`, `rocket` — web frameworks (core usage)
- `itertools` — iterator extensions
- `bytes`, `crossbeam` — concurrency primitives

For these, fetch the latest stable version from crates.io and note any major version jumps or notable recent changes. That's it — don't write a usage guide the agent already knows.

### Less-known crates (full review needed)

These are crates where the implementation agent is likely to struggle, hallucinate APIs, or use outdated patterns. The cost of not reviewing these is high — the agent will waste time guessing, write code against imaginary APIs, and produce output that doesn't compile.

Signs a crate needs full review:
- It's domain-specific (e.g., `hdrhistogram`, `rkyv`, `speedy`, `glommio`)
- It's relatively new or niche (low download count, post-2023 release)
- It has an unconventional API surface (macro-heavy, builder patterns, complex generics)
- The project needs features beyond basic usage (advanced configuration, custom implementations)
- You genuinely aren't confident you know its current API accurately

Be honest with yourself here. If you're not sure whether you know a crate's API well enough to write correct code against it, review it. The downside of reviewing something you already know is wasted context. The downside of skipping something you don't know is broken code.

---

## Researching crates

### Getting the latest stable version

For every confirmed crate (well-known or not), fetch the latest stable version. Use the crates.io API:

```
https://crates.io/api/v1/crates/{crate_name}
```

The response includes `crate.max_stable_version` — that's the one you want. Also check `crate.newest_version` to see if there's a pre-release worth noting.

If the user specified a version constraint during onboarding, note whether it's compatible with the latest stable.

### Deep-diving unfamiliar crates

For crates that need a full review, your goal is to produce reference material that lets an implementation agent write correct code on the first try. Use these sources:

1. **docs.rs** — the definitive API reference. Fetch `https://docs.rs/{crate_name}/latest/{crate_name}/` for the module index. Read the top-level docs and the most relevant modules.

2. **The crate's README / examples** — often the best source of idiomatic usage patterns. Check the repository (linked from crates.io) for an `examples/` directory.

3. **Context7 / find-docs** — if the context7 or find-docs skills are available, use them to pull structured documentation. They're particularly good for getting usage examples.

4. **The crate's repository** — for understanding real-world patterns, check the repo's tests and examples.

Focus your research on the parts of the library the project will actually use. If the project is building a CLI that uses `indicatif` for progress bars, you don't need to review every widget — focus on `ProgressBar`, `MultiProgress`, and the style API.

---

## What to capture in a full review

For each crate that gets a deep review, cover:

### Core concepts
What mental model does this library use? What are the key types, traits, and patterns? This is the "aha" section — if the agent reads only this, it should understand *how the library thinks*.

### Key types and functions
The specific structs, enums, traits, and functions the project will use. Include their signatures and what they do. Don't exhaustively list every method — focus on what's relevant to the project.

### Usage examples
Concrete, compilable examples showing the patterns the project needs. These should be realistic, not toy examples. If the project is using `sqlx` for PostgreSQL queries with compile-time checking, show that — not a trivial SQLite in-memory example.

### Feature flags
If the crate has cargo features the project needs to enable, document them. Many crates ship with a minimal default feature set.

### Common pitfalls
Things that are easy to get wrong — lifetime issues, required feature flags, initialization order, async runtime compatibility. If you've seen (or can anticipate) common mistakes with this crate, flag them.

### Integration notes
How this crate works alongside the project's other dependencies. Does it need a specific async runtime? Does it conflict with anything? Are there compatibility crates needed (e.g., `serde` integration features)?

---

## Writing the output

```markdown
# Library Review

**Date:** YYYY-MM-DD

## Summary
Brief overview — how many crates confirmed, how many needed deep review, any version concerns.

## Version Reference

| Crate | Latest Stable | Project Constraint | Status |
|-------|--------------|-------------------|--------|
| tokio | 1.x.x | none | current, well-known |
| some-niche-crate | 0.x.x | none | reviewed below |

## Well-Known Crates

### [crate-name] — v{latest}
Any notable recent changes, migration notes, or version concerns. If nothing notable, a single line is fine: "Current stable, no concerns."

(Repeat for each well-known crate)

## Detailed Reviews

### [crate-name] — v{latest}

#### What it does
One-paragraph description in the context of this project.

#### Core concepts
The mental model — key types, traits, patterns.

#### Key API surface
The types and functions the project will use, with signatures.

#### Usage examples
```rust
// Realistic examples relevant to the project
```

#### Feature flags
Required features for this project's use case.

#### Pitfalls
Things to watch out for.

#### Integration
How it works with the project's other crates.

(Repeat for each reviewed crate)

## Crates Not Reviewed
Any confirmed crates that were skipped and why (e.g., "user said 'undecided' — not yet confirmed").

## Notes
Anything else discovered during research — deprecation warnings, better alternatives spotted, compatibility concerns between crates.
```

Write to the output path you were given (relative to the project's rex directory).
