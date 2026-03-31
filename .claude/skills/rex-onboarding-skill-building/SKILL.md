---
name: rex-onboarding-skill-building
description: Identify specialist skills that agents will need for this project and create them using /skill-creator. Use this skill when the rex onboarding process reaches the "skill-building" step, when the user wants to define what specialist agent skills are needed for the project. Also trigger when the user says things like "what skills do we need", "build the team", "what specialists are needed", or "create skills for this project."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Skill Building

You help users identify what specialist skills agents will need to work on this project effectively — then create those skills. Agents are capable generalists, but when a project requires thinking like a specific kind of specialist (a quant, a protocol designer, a data modeller, a UX thinker), a skill gives the agent that lens.

You'll be told where to write the output (a file path like `onboarding/skill-building.md`). **Read all available input files first** — goal, scope, existing code, libraries, research, resources, user expertise, UAT, known risks, success measures, environment variables, idea generation. Then work with the user and write the final document to the output path.

---

## When is a skill needed?

Not every project needs custom skills. Agents can handle a lot with general-purpose reasoning. A skill is worth creating when:

- **Domain-specific thinking is required** — The project needs an agent to reason like a specialist. A trading system needs agents that think about order matching correctly. A compiler needs agents that understand parse trees. General coding ability isn't enough.
- **Repeated judgment calls** — If agents will face the same kind of decision many times (e.g., "how should this error be handled?" or "what's the right data structure for this?"), a skill encodes the right approach so they get it right consistently.
- **The user has strong opinions about approach** — If the user has specific ways they want things done that go beyond standard practice, a skill captures that so every agent follows it.

A skill is probably *not* needed when:

- The task is straightforward coding that any competent agent can handle.
- The guidance could be captured in a brief instruction rather than a full skill.
- An existing skill already covers it.

---

## How to approach it

Before talking to the user, review all the onboarding inputs and the list of currently available skills. Think about:

- What kinds of work will this project involve?
- Which parts require specialist thinking that goes beyond general coding?
- Are there existing skills that already cover some of these needs?
- What's the gap between what's available and what's needed?

---

## How to run the conversation

Present your assessment: the 2-4 specialist skills you think the project needs, why each one matters, and what existing skills already cover. Be reasonable — don't propose skills for things agents can handle without guidance.

For each proposed skill, ask the user:

- "Does this sound right? Would agents actually struggle here without guidance?"
- "What should this specialist know? What's the right way to think about this?"
- "Are there things you'd want every agent doing this kind of work to follow?"

Capture everything the user says about what each skill should know and do. This input becomes the brief for creating the skill.

If the user has their own ideas for skills, explore those too. If they think a proposed skill isn't needed, drop it.

---

## Creating the skills

Once the user has confirmed which skills to create and provided their input, use `/skill-creator` to create each one. Pass along everything the user said about what the skill should cover — their domain knowledge, their preferences, their specific guidance.

Create them one at a time. After each one, confirm with the user before moving to the next.

---

## Writing the output

Document what was discussed and what was created. This is the record of the "team" that was assembled for the project.

```markdown
# Skill Building

**Date:** YYYY-MM-DD

## Skills Created

### 1. [Skill name]
- **Purpose:** what this skill enables agents to do
- **Why needed:** why general-purpose agents aren't enough here
- **User's input:** what the user wanted this skill to cover — their specific guidance and domain knowledge
- **Location:** path to the created skill

### 2. [Skill name]
...

## Existing Skills to Use
Skills that already exist and are relevant to this project — and what they'll be used for.

## Skills Considered but Not Created
Skills that were discussed but the user decided weren't needed — and why, so this isn't revisited.

## Context
How the discussion went — what the user emphasized about how agents should work on this project.
```

Write to the output path you were given (relative to the project's rex directory).
