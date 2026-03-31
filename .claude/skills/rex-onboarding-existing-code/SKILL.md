---
name: rex-onboarding-existing-code
description: Understand what existing code the user is bringing to their project during rex onboarding. Use this skill when the rex onboarding process reaches the "existing-code" step, when the user needs to identify or describe existing code resources, or when a refactor project needs to establish what's being refactored and how. Also trigger when the user says things like "I have existing code", "here's what I'm working with", "this is a refactor", or "let me show you the codebase."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Existing Code

You help users identify and describe the existing code resources they're bringing to a project. This might be a codebase they're refactoring, libraries they're wrapping, reference implementations they're drawing from, or nothing at all (greenfield).

You'll be told where to write the output (a file path like `onboarding/existing-code.md`). If input files are provided, read them first for context. You'll also be told the project's **category** (binary, library, or refactor) — this changes what you need to find out.

---

## What you need to establish

### For all projects

1. **Is there existing code?** — Some projects start from scratch. If there's nothing, say so in the output and you're done. Don't manufacture a conversation about code that doesn't exist.

2. **Where is it?** — Get the path(s), repo URL(s), or whatever locates the code. If the user points to a directory that's different from the project's registered directory, confirm whether the project directory should be updated. You can do this via:
   ```bash
   rex project update-directory "/correct/path/to/code"
   ```

3. **What is it?** — Language, framework, rough size, structure. Not a full architecture review — just enough that someone picking this up knows what they're looking at.

4. **What's its current state?** — Working? Broken? Prototype? Production? This affects how the rest of onboarding plays out.

### For refactor projects specifically

Refactor projects have existing code by definition — that's the whole point. You need to go deeper:

5. **What exactly needs refactoring?** — The whole codebase, or specific parts? Get the user to identify the specific modules, files, or subsystems that are in scope for the refactor.

6. **What's wrong with it?** — Why is it being refactored? Performance? Maintainability? Outdated patterns? Migration to a new framework? The reason shapes the approach.

7. **What's the refactor strategy?** — Two main approaches:
   - **Lift and shift** — Move the code to a new structure/framework/language while preserving its behavior. The logic stays the same; the container changes.
   - **In-place refactor** — Restructure the code where it lives. Improve the internals without changing the project boundaries.

   Help the user decide which fits. If they're unsure, ask: "Do you want to end up with the same code in a better shape, or the same behavior in a new codebase?" That usually clarifies it.

8. **What must be preserved?** — APIs, data formats, behavior contracts, integrations — what can't break during the refactor?

---

## How to run the conversation

Start by asking what existing code they're bringing, if any. For refactor projects, you already know there is some — ask them to point you at it.

If the user provides a path, explore it briefly (list the top-level structure) so you can have an informed conversation rather than taking everything on faith.

Keep it practical. You're building an inventory and understanding the starting point, not doing a code review.

---

## Writing the output

Once you have a clear picture, write the output file.

```markdown
# Existing Code

**Date:** YYYY-MM-DD

## Overview
Brief description of what existing code is being brought to this project (or "Greenfield — no existing code").

## Code Resources
For each codebase or significant code resource:
- **Location:** path or URL
- **Language/Framework:** what it's built with
- **Current state:** working/broken/prototype/production
- **Description:** what it does, roughly

## Refactor Details (if applicable)
- **What's being refactored:** specific modules/files/subsystems
- **Why:** the motivation for the refactor
- **Strategy:** lift-and-shift or in-place, and why
- **Must preserve:** APIs, contracts, or behaviors that can't break

## Context
How this was discussed — the user's reasoning, any key decisions about what's in play, and anything they emphasized or were particular about. Preserve their language and framing.
```

Write to the output path you were given (relative to the project's rex directory).
