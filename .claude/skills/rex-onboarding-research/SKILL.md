---
name: rex-onboarding-research
description: Gather research topics the user wants investigated before design and build begins during rex onboarding. Use this skill when the rex onboarding process reaches the "research" step, when the user wants to identify things that need investigating upfront, or when the user says things like "there's some research needed", "we need to look into X first", "check out this repo", or "there's an algorithm I need you to understand."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Research

**CRITICAL: NEVER present numbered options, menus, multiple-choice lists, or dropdown-style selections. Ask one open-ended question at a time and let the user answer in their own words.**

You help users identify what needs to be researched before the project's design and build phases begin. Some projects can jump straight into coding. Others need groundwork — understanding an algorithm, reading through a reference implementation, studying an API's behavior, or digesting a paper.

You'll be told where to write the output (a file path like `onboarding/research.md`). If input files are provided, read them first for context. Then work with the user and write the final document to the output path.

**You will also receive the project object with metadata (category, complexity, title, directory, etc.) — use it.** Don't ask the user to re-state information that's already in the project metadata.

---

## What you're looking for

Research topics are things that would change how the project is designed or built if understood upfront. Examples:

- A specific algorithm or technique the project depends on (e.g., "look into order book matching algorithms before we design the engine")
- A GitHub repo or reference implementation to study (e.g., "check out how ripgrep handles parallel file walking")
- An API or service whose behavior needs understanding (e.g., "read the Stripe Connect docs — the payout flow is non-obvious")
- A paper, spec, or standard that defines the problem space
- A competing product or prior art to learn from
- A technical question that needs answering (e.g., "can SQLite handle our expected write volume?")

You're not doing the research — you're capturing what needs researching, where to find it, and why it matters to this project.

---

## How to run the conversation

### Conversation style

Ask open-ended questions and let the user describe what needs researching. **Never present numbered options, menus, or dropdown-style choices.** Don't pre-populate a list of research categories for them to pick from — ask "Is there anything that needs investigating before we start building?" and let them tell you what they're thinking.

### Flow

Ask the user if there's anything that needs investigating before design and build starts. For each topic they mention, get:

- **What to research** — the topic, algorithm, repo, API, paper, etc.
- **Where to look** — URLs, repo paths, doc pages, or "just look into it generally"
- **Why it matters** — how this research will impact the project's design or implementation
- **What to look for** — any specific questions the user wants answered or aspects they care about

If the user says there's nothing to research, that's fine — document it and move on.

If the user is vague ("we should probably look into caching"), help them get specific: "What about caching do you want understood? Are you trying to decide whether to use it, or how to implement it?"

---

## Writing the output

Capture each research topic faithfully with the user's reasoning. This document will be used by agents doing the actual research later, so be precise about what to look at and what questions to answer.

```markdown
# Research

**Date:** YYYY-MM-DD

## Topics

### 1. [Topic name]
- **What:** What needs to be researched
- **Where:** URLs, repos, docs, or general direction
- **Why:** How this impacts the project — in the user's own words
- **Key questions:** Specific things to find out

### 2. [Topic name]
...

## Context
Any additional notes — how the user prioritized these, connections between topics, or general direction they gave about the research phase.
```

Write to the output path you were given (relative to the project's rex directory).
