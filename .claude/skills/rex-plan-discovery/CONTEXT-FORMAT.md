# CONTEXT.md Format

## Structure

```md
# {Context Name}

{One or two sentence description of what this context is and why it exists.}

## Language

**Order**:
{A concise description of the term}
_Avoid_: Purchase, transaction

**Invoice**:
A request for payment sent to a customer after delivery.
_Avoid_: Bill, payment request

**Customer**:
A person or organization that places orders.
_Avoid_: Client, buyer, account

## Relationships

- An **Order** produces one or more **Invoices**
- An **Invoice** belongs to exactly one **Customer**

## Example dialogue

> **Dev:** "When a **Customer** places an **Order**, do we create the **Invoice** immediately?"
> **Domain expert:** "No — an **Invoice** is only generated once a **Fulfillment** is confirmed."

## Flagged ambiguities

- "account" was used to mean both **Customer** and **User** — resolved: these are distinct concepts.
```

## Rules

- **Be opinionated.** Multiple words for same concept -> pick best, list others as aliases to avoid.
- **Flag conflicts explicit.** Term used ambiguous -> call out in "Flagged ambiguities" w/ clear resolution.
- **Tight definitions.** One sentence max. Define what it IS, not what it does.
- **Show relationships.** Bold term names. Express cardinality where obvious.
- **Only terms specific to project context.** General programming concepts (timeouts, error types, utility patterns) don't belong even if used heavily. Before adding term, ask: unique to this context, or general programming concept? Only former belongs.
- **Group terms under subheadings** when natural clusters emerge. All terms in single cohesive area -> flat list fine.
- **Write example dialogue.** Conversation between dev + domain expert demonstrating how terms interact natural + clarifying boundaries between related concepts.

## Single vs multi-context repos

**Single context (most repos):** One `CONTEXT.md` at repo root.

**Multiple contexts:** `CONTEXT-MAP.md` at repo root lists contexts, where they live, how they relate:

```md
# Context Map

## Contexts

- [Ordering](./src/ordering/CONTEXT.md) — receives and tracks customer orders
- [Billing](./src/billing/CONTEXT.md) — generates invoices and processes payments
- [Fulfillment](./src/fulfillment/CONTEXT.md) — manages warehouse picking and shipping

## Relationships

- **Ordering → Fulfillment**: Ordering emits `OrderPlaced` events; Fulfillment consumes them to start picking
- **Fulfillment → Billing**: Fulfillment emits `ShipmentDispatched` events; Billing consumes them to generate invoices
- **Ordering ↔ Billing**: Shared types for `CustomerId` and `Money`
```

Skill infers structure:

- `CONTEXT-MAP.md` exists -> read to find contexts
- Only root `CONTEXT.md` exists -> single context
- Neither exists -> create root `CONTEXT.md` lazy when first term resolved

Multiple contexts -> infer which one current topic relates to. Unclear? Ask.
