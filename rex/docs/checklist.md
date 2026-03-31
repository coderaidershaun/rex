# Checklist

Rex manages project checklists through `rex/<project-id>/onboarding/checklist.json`. The checklist tracks everything the project must address during design and planning phases.

## Data

### Storage

`rex/<project-id>/onboarding/checklist.json` — the checklist file for the active project.

```json
{
  "project_checklist": {
    "date": "2026-03-31",
    "design_must_haves": [ ... ],
    "architecture_constraints": [ ... ],
    "planning_milestones": [ ... ],
    "objectives": [ ... ],
    "tasks_to_plan_for": [ ... ],
    "research_and_prototyping": [ ... ],
    "risk_mitigations": [ ... ],
    "out_of_scope": [ ... ],
    "context": "..."
  }
}
```

### Item Fields

| Field         | Type            | Description                                              |
|---------------|-----------------|----------------------------------------------------------|
| `id`          | `string`        | Unique kebab-case identifier (e.g. `design-data-models`) |
| `title`       | `string`        | Short, actionable title                                  |
| `description` | `string`        | What this item requires and why                          |
| `complete`    | `bool` or absent| `false` when created, `true` when done. Absent for out-of-scope items |
| `phase`       | `string` or absent | `"design"` or `"planning"`. Absent for out-of-scope items |

### Categories

| Category                     | Typical Phase | Description |
|------------------------------|---------------|-------------|
| `design-must-haves`         | `design`      | Architectural decisions, data models, interface contracts |
| `architecture-constraints`  | `design`      | Non-negotiable technology, compatibility, performance constraints |
| `planning-milestones`       | `planning`    | Key milestones the project plan should define |
| `objectives`                | `planning`    | High-level objectives traced to goal and success measures |
| `tasks-to-plan-for`         | `planning`    | Specific tasks that could be overlooked if not called out |
| `research-and-prototyping`  | `design`      | Items needing investigation or proof-of-concept before implementation |
| `risk-mitigations`          | varies        | Structural → `design`, process → `planning` |
| `out-of-scope`              | none          | Items explicitly excluded — no `complete` or `phase` fields |

### Phases

- **`design`** — Discovery-related. Modules, mermaid diagrams, high-level structs, data models, system boundaries, interface contracts, architectural decisions.
- **`planning`** — Execution-related. Milestones, objectives, tasks, scheduling, success criteria checkpoints.

---

## Commands

All checklist commands operate on the **active** project's checklist. Use `rex project get-active` to check which project is active.

### `rex checklist init`

Initialize an empty `checklist.json` for the active project.

```
rex checklist init
rex checklist init --date 2026-03-31
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `--date` | no       | Date for the checklist in `YYYY-MM-DD` format. Defaults to today. |

**Behavior:**

- Creates an empty `checklist.json` in `rex/<project-id>/onboarding/`.
- Fails if `checklist.json` already exists.

**Error cases:**

- `"No active project."` — if no project is currently active.
- `"checklist.json already exists for project X."` — if the file already exists.

---

### `rex checklist add`

Add a new item to the checklist.

```
rex checklist add \
  --category design-must-haves \
  --id "design-data-models" \
  --title "Define core data models" \
  --description "Establish the primary data structures. Source: goal, scope" \
  --phase design
```

**Arguments:**

| Argument        | Required | Description |
|-----------------|----------|-------------|
| `--category`    | yes      | One of the category values (see Categories table) |
| `--id`          | yes      | Unique kebab-case identifier |
| `--title`       | yes      | Short, actionable title |
| `--description` | yes      | What this item requires and why |
| `--phase`       | conditional | `design` or `planning`. Required for all categories except `out-of-scope` |

**Behavior:**

- Adds the item to the specified category in `checklist.json`.
- Sets `complete: false` for non out-of-scope items.
- Out-of-scope items have no `complete` or `phase` fields.

**Error cases:**

- `"No active project."` — if no project is currently active.
- `"No checklist.json found..."` — if `checklist.json` does not exist. Run `rex checklist init` first.
- `"Item with ID X already exists..."` — if the ID is already used in any category.
- `"--phase is required for non out-of-scope items."` — if phase is missing for a non out-of-scope category.
- `"Out-of-scope items should not have a phase."` — if phase is provided for an out-of-scope item.

---

### `rex checklist list`

List checklist items with optional filters.

```
rex checklist list
rex checklist list --category design-must-haves
rex checklist list --phase design
rex checklist list --incomplete
rex checklist list --phase planning --complete
```

**Arguments:**

| Argument       | Required | Description |
|----------------|----------|-------------|
| `--category`   | no       | Filter by category |
| `--phase`      | no       | Filter by phase (`design` or `planning`) |
| `--complete`   | no       | Show only complete items |
| `--incomplete` | no       | Show only incomplete items |

**Behavior:**

- Displays items grouped by category with completion status, ID, title, and phase.
- Filters are combined (AND logic).
- Shows total count of matching items.

**Output format:**

```
  Checklist — "my-project"
  ────────────────────────────────────────

  Design Must-Haves (2)
    ○  design-data-models         Define core data models           design
    ✓  design-api-contracts       Design API contracts              design

  Planning Milestones (1)
    ○  plan-milestone-mvp         MVP milestone                     planning

  3 total items
```

Legend: `✓` = complete, `○` = incomplete, `–` = out-of-scope (no completion tracking).

---

### `rex checklist get <ID>`

Display details of a specific checklist item.

```
rex checklist get design-data-models
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `ID`     | yes      | The item's unique identifier |

**Behavior:**

- Prints all fields of the item in a labeled layout.
- Read-only — does not modify `checklist.json`.

**Output format:**

```
  ID:              design-data-models
  Title:           Define core data models
  Description:     Establish the primary data structures. Source: goal, scope
  Category:        Design Must-Haves
  Complete:        false
  Phase:           design
```

**Error cases:**

- `"Item X not found in checklist."` — if the ID doesn't match any item.

---

### `rex checklist update <ID>`

Update a checklist item's fields.

```
rex checklist update design-data-models --title "Define all data models"
rex checklist update design-data-models --description "Updated description"
rex checklist update design-data-models --phase planning
```

**Arguments:**

| Argument        | Required | Description |
|-----------------|----------|-------------|
| `ID`            | yes      | The item's unique identifier |
| `--title`       | no       | New title |
| `--description` | no       | New description |
| `--phase`       | no       | New phase (`design` or `planning`) |

At least one of `--title`, `--description`, or `--phase` must be provided.

**Behavior:**

- Updates only the specified fields. Unspecified fields remain unchanged.
- Prints what changed (old → new for title and phase, "updated" for description).

**Error cases:**

- `"Item X not found in checklist."` — if the ID doesn't match any item.
- `"At least one of --title, --description, or --phase must be provided."` — if no fields given.
- `"Cannot set phase on out-of-scope items."` — if trying to set phase on an out-of-scope item.

---

### `rex checklist complete <ID>`

Mark a checklist item as complete.

```
rex checklist complete design-data-models
```

**Behavior:**

- Sets `complete: true` on the item.

**Error cases:**

- `"Item X not found in checklist."` — if the ID doesn't match any item.
- `"Cannot mark out-of-scope items as complete."` — if the item is in the out-of-scope category.

---

### `rex checklist uncomplete <ID>`

Mark a checklist item as incomplete.

```
rex checklist uncomplete design-data-models
```

**Behavior:**

- Sets `complete: false` on the item.

**Error cases:**

- `"Item X not found in checklist."` — if the ID doesn't match any item.
- `"Cannot toggle completion on out-of-scope items."` — if the item is in the out-of-scope category.

---

### `rex checklist remove <ID>`

Remove a checklist item.

```
rex checklist remove design-data-models
```

**Behavior:**

- Removes the item from its category in `checklist.json`.
- Prints which category the item was removed from.

**Error cases:**

- `"Item X not found in checklist."` — if the ID doesn't match any item.

---

### `rex checklist set-context <CONTEXT>`

Set the checklist's context text.

```
rex checklist set-context "Derived from onboarding. User emphasized performance over features."
```

**Arguments:**

| Argument  | Required | Description |
|-----------|----------|-------------|
| `CONTEXT` | yes      | Free-text describing how the checklist was derived |

**Behavior:**

- Replaces the `context` field in `checklist.json`.

**Error cases:**

- `"No active project."` — if no project is currently active.
- `"No checklist.json found..."` — if `checklist.json` does not exist.
