---
name: rex-onboarding-idea-generation
description: Generate achievable ideas that could significantly improve the project, based on everything learned during onboarding. Use this skill when the rex onboarding process reaches the "idea-generation" step, when the user wants fresh ideas for their project. Also trigger when the user says things like "what else could we do", "any ideas", "how could this be better", or "what am I missing."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Idea Generation

You've seen the goal, the scope, the user's expertise, the risks, the resources. Now use all of that to suggest ideas that could make this project significantly better. Not pie-in-the-sky fantasies — achievable improvements that fit within the project's reality.

You'll be told where to write the output (a file path like `onboarding/idea-generation.md`). **Read all available input files first** — goal, scope, existing code, libraries, research, resources, user expertise, UAT, known risks, success measures, environment variables. The more context you absorb, the better your ideas will be. Then work with the user and write the final document to the output path.

---

## What makes a good idea here

A good idea in this context has three qualities:

1. **Achievable** — It can actually be built within this project's scope and constraints. An idea that would require 10x the effort or a completely different architecture isn't helpful. If you're unsure, be honest about the cost.

2. **High impact** — It meaningfully improves the project for the user or their end users. Not a nice-to-have that nobody would notice, but something that would make someone say "oh, that's really good."

3. **Non-obvious** — The user probably hasn't thought of it, or has thought of it vaguely but not concretely. Restating what's already in the scope isn't idea generation — it's a recap. Show that you've synthesized the inputs and seen something new.

---

## How to approach it

Before talking to the user, think through the inputs. Look for:

- **Gaps between goal and scope** — Is there something the user clearly wants but hasn't scoped? Would a small addition close a big gap?
- **Underused resources** — Did the user mention a tool, library, or API that could do more than they're currently planning?
- **User pain points** — Based on their expertise and motivation, what would delight them? What would save them the most time or frustration?
- **Patterns from the domain** — Based on what you know about similar projects, what do the good ones do that this one doesn't yet?
- **Low-effort, high-reward additions** — Things that are easy to build but disproportionately valuable. A good CLI help system. Structured error messages. A progress indicator. These aren't glamorous but they make the difference between "works" and "works well."

---

## How to run the conversation

Present 3-5 ideas. For each, briefly explain what it is, why it would matter for this project specifically, and roughly how much effort it would add. Then ask the user what resonates.

Don't oversell. If an idea is speculative, say so. If it would add significant scope, flag that. The user decides what's worth pursuing — your job is to show them options they hadn't considered.

If the user likes an idea, discuss it enough to capture what they want. If they don't, move on — don't argue for your ideas.

If the user has ideas of their own that come up during this conversation, capture those too. This is a collaborative brainstorm, not a presentation.

---

## Writing the output

Capture both the ideas you proposed and the user's reaction. This document should be useful during design and build — not just a list of "nice ideas" but actionable suggestions with enough context to implement.

```markdown
# Idea Generation

**Date:** YYYY-MM-DD

## Accepted Ideas
Ideas the user wants to pursue:

### 1. [Idea name]
- **What:** description of the idea
- **Why:** why it matters for this project — what the user said about it
- **Effort:** rough sense of additional work involved
- **Notes:** any specifics discussed about how to implement or integrate it

### 2. [Idea name]
...

## Parked Ideas
Ideas the user found interesting but doesn't want to pursue now — captured for later.

## Rejected Ideas
Ideas that were proposed but the user didn't want — and why, so agents don't re-suggest them.

## User's Own Ideas
Ideas the user brought up during the conversation.

## Context
How the discussion went — what inputs informed the ideas, what the user responded to most, any themes that emerged.
```

Write to the output path you were given (relative to the project's rex directory).
