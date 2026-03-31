---
name: rex-onboarding-checklist
description: Synthesize all onboarding inputs into a comprehensive checklist of everything the project must address during design, architecture, and planning phases — milestones, objectives, and tasks. Use this skill when the rex onboarding process reaches the "checklist" step, when the user wants to see everything the project needs to cover before building begins. Also trigger when the user says things like "what do we need to plan for", "give me the checklist", "what must the design include", "summarize what we need to do", or "what should planning cover."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Checklist

You've gathered everything — the goal, scope, risks, resources, expertise, success criteria, and ideas. Now distill all of that into a single checklist that tells the design, architecture, and planning phases exactly what they must address. You're not doing the work — you're defining what the work must include.

You'll be told where to write the output (a file path like `onboarding/checklist.json`). **Read all available input files first** — goal, scope, existing code, libraries, research, resources, user expertise, UAT, known risks, success measures, environment variables, idea generation, skill building. Every onboarding document that exists is relevant. Then work with the user and write the final document to the output path.

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

- **`"design"`** — Discovery-related actions. Things that must happen when modules, mermaid diagrams, high-level structs, data models, system boundaries, interface contracts, and architectural decisions are being planned. This is the phase where the system's shape is discovered and defined. Examples: defining data models, drawing module boundaries, choosing architectural patterns, designing API contracts, creating system diagrams, prototyping unknowns, resolving research questions.

- **`"planning"`** — Execution-related actions. Things that must happen when milestones, objectives, and tasks are being decided. This is the phase where the work is organized into deliverables and schedules. Examples: defining milestones, setting objectives, breaking work into tasks, establishing success criteria checkpoints, sequencing dependencies, assigning risk mitigations to specific milestones.

When in doubt: if the item is about **what the system looks like** → `"design"`. If the item is about **how the work gets done** → `"planning"`.

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

Present your draft checklist to the user, organized by phase. For each item, briefly note which onboarding input(s) it came from so the user can trace the reasoning.

Then ask:
- "Is anything missing? Anything you know needs to happen that didn't come up during onboarding?"
- "Is anything here that you'd consider out of scope or unnecessary?"
- "Are the milestones at the right granularity — too coarse, too fine?"

This conversation should be focused. You've already done the synthesis — the user is reviewing and adjusting, not starting from scratch. If they add items, capture the reasoning. If they remove items, note why so downstream phases don't re-add them.

Keep the conversation proportional to the project. A small project might need a quick confirmation pass. A complex one might need real discussion about priorities and phasing.

---

## Writing the output

The output is a **JSON file** (`checklist.json`). Every item must have these fields:

| Field         | Type     | Description |
|---------------|----------|-------------|
| `id`          | `string` | A unique, stable, kebab-case identifier for the item (e.g. `"design-data-models"`, `"plan-milestone-mvp"`) |
| `title`       | `string` | Short, actionable title (e.g. `"Define core data models"`) |
| `description` | `string` | What this item requires and why — include which onboarding input(s) it was derived from |
| `complete`    | `bool`   | Always `false` when first written — downstream phases mark items complete |
| `phase`       | `string` | Either `"design"` or `"planning"` — see phase classification above |

The top-level structure groups items by category:

```json
{
  "project_checklist": {
    "date": "YYYY-MM-DD",
    "design_must_haves": [
      {
        "id": "design-example-item",
        "title": "Example design item",
        "description": "What this requires and why. Source: goal, scope",
        "complete": false,
        "phase": "design"
      }
    ],
    "architecture_constraints": [
      {
        "id": "arch-example-constraint",
        "title": "Example constraint",
        "description": "Non-negotiable constraint the architecture must respect. Source: libraries-and-sdks",
        "complete": false,
        "phase": "design"
      }
    ],
    "planning_milestones": [
      {
        "id": "plan-milestone-mvp",
        "title": "MVP milestone",
        "description": "What this milestone delivers and what success measures or UAT criteria it satisfies. Source: uat, success-measures",
        "complete": false,
        "phase": "planning"
      }
    ],
    "objectives": [
      {
        "id": "obj-example-objective",
        "title": "Example objective",
        "description": "High-level objective traced to goal and success measures. Measured by: how to verify. Source: goal, success-measures",
        "complete": false,
        "phase": "planning"
      }
    ],
    "tasks_to_plan_for": [
      {
        "id": "task-example-task",
        "title": "Example task",
        "description": "Specific task that could be overlooked if not called out. Why: brief reasoning. Source: known-risks",
        "complete": false,
        "phase": "planning"
      }
    ],
    "research_and_prototyping": [
      {
        "id": "research-example-item",
        "title": "Example research item",
        "description": "What needs to be resolved before full implementation. Unknown: what needs investigation. Source: research",
        "complete": false,
        "phase": "design"
      }
    ],
    "risk_mitigations": [
      {
        "id": "risk-example-risk",
        "title": "Example risk mitigation",
        "description": "Risk from onboarding that the design and plan must address. Mitigation: what the plan should include. Source: known-risks",
        "complete": false,
        "phase": "design"
      }
    ],
    "out_of_scope": [
      {
        "id": "oos-example-item",
        "title": "Example excluded item",
        "description": "Why excluded — listed so downstream phases don't accidentally include it. Source: scope"
      }
    ],
    "context": "How this checklist was derived — what the user emphasized, what was added or removed during review, any priorities or sequencing preferences discussed."
  }
}
```

### Phase assignment guidance by category

These are defaults — override based on the specific item's nature:

- **`design_must_haves`** → typically `"design"` (architectural decisions, data models, interface contracts)
- **`architecture_constraints`** → typically `"design"` (technology choices, compatibility, performance targets)
- **`planning_milestones`** → always `"planning"` (defining what "done" looks like at each stage)
- **`objectives`** → typically `"planning"` (high-level goals the project plan must achieve)
- **`tasks_to_plan_for`** → typically `"planning"` (specific work items to schedule)
- **`research_and_prototyping`** → typically `"design"` (unknowns to resolve before building)
- **`risk_mitigations`** → use judgement: structural/architectural mitigations → `"design"`, process/scheduling mitigations → `"planning"`
- **`out_of_scope`** → no `complete` or `phase` field needed (these are exclusions, not action items)

Adjust categories based on what's relevant. If the project has no research items, drop that array. If there are many milestones, expand that section. The schema is a guide — categories can be added or removed, but the item schema (`id`, `title`, `description`, `complete`, `phase`) is fixed.

Write valid JSON to the output path you were given (relative to the project's rex directory).
