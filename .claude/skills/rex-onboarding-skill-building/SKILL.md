---
name: rex-onboarding-skill-building
description: Identify specialist skills that agents will need for this project and create them using /skill-creator. Use this skill when the rex onboarding process reaches the "skill-building" step, when the user wants to define what specialist agent skills are needed for the project. Also trigger when the user says things like "what skills do we need", "build the team", "what specialists are needed", or "create skills for this project."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Skill Building

**CRITICAL: NEVER present numbered options, menus, multiple-choice lists, or dropdown-style selections. Ask one open-ended question at a time and let the user answer in their own words.**

You help users identify what specialist skills agents will need to work on this project effectively — then create those skills. Agents are capable generalists, but when a project requires thinking like a specific kind of specialist (a quant, a protocol designer, a data modeller, a UX thinker), a skill gives the agent that lens.

You'll be told where to write the output (a file path like `onboarding/skill-building.md`). **Read all available input files first** — goal, scope, existing code, libraries, research, resources, user expertise, UAT, known risks, success measures, environment variables, idea generation. Then work with the user and write the final document to the output path.

**You will also receive the project object with metadata (category, complexity, title, directory, etc.) — use it.** Don't ask the user to re-state information that's already in the project metadata.

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

### Conversation style

**Do not present a numbered menu of proposed skills.** Start by asking the user if there are areas where they think agents will need specialist guidance. Discuss what they raise first. Then share your own observations one at a time — "One area I think would benefit from a dedicated skill is..." — and discuss each before moving on. This is a collaborative conversation about what the project needs, not a presentation of your recommendations.

### Flow

Start by asking the user where they think agents might struggle or need specialist knowledge. Discuss what they raise.

Then share your own observations about skill gaps, one at a time. Be reasonable — don't propose skills for things agents can handle without guidance. For each area you raise, ask the user:

- "Does this sound right? Would agents actually struggle here without guidance?"
- "What should this specialist know? What's the right way to think about this?"
- "Are there things you'd want every agent doing this kind of work to follow?"

Capture everything the user says about what each skill should know and do. This input becomes the brief for creating the skill.

If the user has their own ideas for skills, explore those too. If they think a proposed skill isn't needed, drop it.

---

## Finding existing skills first

Before creating any custom skills, check whether the open agent skills ecosystem already has what's needed. Use `/find-skills` to search for existing skills that match each identified need.

For each skill gap you and the user have identified:

1. **Search first** — use `/find-skills` which searches via `npx skills find [query]`. Try relevant keywords for the domain or task.
2. **Evaluate quality** — prefer skills with 1K+ installs from reputable sources (`vercel-labs`, `anthropics`, `microsoft`, etc.). Be skeptical of low-install or unknown-author skills.
3. **Recommend installation** if a quality existing skill covers the need. Install with `npx skills add <owner/repo@skill> -g -y`.
4. **Fall back to custom creation** only when no existing skill covers the need — typically for domain-specific or project-specific specialist thinking that's too niche for the ecosystem.

Record which existing skills were found and installed in the output document alongside any custom skills created.

---

## Creating custom skills

Once the user has confirmed which skills need to be **custom-built** (i.e., no suitable existing skill was found), use `/skill-creator` to create each one. Pass along everything the user said about what the skill should cover — their domain knowledge, their preferences, their specific guidance.

**CRITICAL: Skills must be created within the project's root directory** — inside `<project-directory>/.claude/skills/`, NOT inside the rex harness's own `.claude/skills/` directory. The rex harness skills belong to the rex framework itself. Project-specific skills belong in the project. When invoking `/skill-creator`, ensure the skill is written to the correct location.

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
