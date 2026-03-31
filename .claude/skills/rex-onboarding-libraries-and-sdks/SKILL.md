---
name: rex-onboarding-libraries-and-sdks
description: Gather the user's preferred libraries, SDKs, and dependencies for their project during rex onboarding. Use this skill when the rex onboarding process reaches the "libraries-and-sdks" step, when the user wants to specify which packages or SDKs to use, or when the user says things like "I want to use X library", "here are my dependencies", "which crates should we use", or "let me tell you about the SDKs."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Libraries and SDKs

You help users identify and document the specific libraries, SDKs, frameworks, and dependencies they want used in their project. This isn't about you recommending packages — it's about capturing what the user already has in mind or has strong opinions about.

You'll be told where to write the output (a file path like `onboarding/libraries-and-sdks.md`). If input files are provided, read them first for context. Then work with the user and write the final document to the output path.

---

## What you need to find out

1. **Are there specific libraries or SDKs the user wants used?** — Maybe they've already decided on `tokio` for async, `clap` for CLI args, `reqwest` for HTTP. Capture these with as much detail as the user provides — version preferences, specific features they need, why they chose it over alternatives.

2. **Are there any they want avoided?** — Sometimes the user has been burned by a package or has organizational constraints. "Don't use X, it caused us problems" is valuable information. Capture the reason too.

3. **Are there APIs or external services involved?** — If the project integrates with third-party services, capture which SDKs or client libraries the user expects to use for those.

4. **Any version constraints?** — Pinned versions, minimum versions, or compatibility requirements with other systems.

5. **Anything they're unsure about?** — If the user knows they need "something for X" but hasn't picked one yet, note that too. It's useful context for later onboarding steps.

---

## How to run the conversation

Ask the user what libraries, SDKs, or dependencies they have in mind for the project. Let them list what they know. For each one, capture:

- What it is and what it's for
- Why they chose it (if they volunteer a reason)
- Any version or configuration preferences
- Any strong opinions (must-use or must-avoid)

If the user says "none" or "I don't have preferences," that's a valid answer — document it and move on. Don't push them to pick libraries if they don't have opinions yet.

If the user mentions a library you know to be deprecated, abandoned, or problematic, it's fair to mention that briefly — but the final call is theirs. You're documenting their preferences, not overriding them.

---

## Writing the output

Capture everything the user said faithfully. The point of this document is that any agent or person working on the project later knows exactly what the user specified — their exact preferences, reasoning, and constraints.

```markdown
# Libraries and SDKs

**Date:** YYYY-MM-DD

## Specified Libraries
For each library/SDK the user wants:
- **Name:** package name
- **Purpose:** what it's for in this project
- **Why:** reason for choosing it (if given)
- **Version/Notes:** any constraints or configuration details

## Avoid
Libraries or SDKs the user explicitly doesn't want, and why.

## Undecided
Areas where the user knows they need something but hasn't chosen yet.

## Context
Any additional notes from the conversation — reasoning, tradeoffs the user mentioned, compatibility concerns, etc.
```

Write to the output path you were given (relative to the project's rex directory).
