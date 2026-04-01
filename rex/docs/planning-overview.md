# Planning: Milestones, Objectives, and Tasks

Rex uses a three-level hierarchy for planning work within a project:

```
Milestone
  └── Objective
        └── Task
```

All three entity types are stored in a single file per project at `rex/<project-id>/planning/planning.json`.

## Hierarchy

**Milestones** are major, meaningful checkpoints. They answer "what does done look like at a high level?" A milestone is reached — not worked on directly — as the cumulative result of completing all its child objectives. Milestones are the chapter titles of a project.

**Objectives** sit beneath a milestone and represent the strategic outcomes needed to reach it. They answer "what must be true for this milestone to be achieved?" Each objective is a coherent, scoped goal — not a single action, but not so broad it could be a milestone.

**Tasks** are the atomic units of work. They answer "what specific thing needs to be done next?" A task should be completable in a single work session with a clear definition of done.

## Common Fields

Every entity (milestone, objective, task) carries:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique kebab-case identifier |
| `title` | string | Short descriptive title |
| `description` | string | Detailed explanation |
| `status` | enum | `not-started`, `in-progress`, `completed`, `blocked` |
| `references` | string[] | File paths, URLs, or entity IDs that provide context |
| `outputs` | string[] | File paths or artifacts this entity produces |
| `checklist` | object[] | Definition-of-done items (`{id, item, done}`) |
| `upstream` | string[] | Same-type entity IDs that must complete first |
| `downstream` | string[] | Same-type entity IDs that depend on this |

Additionally:
- **Milestone** has `objectives: string[]` — child objective IDs (auto-managed)
- **Objective** has `milestone_id: string` and `tasks: string[]` — parent ref and child task IDs (auto-managed)
- **Task** has `objective_id: string` — parent objective reference

## Cross-Referencing

### Parent-Child (auto-managed)

When you create an objective under a milestone, the milestone's `objectives` list is automatically updated. When you create a task under an objective, the objective's `tasks` list is automatically updated. Removing a child cleans up the parent's list. Re-parenting (changing `--milestone` or `--objective` on update) moves the reference.

### Dependencies (bidirectional)

When you add `--add-upstream B` to entity A, entity B's `downstream` list automatically includes A. Same in reverse for `--add-downstream`. Removing dependencies cleans up both sides.

### References (manual)

The `references` field is for additional context pointers — design docs, specs, URLs, or other entity IDs. These are manually managed via `--add-reference` / `--remove-reference`.

## CLI Pattern

All three entity types share these core commands:

```
rex <entity> upsert --id <id> [--title <t>] [--description <d>] [--status <s>] [list-mod flags...]
rex <entity> get <id>
rex <entity> list [--status <s>]
rex <entity> remove <id>
```

- `upsert` creates if new (requires `--title`, `--description`, and parent ref), updates if existing
- `get` outputs the entity as JSON to stdout
- `list` outputs a filtered JSON array to stdout
- `remove` deletes the entity and cleans up references

Additionally, `rex task next` returns the highest-priority eligible task along with its parent objective and milestone, using dependency-aware ordering across all three levels of the hierarchy. See [tasks.md](tasks.md#get-the-next-task-to-work-on) for details.

Status messages go to stderr; machine-parseable JSON goes to stdout. This allows agents to pipe output directly.

## Planning Constraints

These constraints are enforced by the planning skills (`rex-planning-milestones`, `rex-planning-objectives`, `rex-planning-tasks`) to keep plans focused and prevent agent-generated bloat:

- **1-3 milestones per module or topic.** If more are needed, the scope is too broad.
- **1-3 objectives per work milestone.** If more are needed, the milestone must be split.
- **1-3 tasks per objective.** If more are needed, the objective must be split.
- **Review milestones follow heavy milestones.** Every milestone involving significant code gets a paired review milestone with two objectives: (1) review all code, (2) fix significant issues. Lightweight milestones (configuration, setup) can skip review.
- **All mutations via CLI.** Planning entities are created and updated exclusively through `rex <entity> upsert` commands, never by writing `planning.json` directly.
- **Dependencies are explicit.** Every entity must have its upstream and downstream dependencies explicitly set. No implicit dependencies.

When a constraint would be violated (e.g., 4 objectives needed for one milestone), the planning skills escalate by splitting the parent entity and rewiring all upstream/downstream dependencies to keep the graph intact.

## Storage

```
rex/<project-id>/planning/planning.json
```

```json
{
  "milestones": [...],
  "objectives": [...],
  "tasks": [...]
}
```

The file is created automatically on first `upsert`. The `planning/` directory is created during project setup.
