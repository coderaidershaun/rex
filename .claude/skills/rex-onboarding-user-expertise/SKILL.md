---
name: rex-onboarding-user-expertise
description: Learn about the user's expertise, domain knowledge, and insights that should inform the project's design and build. Use this skill when the rex onboarding process reaches the "user-expertise" step, when you need to understand the user's background and how it should shape the project. Also trigger when the user says things like "let me tell you about my background", "here's what I know about this space", "I have experience with X", or "here's how I'd approach this."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: User Expertise

You're here to learn from the user. They know things — about their domain, their craft, the problem space — that agents working on this project later won't have unless someone captures it now. This is your chance to get consultancy from the person who understands this problem best.

You'll be told where to write the output (a file path like `onboarding/user-expertise.md`). If input files are provided, read them first for context. Then work with the user and write the final document to the output path.

---

## What you're after

### Who they are

Get a sense of the user's background — not a CV, but the parts that are relevant to this project. Are they a developer? A quant? A researcher? A designer? A domain expert who doesn't code? Someone wearing all the hats?

Their background shapes how agents should communicate with them and what kinds of decisions can be delegated versus what needs their sign-off.

### What they know that agents won't

This is the real value. The user likely has domain expertise, hard-won intuitions, or specific knowledge that would take an agent a long time to derive — if it could at all. Examples:

- A quant who knows which pricing models actually work in practice vs which ones are textbook-only
- A developer who's built this kind of system before and knows where the hard parts are
- A researcher who understands the state of the art and what's been tried
- A domain expert who knows the business rules that aren't written down anywhere
- Someone who's seen similar projects fail and knows why

### Their instincts about this project

Ask what they think the right approach is. Even rough instincts are valuable — "I think we should start with the data model" or "the hard part is going to be the real-time sync" or "don't over-engineer this, keep it simple." These steer agents away from generic approaches and toward what actually matters for this specific project.

---

## How to run the conversation

Start by asking about their background as it relates to this project. Then go deeper into what they know that would help:

- "What's your experience with this kind of problem?"
- "Are there things you've learned the hard way that agents should know upfront?"
- "If you were advising someone building this, what would you tell them to watch out for?"
- "What's your instinct on the right approach here?"
- "Is there anything about this domain that's non-obvious or counterintuitive?"

Let them talk. This is a conversation where the user is the expert and you're the one learning. Don't challenge their domain knowledge — capture it. If something is unclear, ask them to elaborate so you can record it accurately.

Some users will have a lot to say. Others will be brief. Both are fine — match the conversation to what they give you.

---

## Writing the output

This document is a knowledge transfer. Agents reading it later should feel like they got a briefing from someone who really understands the problem. Capture the user's expertise, instincts, and advice faithfully — in their own words and framing where possible.

```markdown
# User Expertise

**Date:** YYYY-MM-DD
**User:** [name if known]

## Background
Who the user is and what expertise they bring — as it relates to this project.

## Domain Knowledge
Things the user knows about the problem space, domain, or technology that agents should understand. Preserve their language — these are expert insights, not summaries.

## Project Instincts
The user's thoughts on approach, priorities, pitfalls, or how to tackle this project. What would they tell someone starting this work?

## Key Advice
Specific guidance the user wants agents to follow — things to do, things to avoid, or principles to apply. Capture the reasoning behind each piece of advice.

## Context
Any additional notes — how the user framed their expertise, what they emphasized most, connections to other parts of the project.
```

Write to the output path you were given (relative to the project's rex directory).
