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

## Project size sanity check

After the user has described their scope, step back and assess whether the project is the right size for a single rex-managed project. This matters because not every project needs the full rex harness, and some projects are better split up. Think carefully before making this judgment — most users doing this are smart and capable, and the bar for "too big" should be high.

### Projects that don't need rex

Some things are just too simple. A hello world program, a small utility script, a single-file CLI tool — these don't need onboarding, planning phases, or a project harness. If the scope looks like something that could be built in an afternoon without any architectural decisions, say so plainly:

> "This looks straightforward enough that you could just build it directly — the rex onboarding process would add more overhead than value here. Want to just go ahead and write it?"

Don't be condescending about it. Some of the best software is simple.

### Projects that might be better as multiple projects

Large, multi-system projects often benefit from being broken into separate library/binary projects rather than built as one monolith. This is especially true when the project contains clearly separable concerns that could each be independently useful.

**Example:** A user wants to build a trading engine. That's a legitimate, achievable goal. But if the current repo doesn't already have an exchange integration library, they're really scoping two projects: (1) an exchange integration crate that handles API connectivity, order management, and market data, and (2) the engine itself that uses that crate for strategy execution, risk management, etc. Building both as one project risks muddling concerns that should be clean boundaries.

When you spot this pattern, suggest the split — but frame it as advice, not a gate:

> "This is definitely buildable, but I think you'd get better results splitting it into [X] and [Y]. Each one becomes a focused project with clean boundaries, and [Y] can depend on [X] as a crate. A mono-repo approach works well here — both projects live in the same workspace, they just have separate rex onboarding so each gets the right focus."

**Be ambitious, not reckless.** The goal is not to stop users from building big things. It's to help them build big things *well*. A trading engine is fine. A trading engine + exchange integration + backtesting framework + UI dashboard + data pipeline in one project — that's where you push back. But give it real thought before deciding something is too large. Users can always disagree and continue regardless.

### When the user wants to change scope significantly

If the sanity check leads the user to fundamentally change what they're building — not just trimming scope, but redefining the project — then the existing goal is no longer valid. In this case:

1. Set the goal status back to in-progress: `rex-cli project update-status goal in-progress`
2. Tell the user the goal needs to be redefined to match the new direction
3. Invoke the `rex-onboarding-goal` skill to walk through goal definition again
4. Once the new goal is confirmed, return to scope definition with the updated goal

This only applies when the project's fundamental direction changes. Trimming features or deferring work is normal scope refinement — it doesn't invalidate the goal.

---

## How to run the conversation

### Conversation style

Ask open-ended questions and let the user describe things in their own words. **Never present numbered options, menus, or dropdown-style choices.** Don't ask "Which of these would you include? 1) Auth 2) Dashboard 3) API..." — ask "What needs to exist for this goal to be met?" and let them walk you through it. The user's natural way of describing boundaries is more useful than picking from your list.

The only time a fixed-choice question is appropriate is for genuinely binary decisions that don't benefit from discussion (e.g., "Web only, or mobile too?"). Even then, phrase it as a question, not a numbered list.

### Flow

If you have the goal document, start from it. If not, ask the user what the project goal is so you have a frame.

Then work through what's needed to achieve that goal:

- **If the user is thinking too big**: "That's a lot. If you had to pick the smallest version that still achieves the goal, what would you keep?"
- **If the user is thinking too small**: "Would that actually achieve the goal you described? What's missing?"
- **If boundaries are fuzzy**: "Would X be part of this project, or is that a separate thing?"
- **If everything feels essential**: "If you had to ship in half the time, what would you cut first?" — whatever they cut last is the real core.

Help the user be honest about what's actually v1 versus what they're hoping to sneak in. Most projects fail from too much scope, not too little.

### Converge

Once you have a clear picture — and the sanity check above has been considered — present the scope back as three lists: in, out, deferred. Ask the user to confirm. Iterate until they're satisfied.

---

## Writing the output

Once confirmed, write the output file. This document is the permanent record of the user's scope decisions — capture their reasoning faithfully. Any agent or person reading this later should understand not just what was decided, but *why* the user drew the line where they did.

```markdown
# Project Scope

**Date:** YYYY-MM-DD

## Project size assessment
Brief note on whether this project is appropriately sized. If you recommended splitting or simplifying, capture what was discussed and what the user decided.

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
