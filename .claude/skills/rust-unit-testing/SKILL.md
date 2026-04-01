---
name: rust-unit-testing
description: Write minimal, targeted Rust unit tests to verify code correctness — then clean up after. Use this skill when verifying Rust code works correctly, after writing complex logic (orderbook matching, state machines, parsers, financial calculations), when the user says "test this", "make sure this works", "verify the logic", "write a quick test", or when working on code where subtle bugs would be costly. Also trigger after implementing non-trivial algorithms or data structures where a quick sanity check prevents hours of debugging later.
disable-model-invocation: false
user-invocable: true
---

# Rust Unit Testing

You write the fewest tests needed to prove the code works, then you clean up after yourself. Tests are scaffolding, not furniture — they go up to build confidence, then come down so they don't clutter the codebase.

*Note: This is lighter-weight work — suitable for faster model tiers (e.g., Sonnet).*

---

## Philosophy

Most codebases drown in tests that nobody reads, nobody maintains, and that break every time someone renames a field. This project takes a different approach:

- **Tests are verification tools, not permanent fixtures.** Write them, run them, confirm the code works, then remove them unless there's a strong reason to keep one.
- **Fewer tests, better tests.** One test that exercises the critical path is worth more than twenty that test getters and setters.
- **Tests shine where complexity hides bugs.** An orderbook matching engine, a financial calculation, a state machine with tricky transitions — these deserve tests. A simple struct with three fields does not.

---

## When to test

### Tests are valuable for:
- **Algorithmic correctness** — matching engines, sorting, parsing, mathematical operations
- **State machines** — transitions, edge cases, invalid state rejection
- **Serialization/deserialization** — roundtrip correctness, edge cases in formats
- **Business logic with rules** — calculations with specific expected outputs
- **Boundary conditions** — overflow, empty inputs, off-by-one scenarios
- **Code you're not 100% confident in** — if you have any doubt, test it

### Tests are usually unnecessary for:
- Simple structs and enums with derived traits
- Thin wrappers around standard library functions
- Code that's just plumbing (reading config, passing data between modules)
- Anything where `cargo check` already catches the class of bug you'd test for

---

## How to write tests

### Keep them inline

Use `#[cfg(test)]` modules at the bottom of the file being tested. This keeps tests close to the code they verify and makes cleanup easy.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_limit_orders_by_price_time_priority() {
        // One focused test that exercises the core logic
    }
}
```

### Keep them simple

A good test has three lines of thought:
1. **Setup** — create the inputs
2. **Act** — call the function
3. **Assert** — check the output

If your test needs more than ~15 lines, the code under test might need refactoring, not more test infrastructure.

### Keep them few

For most code changes: **0, 1, or 2 tests.** That's it.

- **0 tests**: The code is straightforward, types enforce correctness, `cargo check` is sufficient
- **1 test**: There's one critical path worth verifying (the happy path with realistic inputs)
- **2 tests**: There's a happy path and one important edge case (e.g., empty input, boundary value)

For genuinely complex logic (matching engines, financial calculations, protocol implementations), you might write 3-5 tests to cover the key scenarios. But this is the exception, not the rule.

### Name tests descriptively

The test name should read as a specification:
- `matches_limit_orders_by_price_time_priority` — good
- `test_matching` — useless
- `it_works` — no

---

## The workflow

### Step 1: Assess complexity

Before writing any tests, decide how many you need:

| Complexity | Tests | Examples |
|---|---|---|
| Trivial | 0 | New struct, simple enum, config plumbing |
| Moderate | 1-2 | Data transformation, validation logic, simple parsing |
| Complex | 2-5 | Matching engine, state machine, financial calc, protocol impl |

### Step 2: Write and run

Write the tests in a `#[cfg(test)]` module. Run them:

```bash
cargo test --lib
```

If targeting a specific module:

```bash
cargo test --lib module_name
```

Fix any failures. The point is to catch bugs now, not to build a regression suite.

### Step 3: Decide what stays

After all tests pass, make the keep/remove decision for each test:

**Keep a test if:**
- It verifies complex logic where a future change could silently introduce a bug
- The correctness property it checks isn't obvious from reading the code
- It serves as executable documentation for tricky business rules
- Removing it would make you nervous about future refactors

**Remove a test if:**
- It verified the code works during development but doesn't add ongoing value
- The logic it tests is simple enough that a bug would be caught by the type system or code review
- It's testing implementation details that will break on any refactor
- It duplicates confidence you already have from another test

### Step 4: Clean up

Remove the tests you've decided to discard. If you're removing all tests from a file, remove the entire `#[cfg(test)]` module — don't leave an empty one behind.

If you kept one or two tests, make sure they're clean, well-named, and standalone. They should make sense to someone reading them six months from now without context.

Run `cargo test --lib` one final time to confirm the remaining tests (if any) still pass after cleanup.

---

## What a kept test looks like

If you decide to keep a test, it should be a gem — small, clear, and obviously valuable:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bid_at_higher_price_fills_against_lower_ask() {
        let mut book = OrderBook::new();
        book.add_order(Order::limit(Side::Ask, dec!(100.0), dec!(1.0)));
        book.add_order(Order::limit(Side::Bid, dec!(101.0), dec!(1.0)));

        assert_eq!(book.trades().len(), 1);
        assert_eq!(book.trades()[0].price, dec!(100.0));
        assert_eq!(book.trades()[0].quantity, dec!(1.0));
    }
}
```

No test utilities, no fixtures, no setup functions. Just the test.
