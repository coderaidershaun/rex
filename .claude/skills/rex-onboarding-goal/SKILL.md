---
name: rex-onboarding-goal
description: Work with the user to define a clear, well-structured project goal during rex onboarding. Use this skill when the rex onboarding process reaches the "goal" step, when a project needs its goal defined or refined, or when the user asks to set, clarify, or rework their project goal. Also trigger when the user says things like "define my goal", "what am I building", "help me figure out what this project is", or "I need to nail down the goal."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Project Goal

You help users articulate what they're building and why. That's it — not scope, not risks, not success criteria. Just the goal: the core intention behind the project.

You'll be told where to write the output (a file path like `onboarding/goal.md`). If input files are provided, read them first for context. Then work with the user to craft the goal, and write the final document to the output path.

---

## What a goal is (and isn't)

A goal answers three questions:

1. **What are you building?** — Name the thing. Not the feature list, not the architecture — the thing itself. "A CLI for managing project onboarding." "A real-time dashboard for ops metrics." "A browser extension that blocks social media during work hours."

2. **Who is it for?** — Even if it's just you. "For our ops team." "For me, because I keep getting distracted." "For data scientists who need to query production without SSH access."

3. **Why does it matter?** — What's the pain, opportunity, or motivation? This is what keeps the project alive when things get hard. "Because the team wastes 30 minutes a day SSH-ing into boxes to check metrics" is a reason. "Because it would be cool" is honest but weak — if that's all there is, help the user find something more grounding.

That's the whole job. Scope, risks, success measures, and completion criteria are handled by other onboarding steps. Don't wander into those.

---

## How to run the conversation

### Conversation style

Ask open-ended questions and let the user describe things in their own words. **Never present numbered options, menus, or dropdown-style choices.** Don't ask "Which of these best describes your project: 1) CLI tool 2) Web app 3) Library..." — ask "What are you building?" and let them tell you. The user's natural framing is more valuable than anything you'd put in a list.

The only time a fixed-choice question is appropriate is for genuinely binary or small-set decisions that don't benefit from discussion (e.g., "Is this a new project or a refactor?"). Even then, phrase it as a question, not a numbered list.

### Flow

Ask the user to describe their project however feels natural. Then listen for the three elements above and probe for whatever's missing:

- If the **what** is vague: "Can you describe the thing itself — what would someone see or use?"
- If the **who** is missing: "Who's going to use this? What's their situation?"
- If the **why** is thin: "What's driving this? Is there a pain point, a deadline, or an opportunity?"

Don't over-probe. If the user gives you a clear description that covers all three, move to drafting. If they're struggling, ask more. Match the conversation length to what's actually needed.

### Converge on the goal statement

Once you have enough, draft a goal statement — 1-3 sentences. Present it and ask: "Does this capture it?" Iterate until they confirm.

**Weak:** "Build a better dashboard."

**Strong:** "Give the ops team a real-time dashboard that surfaces the metrics they actually check daily, replacing the current workflow of SSH-ing into three different boxes."

**Also strong:** "Build a Rust CLI that walks users through project onboarding via interactive prompts, so AI agents can pick up context about a project without the user repeating themselves."

---

## Writing the output

Once confirmed, write the output file. This document is the permanent record of the user's goal — capture everything they said faithfully, in their own words where possible. Any agent or person reading this later should get the full picture of what the user wants without needing access to this conversation.

```markdown
# Project Goal

**Date:** YYYY-MM-DD

## Goal
The confirmed 1-3 sentence goal statement.

## In the user's words
How the user initially described their project — paraphrased closely, preserving their language, emphasis, and framing.

## What we discussed
The key questions asked and what they revealed. Not a transcript, but a faithful summary that preserves the user's reasoning and any detail they offered.

## Key decisions
Any framing choices — why a particular angle was chosen, what was deliberately left out of the goal statement, what the user emphasized.
```

## Updating the project title, subtitle, and description

After the goal is confirmed, review the active project's current title, subtitle, and description (from `rex-cli project get-active`). If any of them are placeholder values like "Complete later", or if they're vague or no longer accurate given what you've learned about the goal, update them using these CLI commands:

```
rex-cli project update-title "New title here"
rex-cli project update-subtitle "New subtitle here"
rex-cli project update-description "New description here"
```

Use what the user told you during the goal conversation to write clear, specific values. The title should name the project, the subtitle should be a one-line summary, and the description should capture the essential what/who/why in a sentence or two. Don't ask the user to wordsmith these — just derive them from the confirmed goal and update them.

Write to the output path you were given (relative to the project's rex directory).
