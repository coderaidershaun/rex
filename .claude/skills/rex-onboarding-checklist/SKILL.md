---
name: rex-onboarding-checklist
description: Synthesize all onboarding inputs into a comprehensive checklist of everything the project must address during design, architecture, and planning phases — milestones, objectives, and tasks. Use this skill when the rex onboarding process reaches the "checklist" step, when the user wants to see everything the project needs to cover before building begins. Also trigger when the user says things like "what do we need to plan for", "give me the checklist", "what must the design include", "summarize what we need to do", or "what should planning cover."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Checklist

You've gathered everything — the goal, scope, risks, resources, expertise, success criteria, and ideas. Now distill all of that into a single checklist that tells the design, architecture, and planning phases exactly what they must address. You're not doing the work — you're defining what the work must include.

You'll be told where to write the output (a file path like `onboarding/checklist.md`). **Read all available input files first** — goal, scope, existing code, libraries, research, resources, user expertise, UAT, known risks, success measures, environment variables, idea generation, skill building. Every onboarding document that exists is relevant. Then work with the user and write the final document to the output path.

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

The checklist should be immediately useful to whoever picks up design and planning next — whether that's an agent or the user. Every item should be concrete enough that someone can look at it and say "yes, we addressed this" or "no, we missed this."

```markdown
# Project Checklist

**Date:** YYYY-MM-DD

## Design Must-Haves
Things the design phase must address — architectural decisions, data models, interface contracts, system boundaries.

- [ ] [Item] — *Source: [which onboarding input(s)]*
- [ ] [Item] — *Source: [which onboarding input(s)]*

## Architecture Constraints
Non-negotiable constraints the architecture must respect — technology choices, compatibility requirements, performance targets, security requirements.

- [ ] [Item] — *Source: [which onboarding input(s)]*

## Planning Milestones
Key milestones the project plan should define — what "done" looks like at each stage.

### Milestone 1: [Name]
- **What:** description of what this milestone delivers
- **Validates:** what success measures or UAT criteria this satisfies
- **Checklist:**
  - [ ] [Specific deliverable or verification]
  - [ ] [Specific deliverable or verification]

### Milestone 2: [Name]
...

## Objectives
High-level objectives the project must achieve — traced back to the goal and success measures.

- [ ] [Objective] — *Measured by: [how to verify]*

## Tasks to Plan For
Specific tasks that must appear in the project plan — things that could be overlooked if not called out explicitly.

- [ ] [Task] — *Why: [brief reasoning]*
- [ ] [Task] — *Why: [brief reasoning]*

## Research and Prototyping
Items that need investigation or proof-of-concept work before full implementation.

- [ ] [Item] — *Unknown: [what needs to be resolved]*

## Risk Mitigations to Design For
Risks from onboarding that the design and plan must actively address.

- [ ] [Risk] — *Mitigation: [what the plan should include]*

## Out of Scope (Confirmed)
Items explicitly excluded — listed here so downstream phases don't accidentally include them.

- [Item] — *Reason: [why excluded]*

## Context
How this checklist was derived — what the user emphasized, what was added or removed during review, any priorities or sequencing preferences discussed.
```

Adjust sections based on what's relevant. If the project has no research items, drop that section. If there are many milestones, expand that section. The template is a guide, not a constraint.

Write to the output path you were given (relative to the project's rex directory).
