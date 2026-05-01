---
name: rex-utils-task-request
description: Schema + contract for tasks dispatched through the rex pipeline. Sub-skill embedded in other rex-* skills/agents so they understand the two JSON envelopes (project + step) they receive and what to do with them. Use when an agent receives a rex pipeline task request, sees `project-id` + `step` JSON in input, or another skill references this one.
disable-model-invocation: false
user-invocable: true
---

# Rex Pipeline Task Request — Schema + Contract

Sub-skill. Embedded in other rex-* skills/agents. Defines what you get + what you must do.

You receive task instructions from human or agent. Along with below.

## You receive two JSON envelopes

### 1. Project envelope (context)

```json
{
  "project-id": "some-project-slug",
  "category": "feature",
  "title": "Some title",
  "subtitle": "Some subtitle",
  "description": "Some description",
  "complexity": "medium",
  "chunks-required": 0,
  "chunks-completed": 0,
  "tasks-required": 0,
  "tasks-completed": 0,
  "completed": false
}
```

### 2. Step envelope (task)

```json
{
  "step": "some-step",
  "required": true,
  "skill": "some-optional-skill",
  "agent": "this-is-you",
  "instructions": ["Some optional instructions."],
  "inputs": "rex/active/<project-id>/*",
  "outputs": "rex/active/<project-id>/some-output.md",
  "completed": false
}
```

Most fields optional. Optional fields are **omitted** when absent (not present as `null`). Don't assume presence.

## Field semantics

### Project envelope

| Field | Type | Meaning | If missing |
|-------|------|---------|------------|
| `project-id` | string | Slug for the project. Use to scope file paths, memory. | Treat task as ad-hoc. |
| `category` | string | e.g. `feature`, `refactor`, `spike`, `research`. | Skip. |
| `title` | string | Human-readable project name. | Skip in user-facing output. |
| `subtitle` | string | One-line elaboration of title. | Skip. |
| `description` | string | Multi-sentence project context. | Use only what step gives. |
| `complexity` | string | `low` / `medium` / `high`. Hint for depth of work. | Default `medium`. |
| `chunks-required` / `chunks-completed` | int | Progress counters at the chunk level. | Treat as 0. |
| `tasks-required` / `tasks-completed` | int | Progress counters at the task level. | Treat as 0. |
| `completed` | bool | `true` once every required step is done. | Treat as `false`. |

### Step envelope

| Field | Type | Meaning | If missing |
|-------|------|---------|------------|
| `step` | string | Step name in pipeline. Identifier for logging/output. | Use generic label. |
| `required` | bool | `true` if the step is mandatory for every project. | Treat as `false`. |
| `skill` | string | Skill to invoke. Load it before acting. | Use your default behavior. |
| `agent` | string | Always you. Confirms identity. | Ignore. |
| `instructions` | array of string | Free-form lines from orchestrator. Highest priority. | Fall back to `skill` + inputs. |
| `inputs` | string (path glob) | Glob to READ before acting (e.g. `rex/active/<project-id>/*`). | Nothing to read. |
| `outputs` | string (path glob) | Glob describing what to PRODUCE. | Nothing to write. Return result inline. |
| `completed` | bool | Always `false` on receipt. You flip to `true` when done. | Treat as `false`. |

## Contract — what you MUST do

1. **Parse both envelopes.** Don't act before you know the shape.
2. **Load `skill` if given.** Apply its rules. Don't improvise.
3. **Read every path the `inputs` glob matches.** All of them. Before producing anything.
4. **Honor `instructions` first.** Then skill. Then inputs. Hierarchy: instructions > skill > inputs > defaults.
5. **Produce the file(s) described by `outputs`.** Exact path. Don't rename. Don't skip.
6. **Mark `completed: true`** in your return envelope when done.
7. **Surface failures explicitly.** If you can't produce an output, say which one + why. Don't silently skip.

## Output discipline

- `outputs` = a path glob (e.g. `rex/active/<id>/prd.md` or `rex/active/<id>/*`). Write the file(s) described. Caller reads them after.
- No `outputs` → return result inline in your response.
- Parent dir missing → create it. Don't error.
- File exists → overwrite if step is meant to produce it. If unsure, ask.

## Complexity hint behavior

| `complexity` | Default behavior |
|--------------|------------------|
| `low` | Single-pass, minimal exploration, ship lean. |
| `medium` | Some exploration, validate assumption, then act. |
| `high` | Multi-step plan, sub-agent if available, advisor check before commit. |

`instructions` overrides this if explicit.

## Failure mode

- Missing input file → report which path, do not invent content.
- Skill not loadable → report skill name, ask orchestrator.
- Conflict between `instructions` and `skill` → surface conflict, ask. Don't silently pick.
- Output cannot be produced → return envelope w/ `completed: false` + reason.

## Example

Receive:

```json
// project
{ "project-id": "rex", "complexity": "medium" }

// step
{
  "step": "draft-error-types",
  "required": true,
  "skill": "rex-code-error-writing",
  "inputs": "src/orderbook.rs",
  "outputs": "src/orderbook/errors.rs",
  "completed": false
}
```

Do:

1. Load `rex-code-error-writing` skill.
2. Read `src/orderbook.rs`.
3. Per skill rules (thiserror enum, variant carries context, no `Box<dyn Error>`), draft errors.
4. Write `src/orderbook/errors.rs`.
5. Return envelope w/ `completed: true`.

## Fetching context from disk

If the project envelope or step envelope was not handed to you, run:

| Command | Returns |
|---|---|
| `rex project` | Full active project as JSON (project envelope + every step). |
| `rex project meta` | Project envelope only — see fields above. No `steps` key. |
| `rex project step` | Step envelope for the first incomplete step, or `{"status": "all-steps-complete"}`. |

Read first. Don't shell out if the orchestrator already gave you the envelopes — instructions > skill > inputs > defaults still applies.
