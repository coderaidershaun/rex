---
name: rex-clean-fitness-functions
description: Audit `tests/fitness/`. Remove fitness tests that are overkill, stale, or misclassified. Consolidate groups testing the same invariant. Keeps the fitness suite small + immortal + fast. Use when user says "clean fitness functions", "audit fitness suite", "consolidate fitness tests", "tests/fitness too big", or pipeline orchestrator dispatches `rex-clean-fitness-functions` step.
disable-model-invocation: false
user-invocable: true
---

# rex-clean-fitness-functions

Fitness functions should be **few, immortal, fast**. Rules from `rex-code-tests-fitness-functions`. This skill enforces that posture by pruning + consolidating `tests/fitness/`.

I/O contract lives in `rex-utils-task-request`. Updates `tests/fitness/` and emits an audit report.

## What stays

A test belongs in `tests/fitness/` only if **all four** are true:

- **True invariant** — property must hold across all inputs, all time, all refactors.
- **Architectural** — protects boundary, conservation law, reconstruction guarantee, schema compat. Not a single bug.
- **Fast enough to run every CI** — minutes, not hours. Disabled = dead.
- **Deterministic** — proptest seed reproducible. No flakes.

Anchor good examples: orderbook reconstruction (snapshot + updates ≡ streamed state), money conservation across transfers, schema backward-compatibility, no-cross-context-import rules.

## What to remove

| Symptom | Why it's overkill | Where it belongs instead |
|---------|-------------------|--------------------------|
| Tests one specific scenario, not a property | Regression test wearing fitness hat | `tests/unit/` or `tests/integration/` |
| Asserts a single value or fixed sequence | Not exploring input space | `tests/unit/` |
| Hits real APIs / DB / network | Side-effectful, slow, flaky | `tests/integration/` |
| Disabled (`#[ignore]`, commented out, skipped in CI) | Dead test = lying invariant | Delete. If it mattered, fix it. If not, gone |
| Behavior also covered by a stronger fitness check | Duplicate | Delete the weaker one |
| Property no longer matters (architecture moved past it) | Stale | Delete. Don't archive |
| Property holds trivially under current types | Compiler enforces it; test earns nothing | Delete. Type system already won |
| No proptest, no input generation, just a fixed example | Not a fitness function | Promote to unit, or expand to property |

If a test matches any row → remove (or move to its rightful suite). Don't keep "just in case".

## What to consolidate

Groups that should be **one** test:

- **Same invariant, multiple narrow checks.** Five tests each asserting one slice of orderbook reconstruction → one property test exploring the full input space.
- **Same generator, different assertions.** Four tests build the same input then check four properties → one proptest body, all assertions inside.
- **Conjugate invariants sharing input distribution.** "Sum stays constant" + "sum never negative" → combine.
- **Schema-compat tested per type.** Twenty tests, one per type → single proptest parameterised by type registry.

Consolidation rule: if removing N tests and writing one stronger test gives equal-or-better coverage at lower CI cost, consolidate. New proptest must run with at least the seed count of the most-thorough original.

## What stays even if it looks redundant

- Two invariants that *appear* similar but have **different failure modes** under fault injection. Keep both.
- Critical-path invariants (money, audit, recovery determinism) — bias toward keeping. False-confidence cost > extra-test cost.
- Anything tagged `#[fitness(critical)]` or referenced from an ADR. Escalate before removing.

## Process

1. **Inventory.** Walk `tests/fitness/`. List every test file + function. Note: proptest? deterministic? CI runtime? `#[ignore]`?
2. **Classify each.** Stays / remove / consolidate. Record reason per test.
3. **Cross-check ADRs.** Any test referenced by an ADR → don't remove without escalation.
4. **Remove.** Apply deletes. Move misclassified tests to `tests/unit/` or `tests/integration/` (don't just delete those — they may earn their place there).
5. **Consolidate.** Replace groups with stronger combined property tests. Preserve seed counts.
6. **Verify.** Run `cargo test --test fitness` (or project's fitness target). All remaining tests pass. CI runtime should drop.
7. **Emit audit report.**
8. **Publish.** Updated `tests/fitness/` + audit report to paths given by task envelope.

## Audit report shape

```md
# Fitness suite audit — <date>

## Removed (N)
- `tests/fitness/foo.rs::test_specific_scenario` — single-scenario regression. Moved to `tests/unit/foo.rs`.
- `tests/fitness/bar.rs::test_disabled_thing` — `#[ignore]` since 2025-09. Deleted.

## Consolidated
- `orderbook_reconstruction_*` (5 tests) → `orderbook_reconstruction_property` (1 proptest, 10k cases). Coverage equal or stronger.

## Kept under review
- `tests/fitness/replay_determinism.rs` — slow (45s) but critical. Recommend faster generator, not removal.

## Escalations (need user)
- `tests/fitness/legacy_x.rs` referenced by ADR-0007. Removal blocked.

## CI runtime
- Before: 7m 12s
- After:  2m 48s
```

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| Deleting silently | No audit trail. Reviewer can't sanity-check | Always emit audit report |
| Removing slow tests because slow | Slow ≠ wrong. Critical-path invariants earn their seconds | Optimise generator first. Remove only if redundant |
| Consolidating across unrelated invariants | One failure mode hides another | Only consolidate when input space + failure mode align |
| Promoting fitness → unit just to delete the file | Skill is for *moving* misclassified tests, not erasing | Move properly. Note in report |
| Removing without running suite afterwards | Could leave broken state | Run suite. Confirm green |
| Touching tests outside `tests/fitness/` without reason | Out of scope | Stay in fitness suite. Other suites have their own clean skills |

## Hand-off

After cleaning:

- `rex-code-tests-fitness-functions` rules now reflect actual suite. Future additions held to the same bar.
- Faster CI → faster autopilot iteration.
- Audit report → input to next refinement / review gate so user sees what was pruned.

Goal: the fitness suite earns its CI cost on every run. If it doesn't, it's not a fitness suite — it's a graveyard.
