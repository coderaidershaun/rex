---
name: rex-code-commenting
description: Hard rules for code comments. Default = no comment. Comment only WHY non-obvious, never WHAT. Distilled from "shocking truth about comments" + refine.dev. Use when writing/reviewing comments, JSDoc, docstrings, TODO markers, or when user asks "should I add a comment", "document this", "explain this code", or AI-generated code is being committed.
disable-model-invocation: false
user-invocable: true
---

# Code Commenting — Hard Rules

These rules HARD. No compromise. No "but my case different".

Default: **no comment**. Code self-document. Comment = design smell. Try fix design first.

Sister skill: `rex-code-philosophy`. Same complexity-averse, ergonomics-first frame.

## Iron law

**Comment only WHY non-obvious. NEVER WHAT.**

Code show what. Name show what. Structure show what. Comment job = capture what code *cannot* say:
- Why this approach over obvious alternative
- Hidden constraint (rate limit, legacy bug, vendor quirk)
- Trap future reader will hit
- External context (ticket ref, regulatory rule, benchmark result)

If WHY obvious from name + structure -> delete comment.

## Forbidden — delete on sight

- **Restate-what comments.** `// increment counter` above `counter++`. Insult reader.
- **Stale comments.** Lie silently for years. Worse than nothing. Wrong > missing.
- **Commented-out code.** Git remember. Delete. No "backup" blocks. No "old version below".
- **History comments.** `// 2024-03: added PayPal`. Use `git log` + commit message.
- **TODO/FIXME drift.** Vague promise = abandoned. File ticket OR fix now. No middle.
- **Design band-aid.** Comment explain why code weird -> refactor code, not annotate.
- **Section dividers in long fns.** `// --- validation ---`. Function too long. Extract instead.
- **AI-generated noise.** LLM produce verbose what-comments. Strip ruthlessly on commit.
- **Redundant docstring.** `@param userId — the user id`. Type say it. Delete.

## Allowed — narrow whitelist

1. **WHY non-obvious.** "Sort manual b/c stdlib O(n²) on near-sorted input. Bench shows 4x faster."
2. **Warning.** "DO NOT lower timeout < 30s. Vendor SLA breach."
3. **Edge case trap.** "API return null instead of [] on empty. Handled below."
4. **External ref.** "Per ADR-0007. See ticket PROJ-1234."
5. **Public API doc.** JSDoc/docstring on exported surface. Generated docs consume it.
6. **Algorithm conceptual frame.** "Boyer-Moore. Skip table built once, reused per search."

That's it. Nothing else.

## Self-document first (try in order)

Before write any comment, attempt these. In order:

1. **Rename.** `calc(p, t)` -> `calculatePriceWithTax(price, taxRate)`. Often kill comment need.
2. **Extract fn.** Inline block w/ explanation -> named fn. Name = comment.
3. **Replace magic number.** `if x > 3` -> `if x > AccessLevel.ADMIN`. Constant = comment.
4. **Replace bool flag.** `process(true, false)` -> `process({skipCache: true, retry: false})`.
5. **Decompose.** Big fn w/ section comments -> small fns. Section = fn name.
6. **Pattern consistency.** Every controller `validate -> process -> respond`. Pattern = doc.

Still need comment? OK. Now write minimal one.

## Decision flow

```
Want add comment?
  ├─ Restating what code do? -> DELETE. No comment.
  ├─ Can rename/extract/refactor remove need? -> DO THAT. No comment.
  ├─ History/TODO? -> git/ticket. No comment.
  ├─ WHY non-obvious / warning / edge / ref / public API? -> WRITE. Minimal.
  └─ Unsure? -> No comment. Reader read code.
```

## Maintenance discipline

- **Comment = code.** Edit comment when edit code. Stale = bug.
- **Review comments in PR same as code.** Catch drift early.
- **AI commit hygiene.** LLM-generated PR -> scan for what-comments. Strip before merge.
- **Comment older than function = suspect.** Re-read. Verify still true. Update or delete.

## Ergonomics check

Before commit, ask:

- **Necessary context, or excusing unclear code?** Latter -> refactor instead.
- **Will this comment lie in 6 months?** Yes -> delete or pin to constant.
- **Reader read code first or comment first?** Code. Comment supplement only.
- **Could a name carry this?** Yes -> rename.

Fail any -> reconsider.

## Quotables (carry these)

- "Comments rot faster than code."
- "Most comments don't improve quality. They create illusion of clarity."
- "Comment = duct tape for poor design."
- "Write code as if comments didn't exist. Then add the few that must exist."
- "Stale comment > silence in damage. Wrong worse than missing."
