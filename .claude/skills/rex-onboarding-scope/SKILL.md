---
name: rex-onboarding-scope
description: Work with the user to define the scope of their project during rex onboarding. Use this skill when the rex onboarding process reaches the "scope" step, when a project needs its boundaries defined, or when the user asks to scope their project. Also trigger when the user says things like "define the scope", "what's in and out", "how big is this", "what should I include", or "help me draw the line on this project."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Project Scope

You help users draw a line around their project — what's in, what's out, and where the edges are. The goal has already been defined elsewhere. Your job is to take that goal and help the user decide what work is actually needed to achieve it.

You'll be told where to write the output (a file path like `onboarding/scope.md`). If input files are provided, read them first for context — the goal document is especially important if available. Then work with the user to define scope, and write the final document to the output path.

---

## What scope is (and isn't)

Scope answers: **what work is included in this project, and what isn't?**

It's not the goal (that's already defined). It's not success measures or risks — those are separate steps. Scope is the boundary that turns an intention into a concrete chunk of work.

Good scope has three parts:

1. **What's in** — The features, components, or deliverables that must exist for the goal to be met. Be specific enough to be useful. "User authentication" is too vague. "Email/password login with JWT sessions" is scoped. "Email/password login with JWT sessions, OAuth, SSO, MFA, and account recovery" is a different scope entirely.

2. **What's out** — The things someone might reasonably expect to be included but aren't. This is where most scope conversations pay off. Explicitly excluding things prevents scope creep and mismatched expectations. "No mobile app — web only for now." "No real-time sync — polling is fine for v1."

3. **What's deferred** — Things the user wants eventually but not in this round. These aren't rejected — they're acknowledged and parked. This gives the user confidence that their ideas aren't being lost, while keeping the current work bounded.

---

## How to run the conversation

If you have the goal document, start from it. If not, ask the user what the project goal is so you have a frame.

Then work through what's needed to achieve that goal:

- **If the user is thinking too big**: "That's a lot. If you had to pick the smallest version that still achieves the goal, what would you keep?"
- **If the user is thinking too small**: "Would that actually achieve the goal you described? What's missing?"
- **If boundaries are fuzzy**: "Would X be part of this project, or is that a separate thing?"
- **If everything feels essential**: "If you had to ship in half the time, what would you cut first?" — whatever they cut last is the real core.

Help the user be honest about what's actually v1 versus what they're hoping to sneak in. Most projects fail from too much scope, not too little.

### Converge

Once you have a clear picture, present the scope back as three lists: in, out, deferred. Ask the user to confirm. Iterate until they're satisfied.

---

## Writing the output

Once confirmed, write the output file. This document is the permanent record of the user's scope decisions — capture their reasoning faithfully. Any agent or person reading this later should understand not just what was decided, but *why* the user drew the line where they did.

```markdown
# Project Scope

**Date:** YYYY-MM-DD

## What's in
Bulleted list of what's included — specific enough to act on. Include the user's reasoning where they gave it.

## What's out
Bulleted list of explicit exclusions and why the user excluded them.

## Deferred
Bulleted list of things the user wants later but not now — capture what they said about timing or priority.

## How we got here
A narrative covering the conversation — what the user said, what tradeoffs they considered, why the line was drawn where it was. Preserve the user's own framing and language.
```

Write to the output path you were given (relative to the project's rex directory).
