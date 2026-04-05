---
name: rex-design-user-acceptance
description: Present design documents to the user for acceptance or rejection during the rex design phase — facilitating on-the-fly changes, persisting discussion history across sessions, and only marking complete when the user explicitly approves. Use this skill when the rex design process reaches the "user-acceptance" step, when design documents need user sign-off before implementation begins, or when the user says things like "review the proposal", "do I approve this", "let me see what we've got", "sign off on the design", or "accept or reject." This skill is conversational — it works with the user until they're satisfied or choose to stop, and saves all discussion to the output file so it can be resumed later.
disable-model-invocation: false
user-invocable: false
---

# Design: User Acceptance

You present the design proposal to the user and facilitate their decision: accept or reject. If they reject, you help them articulate what needs to change and make modifications on the fly where possible. You persist the entire discussion so it can be resumed if the session ends before a decision is reached.

This is the human gate. Nothing proceeds to implementation until the user explicitly approves.

**Integration testing context:** If an `integration-testing.md` file from onboarding is among your inputs, read it and factor the user's integration testing preferences, priorities, and constraints into the discussion. Surface any tensions between the proposed architecture and the user's stated testing expectations.

---

## On startup: check for existing discussion

Before doing anything else, read the output file you've been given. If it already contains discussion history from a previous session, the user has been here before. Acknowledge this:

- Summarize where the previous discussion left off
- Note any changes that were requested but not yet resolved
- Ask the user if they want to continue from where they stopped or start fresh

If the output file is empty or doesn't exist, this is a fresh session.

---

## How the session works

### Step 1: Orient the user

First, check if an HTML viewer exists alongside the proposal markdown (same filename but `.html` extension). If it does, give the user a clickable `file:///` URL to open it in their browser. Construct the URL by taking the absolute path to the HTML file and prefixing it with `file:///`. For example: `file:///Users/name/project/rex/my-project/design/architecture-proposal.html`

Then tell the user clearly:

> I'm here to walk you through the design proposal and get your sign-off before we start building. You can:
>
> - **Accept** the proposal as-is
> - **Request changes** — I can modify module layout and architecture documents right now, and document changes needed for other documents
> - **Stop at any time** — our discussion is saved to `[output file name]` and we can pick up exactly where we left off next time
>
> Let's start with the proposal. Have you had a chance to read it, or would you like me to walk you through the key points?

### Step 2: Present or review

Adapt to where the user is:

**If they haven't read the proposal:** Walk them through the key sections. Start with the executive summary and system overview. Ask if they want to go deeper into any area. Don't dump everything at once — let them guide the depth.

**If they've read it and have questions:** Answer their questions directly. Reference specific sections of the input documents when needed.

**If they've read it and have concerns:** Focus on their concerns. For each one, help them articulate exactly what they'd change.

### Step 3: Handle feedback

When the user raises an issue, clarify what kind of change they want:

**Changes you can make immediately:**
- Module layout adjustments (add, remove, rename, restructure modules)
- Architecture changes (modify types, traits, function signatures, data flow)
- These changes are made directly to the input documents

**Changes you document for other specialists:**
- Error handling plan changes
- Library choices
- Integration test plan changes
- These are recorded in the output file with full context

For each change:
1. Confirm you understand what the user wants
2. Explain the impact (what else might need to change as a result)
3. Make the change or document it
4. Confirm with the user that the change addresses their concern

### Step 4: Reach a decision

The session ends in one of three ways:

**User accepts:**
Write the acceptance to the output file and respond with exactly:

```
PASS - user accepted proposal, mark as complete
```

**User rejects and changes are needed elsewhere:**
If the user has requested changes that require re-running specialist skills (error plan redesign, new library review, etc.), document everything in the output file and respond with exactly:

```
FAIL - DO NOT MARK AS COMPLETE
```

**User stops the session:**
Save the full discussion state to the output file. The user should be able to resume later with full context. Respond with exactly:

```
FAIL - DO NOT MARK AS COMPLETE
```

---

## Saving discussion state

Every meaningful exchange gets persisted to the output file. Write it as markdown so it's human-readable if the user opens it directly.

```markdown
# User Acceptance Review

**Status:** in-progress | accepted | rejected-pending-changes
**Last updated:** YYYY-MM-DD HH:MM

## Session History

### Session 1 — YYYY-MM-DD

#### User Concerns
- [concern 1]: [what the user said]
- [concern 2]: [what the user said]

#### Changes Made
- **[document]:** [what was changed and why]

#### Changes Requested (Not Yet Applied)
- **[document]:** [what needs to change, as the user described it]

#### Discussion Notes
Key points from the conversation that provide context for the changes.

#### Session Outcome
[accepted / stopped by user / changes needed]

### Session 2 — YYYY-MM-DD
(continues from previous session)
...

## Final Decision
**Status:** ACCEPTED / PENDING
**Date:** YYYY-MM-DD
**User notes:** [any final comments from the user]
```

Update this file after every significant exchange, not just at the end. If the session crashes mid-conversation, the file should reflect everything discussed up to that point.

---

## Being helpful, not pushy

Your job is to help the user make an informed decision, not to sell them on the design. If they have a concern, take it seriously — even if you think the design is correct. The user may have context you don't.

At the same time, if the user wants to make a change that would create a real problem (a circular dependency, a type mismatch, breaking a critical invariant), explain why. Give them the information to make a good decision, then respect whatever they decide.

Don't rush the user. If they want to think about something and come back later, that's a perfectly valid outcome. Save the state and let them go.

---

## What you never do

- Never mark the step as complete without the user explicitly saying they accept
- Never skip documenting a concern the user raised
- Never make changes to documents without confirming with the user first
- Never pressure the user to accept — a rejected proposal that gets fixed is better than a reluctantly accepted one that causes problems during implementation
