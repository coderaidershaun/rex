---
name: rex-onboarding-resources
description: Gather the external resources, tools, and codebases the user wants agents to use during their project. Use this skill when the rex onboarding process reaches the "resources" step, when the user wants to specify MCPs, CLI tools, reference codebases, or other tooling available to the project. Also trigger when the user says things like "here are the tools to use", "use this MCP for X", "there's a CLI for that", or "reference this other codebase."
disable-model-invocation: false
user-invocable: false
---

# Onboarding: Resources

You help users document the external resources, tools, and codebases that should be available to agents working on the project. This is the "toolbox" — what's at the agents' disposal beyond the project's own code.

You'll be told where to write the output (a file path like `onboarding/resources.md`). If input files are provided, read them first for context. Then work with the user and write the final document to the output path.

---

## What counts as a resource

- **Reference codebases** — Other repos or projects that agents should look at for patterns, conventions, or working examples. "Our auth service is at ~/Code/auth-service — follow its patterns for error handling."
- **MCP servers** — Model Context Protocol servers the user has available. Capture what each one does and when agents should use it. "Use the GitHub MCP for PR operations, not the CLI."
- **CLI tools** — Agentic or conventional CLI tools available on the system. "Use `gh` for GitHub interactions." "There's a `deploy` script in ~/bin that handles staging pushes."
- **APIs and services** — External services agents might need to interact with, and how to access them.
- **Documentation sources** — Where to find docs that aren't in the project itself. Internal wikis, Notion pages, Confluence spaces, etc.
- **Other projects or monorepo paths** — Sibling projects that share code, types, or conventions.

---

## How to run the conversation

### Conversation style

Ask open-ended questions and let the user describe their tooling. **Never present numbered options, menus, or dropdown-style choices.** Don't list categories of resources for them to pick from — ask "What tools or resources are available for this project?" and let them tell you what they've got.

### Flow

Ask the user what tools, codebases, or external resources are available for this project. For each one, get:

- **What it is** — name and type (MCP, CLI tool, codebase, API, etc.)
- **Where it is** — path, URL, or how to access it
- **When to use it** — the specific situations where agents should reach for this resource
- **When not to use it** — if there are cases where it might seem appropriate but shouldn't be used
- **Any setup or caveats** — authentication needed, rate limits, quirks

If the user doesn't have anything beyond the defaults, that's fine — document it and move on.

---

## Writing the output

Capture each resource with enough detail that an agent encountering this document for the first time knows exactly what's available and when to use each thing. Preserve the user's reasoning about *when* and *why* — that's what makes this useful rather than just a list.

```markdown
# Resources

**Date:** YYYY-MM-DD

## Reference Codebases
For each:
- **Name/Path:** where to find it
- **What it is:** brief description
- **When to use:** what to look at it for
- **Notes:** any caveats from the user

## MCP Servers
For each:
- **Name:** the MCP server
- **What it does:** capabilities
- **When to use:** specific situations
- **When not to use:** if applicable

## CLI Tools
For each:
- **Name/Command:** how to invoke it
- **What it does:** capabilities
- **When to use:** specific situations
- **Notes:** setup, auth, quirks

## Other Resources
Anything that doesn't fit above — APIs, doc sources, wikis, etc.

## Context
Any additional direction from the user — general preferences about tool usage, things they emphasized, or connections between resources.
```

Write to the output path you were given (relative to the project's rex directory).
