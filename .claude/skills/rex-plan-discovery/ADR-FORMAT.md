# ADR Format

ADRs live in `docs/adr/`. Sequential numbering: `0001-slug.md`, `0002-slug.md`, etc.

Create `docs/adr/` lazy — only when first ADR needed.

## Template

```md
# {Short title of the decision}

{1-3 sentences: what's the context, what did we decide, and why.}
```

That's it. ADR can be single paragraph. Value = recording *that* decision was made + *why* — not filling out sections.

## Optional sections

Include only when add genuine value. Most ADRs won't need them.

- **Status** frontmatter (`proposed | accepted | deprecated | superseded by ADR-NNNN`) — useful when decisions revisited
- **Considered Options** — only when rejected alternatives worth remembering
- **Consequences** — only when non-obvious downstream effects need calling out

## Numbering

Scan `docs/adr/` for highest existing number. Increment by one.

## When to offer ADR

All 3 must be true:

1. **Hard to reverse** — cost of changing mind later meaningful
2. **Surprising without context** — future reader looks at code + wonders "why on earth did they do it this way?"
3. **Result of real trade-off** — genuine alternatives, picked one for specific reasons

Decision easy to reverse? Skip — you'll just reverse it. Not surprising? Nobody wonders why. No real alternative? Nothing to record beyond "did obvious thing."

### What qualifies

- **Architectural shape.** "Using monorepo." "Write model = event-sourced. Read model = projected into Postgres."
- **Integration patterns between contexts.** "Ordering + Billing communicate via domain events, not synchronous HTTP."
- **Tech choices carrying lock-in.** DB, message bus, auth provider, deploy target. Not every library — only ones that take a quarter to swap.
- **Boundary + scope decisions.** "Customer data owned by Customer context. Other contexts reference by ID only." Explicit no-s as valuable as yes-s.
- **Deliberate deviations from obvious path.** "Manual SQL instead of ORM because X." Anything where reasonable reader assumes opposite. Stops next engineer from "fixing" something deliberate.
- **Constraints not visible in code.** "Can't use AWS due to compliance." "Response times must be under 200ms due to partner API contract."
- **Rejected alternatives when rejection non-obvious.** Considered GraphQL, picked REST for subtle reasons -> record. Else someone suggests GraphQL again in 6 months.
