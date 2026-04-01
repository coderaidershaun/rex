---
name: rex-onboarding-checklist
description: Synthesize all onboarding inputs into a comprehensive checklist of everything the project must address during design, architecture, and planning phases — milestones, objectives, and tasks. Use this skill when the rex onboarding process reaches the "checklist" step, when the user wants to see everything the project needs to cover before building begins. Also trigger when the user says things like "what do we need to plan for", "give me the checklist", "what must the design include", "summarize what we need to do", or "what should planning cover."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Checklist

You've gathered everything — the goal, scope, risks, resources, expertise, success criteria, and ideas. Now distill all of that into a single checklist that tells the design, architecture, and planning phases exactly what they must address. You're not doing the work — you're defining what the work must include.

**Read all available input files first** — goal, scope, existing code, libraries, research, resources, user expertise, UAT, known risks, success measures, environment variables, idea generation, skill building. Every onboarding document that exists is relevant. Then work with the user and use the `rex-cli checklist` CLI to populate the checklist.

---

## What this is (and isn't)

This is a **completeness check** — a structured list of everything the project's planning phases must account for, derived from what was learned during onboarding.

It **is**:
- A checklist that design and architecture must satisfy before implementation begins
- Milestones that the planning phase should define
- Objectives that must be met for the project to succeed
- Tasks that must appear somewhere in the project plan
- Constraints and requirements that must be respected throughout

It **is not**:
- A project plan (that comes later)
- An architecture document (that comes later)
- A task breakdown with estimates (that comes later)
- Execution of any of the items — you're saying "this must be planned for," not "here's how to do it"

Think of it as the acceptance criteria for the planning phase itself. If someone completed design and architecture but missed something on this checklist, they'd need to go back.

---

## Phase classification

Every checklist item must be assigned to exactly one phase:

- **`design`** — Discovery-related actions. Things that must happen when modules, mermaid diagrams, high-level structs, data models, system boundaries, interface contracts, and architectural decisions are being planned. This is the phase where the system's shape is discovered and defined. Examples: defining data models, drawing module boundaries, choosing architectural patterns, designing API contracts, creating system diagrams, prototyping unknowns, resolving research questions.

- **`planning`** — Execution-related actions. Things that must happen when milestones, objectives, and tasks are being decided. This is the phase where the work is organized into deliverables and schedules. Examples: defining milestones, setting objectives, breaking work into tasks, establishing success criteria checkpoints, sequencing dependencies, assigning risk mitigations to specific milestones.

When in doubt: if the item is about **what the system looks like** → `design`. If the item is about **how the work gets done** → `planning`.

---

## How to approach it

Before talking to the user, read every onboarding input and extract what each one implies for planning. Look for:

- **From the goal:** What must the design achieve? What's the core thing that can't be compromised?
- **From the scope:** What's in, what's out, what are the boundaries the architecture must respect?
- **From existing code:** What must be accounted for, integrated with, or migrated from?
- **From libraries and SDKs:** What dependencies must the architecture incorporate? Are there integration points to design for?
- **From research:** What unknowns were flagged? What must be investigated or prototyped during design?
- **From resources:** What tools, APIs, or external systems must the design accommodate?
- **From user expertise:** What does the user know deeply that should shape the approach? What are they less familiar with that needs more careful planning?
- **From UAT:** What must the final deliverable look like? What does the user need to test? This shapes what milestones to define.
- **From known risks:** What must the design mitigate? What contingencies should the plan include?
- **From success measures:** What must be verifiable? What metrics or criteria should milestones check against?
- **From environment variables:** What configuration, secrets, or infrastructure must be planned for?
- **From idea generation:** Which accepted ideas add requirements to the plan? Which parked ideas should the architecture leave room for?
- **From skill building:** What specialist concerns must the design address?

---

## How to run the conversation

Present your draft checklist to the user, organized by phase and category. For each item, briefly note which onboarding input(s) it came from so the user can trace the reasoning.

Then ask:
- "Is anything missing? Anything you know needs to happen that didn't come up during onboarding?"
- "Is anything here that you'd consider out of scope or unnecessary?"
- "Are the milestones at the right granularity — too coarse, too fine?"

This conversation should be focused. You've already done the synthesis — the user is reviewing and adjusting, not starting from scratch. If they add items, capture the reasoning. If they remove items, note why so downstream phases don't re-add them.

Keep the conversation proportional to the project. A small project might need a quick confirmation pass. A complex one might need real discussion about priorities and phasing.

---

## Writing the output using the CLI

Once the user approves the checklist, use the `rex-cli checklist` CLI commands to populate `checklist.json`. **Do not write the JSON file directly** — use the CLI for all mutations.

### Step 1: Initialize the checklist

```bash
rex-cli checklist init
```

This creates an empty `checklist.json` in the active project's onboarding directory. Optionally pass `--date YYYY-MM-DD` to set a specific date.

### Step 2: Add items

Use `rex-cli checklist add` for each item. Every item needs `--category`, `--id`, `--title`, and `--description`. All items except `out-of-scope` also require `--phase`.

```bash
rex-cli checklist add \
  --category design-must-haves \
  --id "design-data-models" \
  --title "Define core data models" \
  --description "Establish the primary data structures. Source: goal, scope" \
  --phase design
```

```bash
rex-cli checklist add \
  --category planning-milestones \
  --id "plan-milestone-mvp" \
  --title "MVP milestone" \
  --description "Define what the MVP delivers. Source: uat, success-measures" \
  --phase planning
```

```bash
rex-cli checklist add \
  --category out-of-scope \
  --id "oos-mobile-app" \
  --title "Mobile application" \
  --description "Excluded per scope discussion. Source: scope"
```

### Step 3: Set the context

```bash
rex-cli checklist set-context "Derived from onboarding inputs. User emphasized X, adjusted Y during review."
```

### Available categories

| CLI value                    | Phase default | Description |
|------------------------------|---------------|-------------|
| `design-must-haves`         | `design`      | Architectural decisions, data models, interface contracts |
| `architecture-constraints`  | `design`      | Non-negotiable technology/compatibility/performance constraints |
| `planning-milestones`       | `planning`    | Key milestones the project plan should define |
| `objectives`                | `planning`    | High-level objectives traced to goal and success measures |
| `tasks-to-plan-for`         | `planning`    | Specific tasks that could be overlooked |
| `research-and-prototyping`  | `design`      | Items needing investigation before implementation |
| `risk-mitigations`          | varies        | Structural mitigations → `design`, process mitigations → `planning` |
| `out-of-scope`              | none          | Excluded items (no `--phase` or `--complete` fields) |

### Phase assignment guidance

These are defaults — override based on the specific item's nature:

- **`design-must-haves`** → typically `design`
- **`architecture-constraints`** → typically `design`
- **`planning-milestones`** → always `planning`
- **`objectives`** → typically `planning`
- **`tasks-to-plan-for`** → typically `planning`
- **`research-and-prototyping`** → typically `design`
- **`risk-mitigations`** → use judgement per item
- **`out-of-scope`** → no phase (omit `--phase`)

### Item schema

Every item written to `checklist.json` has these fields:

| Field         | Type     | Description |
|---------------|----------|-------------|
| `id`          | `string` | Unique, stable, kebab-case identifier (e.g. `design-data-models`) |
| `title`       | `string` | Short, actionable title |
| `description` | `string` | What this item requires and why — include source onboarding input(s) |
| `complete`    | `bool`   | Always `false` when first added — downstream phases mark items complete |
| `phase`       | `string` | `"design"` or `"planning"` — omitted for out-of-scope items |

### Shell quoting

Descriptions and titles may contain special characters. Always wrap `--title` and `--description` values in double quotes. If the value itself contains double quotes, escape them with `\"`.

### Other useful commands

After initial population, these commands can be used to manage the checklist:

```bash
rex-cli checklist list                                    # List all items
rex-cli checklist list --phase design                     # Filter by phase
rex-cli checklist list --category risk-mitigations         # Filter by category
rex-cli checklist list --incomplete                        # Show only incomplete items
rex-cli checklist get <ID>                                 # Show item details
rex-cli checklist update <ID> --title "New title"          # Update fields
rex-cli checklist update <ID> --description "New desc"     # Update description
rex-cli checklist update <ID> --phase planning             # Change phase
rex-cli checklist complete <ID>                            # Mark complete
rex-cli checklist uncomplete <ID>                          # Mark incomplete
rex-cli checklist remove <ID>                              # Remove an item
```

Adjust categories based on what's relevant. If the project has no research items, don't add items to that category. The categories are a guide — not every project needs all of them.
