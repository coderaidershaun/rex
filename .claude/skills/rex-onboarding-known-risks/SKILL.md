---
name: rex-onboarding-known-risks
description: Identify risks and pitfalls for the project during rex onboarding, especially those related to LLM-agent-driven development. Use this skill when the rex onboarding process reaches the "known-risks" step, when the user wants to think through what could go wrong. Also trigger when the user says things like "what could go wrong", "what are the risks", "what should we watch out for", or "where might agents struggle."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Known Risks

You help users think through what could go wrong with their project — both the usual software risks and the specific pitfalls of having LLM agents build it. Your job is to surface a few key risks, get the user's take, and capture their input on how to handle them.

You'll be told where to write the output (a file path like `onboarding/known-risks.md`). If input files are provided, read them first for context. Read `rex/docs/projects.md` to understand the full rex process. Then work with the user and write the final document to the output path.

---

## What you're looking for

There are two categories of risk:

### Project risks
Things that could go wrong with the project itself — technical challenges, integration complexity, unclear requirements, dependency on external services, etc. These are project-specific and you'll need context from the user to identify them.

### Agent-driven development risks
This project will be built by LLM agents running long tasks. That introduces specific pitfalls:

- **Drift** — Agents can gradually move away from the intended design over a long task, especially if the instructions are ambiguous.
- **Hallucinated APIs** — Agents may use functions, flags, or library features that don't exist.
- **Over-engineering** — Agents tend to add unnecessary abstraction layers, generic frameworks, or configurability that wasn't asked for.
- **Silent failures** — Agents may write code that compiles and runs but doesn't do the right thing, especially with edge cases.
- **Lost context** — On long-running tasks, agents may lose track of earlier decisions or constraints.

Don't dump all of these on the user. Mention the 2-3 that are most relevant to *their* project and ask if they have concerns of their own.

---

## How to run the conversation

### Conversation style

Lead with open-ended questions, not numbered lists of risks. **Never present a menu of risks for the user to pick from.** Don't say "Here are the top 5 risks: 1)... 2)... 3)..." — instead, mention one or two concerns conversationally and ask what the user thinks. Let them surface their own worries first. You can weave in your observations as the conversation develops, but the user's concerns take priority over your catalogue.

### Flow

Start by briefly mentioning a concern or two you see — both project-specific and agent-related. Frame them conversationally, not as a numbered list. Then ask the user:

- "Do any of these concern you? Are there others you're thinking about?"
- "Have you been burned by anything like this before?"
- "Is there anything about this project that you think agents will find particularly tricky?"

For each risk the user cares about, discuss what mitigation looks like. What should agents do (or avoid doing) to reduce the risk? The user's input here is what matters — they know their domain and what's likely to go wrong.

Don't overdo the recommendations. A few well-chosen mitigations are better than a wall of defensive measures.

---

## Writing the output

Capture the risks and mitigations that the user actually engaged with. This document should help agents working on the project later understand what to be careful about — and what the user specifically asked them to watch for.

```markdown
# Known Risks

**Date:** YYYY-MM-DD

## Risks

### 1. [Risk name]
- **What:** description of the risk
- **Why it matters:** impact on this project specifically
- **Mitigation:** what agents should do about it — in the user's words where possible
- **Source:** project-specific or agent-driven development

### 2. [Risk name]
...

## User's concerns
Anything the user raised that doesn't fit neatly into a single risk — general worries, past experiences, or guidance about what to be careful with.

## Context
How this was discussed — what the user emphasized, what they dismissed, any reasoning behind their choices.
```

Write to the output path you were given (relative to the project's rex directory).
