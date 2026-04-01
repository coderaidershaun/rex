---
name: rex-onboarding-environment-variables
description: Identify environment variables, API keys, and secrets needed for the project during rex onboarding. Use this skill when the rex onboarding process reaches the "environment-variables" step, when the user needs to specify what env vars or credentials are required. Also trigger when the user says things like "here are the API keys", "you'll need these env vars", "there's a .env file", or "what credentials do we need."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Environment Variables

You help users document what environment variables, API keys, secrets, or credentials the project needs — and where to find them. Agents building the project later need to know what's required and where it lives so they don't hardcode secrets or skip configuration.

You'll be told where to write the output (a file path like `onboarding/environment-variables.md`). If input files are provided, read them first for context. Then work with the user and write the final document to the output path.

---

## What you need to find out

1. **Are there any environment variables needed?** — API keys, database URLs, service tokens, feature flags, config values. If the project calls external services, there are almost certainly credentials involved.

2. **Where are they?** — A `.env` file, a secrets manager, exported in a shell profile, passed in CI, etc. Get the location but **never ask for or record the actual secret values**.

3. **Will any be needed later?** — Even if the project doesn't need them yet, the user might know that a future integration will require keys. Capture that so agents don't get stuck later.

4. **Are there any `.env.example` or config templates?** — If so, note where they are.

If the user says there are no environment variables and none expected, that's a valid answer — document it and move on.

---

## How to run the conversation

### Conversation style

Ask open-ended questions. **Never present numbered options, menus, or dropdown-style choices.** Don't list common variable types for them to pick from — ask "Does this project need any API keys, credentials, or environment variables?" and let them tell you.

### Flow

Ask the user whether the project needs any environment variables or credentials. If they say yes, get the details for each one. If they're unsure, prompt gently:

- "Does this project talk to any external APIs or services?"
- "Will there be a database connection?"
- "Is there a `.env` file already, or will we need to create one?"

**Never ask for actual secret values.** Only capture variable names, what they're for, and where to find them.

---

## Writing the output

```markdown
# Environment Variables

**Date:** YYYY-MM-DD

## Required Variables
For each:
- **Name:** `VARIABLE_NAME`
- **Purpose:** what it's used for
- **Where to find it:** location of the value (e.g., "in `.env` at project root", "in 1Password vault X", "ask the user")
- **Notes:** any format requirements, default values, or caveats

## Expected Future Variables
Variables not needed yet but anticipated — what they'll be for and when they'll be needed.

## Configuration Files
Any `.env`, `.env.example`, config templates, or similar files — where they are and what they contain.

## Context
Any additional notes from the conversation — how the user manages secrets, preferences about config approach, etc.
```

Write to the output path you were given (relative to the project's rex directory).
