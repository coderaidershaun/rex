---
name: rust-ergonomic-refactoring
description: Refactor Rust code for better ergonomics, readability, and idiomatic style without sacrificing performance. Use this skill whenever the user asks to clean up Rust code, improve readability, make Rust code more idiomatic, refactor for ergonomics, simplify Rust logic, or asks about `#[inline]` usage. Also trigger when Rust code looks clunky, overly verbose, or uses anti-patterns — even if the user just says "clean this up" or "make this better" in the context of Rust files. Covers iterator chains, error handling, type conversions, builder patterns, smart use of `impl`, trait design, and performance-aware style choices.
disable-model-invocation: false
user-invocable: true
---

# Rust Ergonomic Refactoring

You refactor Rust code to be self-evidently readable — code that a competent Rustacean can scan and understand without mental gymnastics. The goal is code that reads like well-written prose: clear intent, minimal ceremony, no wasted motion. Every change you make must preserve (or improve) runtime performance.

## Core Philosophy

**Self-evident code > commented code.** If code needs a comment to explain what it does, refactor the code first.

Ergonomic Rust isn't about being clever. It's about removing friction between the reader's eyes and the code's intent. When you read ergonomic code, you think "of course" — not "oh, clever". The best refactoring is often deletion: removing scaffolding the compiler doesn't need and the reader doesn't want.

**Does NOT:** add runtime overhead, over-abstract, change public APIs without explicit approval.

## Process

1. Read the target file(s) completely — understand the data flow and ownership model before touching anything
2. **Check file length.** If any file exceeds 500 lines, it needs extra scrutiny. Long files often signal over-commenting, duplicated logic, or types that should be split into submodules. As a first pass on long files, check the comment-to-code ratio — if comments are contributing significantly to the line count, invoke the `rust-commenting` skill on that file to strip comments down to their minimal, essential form before continuing with structural refactoring. After comment trimming, re-check the line count. If still over 500, look for structural improvements: extract types into submodules, consolidate duplicated logic, simplify verbose patterns.
3. Identify patterns from the catalog below that apply
4. Apply transformations in order of impact: biggest readability wins first
5. Verify that no semantic changes were introduced — the refactored code must do exactly what the original did
6. If you're unsure whether a transformation preserves performance, leave a brief note and keep the original

Read `references/patterns.md` for the full catalog of ergonomic patterns with before/after examples covering iterator chains, error handling, type design, `#[inline]` usage, and more.

## When NOT to refactor

- Hot loops where micro-optimization matters and the current form was chosen deliberately
- `unsafe` blocks — don't rearrange these without deep understanding of the invariants
- FFI boundaries — C-compatible signatures are ugly for a reason
- Code with extensive test coverage that you'd need to rewrite — flag it for the user instead
- Generated code or macro output
