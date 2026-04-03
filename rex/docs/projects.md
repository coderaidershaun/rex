# Projects

Rex manages projects through `rex/projects.json`. Only one project can be **active** at a time. All others are stored as **inactive**.

## Data

### Storage

`rex/projects.json` — the project registry.

```json
{
  "active": { ... },
  "inactive": [ ... ]
}
```

When a project is created, a metadata directory is also created at `rex/<project-id>/`.

### Project Fields

| Field         | Required | Description                                      |
|---------------|----------|--------------------------------------------------|
| `id`          | yes      | Unique project identifier (used as directory name)|
| `category`    | yes      | One of `binary`, `library`, `refactor`           |
| `complexity`  | yes      | One of `small`, `medium`, `large`                |
| `title`       | yes      | Project title (defaults to "Complete later")     |
| `subtitle`    | yes      | Short one-line summary (defaults to "Complete later") |
| `description` | yes      | Detailed project description (defaults to "Complete later") |
| `directory`   | yes      | Absolute path to the project's working directory |
| `user_name`   | no       | Name of the project owner                        |
| `locked`      | yes      | `false` by default. When `true`, agents must not work on the project and the operator will skip it |

---

## Commands

All commands that edit a project (`update`, `update-status`) operate on the **active** project. Use `get-active` to check which project is active, and `activate` to switch.

### `rex project create`

Interactively creates a new project and sets it as **active**.

```
rex project create
```

**Prompts (in order):**

1. **Project ID** — must be unique across all projects (active and inactive). Lowercase letters and hyphens only.
2. **Complexity** — select from `small`, `medium`, `large`.
3. **Title** — project title (press Enter to default to "Complete later").
4. **Subtitle** — short summary (press Enter to default to "Complete later").
5. **Description** — detailed description (press Enter to default to "Complete later").
6. **Directory** — the project's working directory. If a folder matching the project ID exists in the current working directory, it is auto-detected and offered as the default. Otherwise the user types a path manually. Relative paths are resolved to absolute.
7. **User Name** — optional, press Enter to skip.
8. **Category & Onboarding/Design Items** — interactive tab-selection widget for category (`binary`, `library`, `refactor`) and which onboarding/design steps to include.
9. **Summary & Confirm** — review all fields. Options: Create, Go back (returns to step 8), Cancel.
10. **Initialize rex inside project?** — whether to run `rex init` inside the project directory, creating a self-contained project with its own harness. Defaults to Yes when no outer harness exists, No when one does.

**Behavior on confirm — init inside project (Yes):**

- Runs `rex init` inside the project directory, creating `.claude/`, `rex/docs/`, and `CLAUDE.md` there.
- Creates a **new** `rex/projects.json` inside the project directory with this project as active.
- Creates `rex/<project-id>/` with subdirectories: `onboarding/`, `user-support/`, `planning/`, `design/`, `execution/`, `uat/`.
- Creates `rex/<project-id>/project-status.json` with the selected onboarding and design items.
- The outer `rex/projects.json` (if any) is **not** modified.
- If the project directory does not exist, it is scaffolded via `cargo new`.

**Behavior on confirm — init inside project (No):**

- If there is already an active project, it is moved to `inactive`.
- The new project becomes `active`.
- If the project directory does not exist, it is scaffolded via `cargo new`.
- The directory `rex/<project-id>/` is created with subdirectories: `onboarding/`, `user-support/`, `planning/`, `design/`, `execution/`, `uat/`.
- `rex/<project-id>/project-status.json` is created with the selected onboarding and design items.
- `rex/projects.json` is updated.

**Behavior on cancel:**

- Nothing is written. No side effects.

**Duplicate protection:**

- If a project with the given ID already exists (active or inactive), creation is rejected.

---

### `rex project get-active`

Displays the current active project.

```
rex project get-active
```

**Output:**

- If an active project exists, prints all fields in a formatted, labeled layout.
- If no active project exists, prints an informational message.

This command is read-only and does not modify `rex/projects.json`.

---

### `rex project remove <ID>`

Removes a project from the registry and deletes its metadata directory.

```
rex project remove my-project
```

**Behavior:**

1. Finds the project by ID (searches both active and inactive).
2. Removes `rex/<project-id>/` metadata directory if it exists.
3. Prompts with a **WARNING**: "Do you also want the project source directory removed?" — a select widget with **No** (green, default) and **Yes** (yellow).
4. If the user selects Yes, a second **WARNING** confirms: "This will delete the entire project code in directory \<directory\>. Are you certain?" — defaults to No.
5. If confirmed, the project's source directory is deleted.
6. Updates `rex/projects.json`. If the removed project was active, `active` becomes `null`.

**Error cases:**

- If the project ID is not found, prints an error and exits.

---

### `rex project activate <ID>`

Moves an inactive project to active.

```
rex project activate my-project
```

**Behavior:**

- The project is removed from `inactive` and set as `active`.
- If there is already an active project, it is moved to `inactive`.
- Updates `rex/projects.json`.

**Error cases:**

- `"Project X is already the active project."` — if the ID matches the current active project.
- `"Project X not found."` — if the ID does not exist in inactive projects.

---

### `rex project update [FLAGS]`

Updates one or more fields of the active project in a single operation.

```
rex project update --title "My New Title" --directory /path/to/new/directory
rex project update --category library --complexity large
rex project update --subtitle "Brief summary" --description "Detailed description"
```

**Flags:**

| Flag              | Value                                  |
|-------------------|----------------------------------------|
| `--title`         | New project title                      |
| `--subtitle`      | New short summary                      |
| `--description`   | New detailed description               |
| `--directory`     | New absolute path to working directory |
| `--category`      | One of `binary`, `library`, `refactor` |
| `--complexity`    | One of `small`, `medium`, `large`      |

At least one flag is required. Any combination of flags can be used together.

**Behavior:**

- Loads `rex/projects.json` once, applies all requested changes, and saves once.
- For each updated field, prints the old and new value.

**Error cases:**

- `"No active project."` — if no project is currently active.
- `"At least one field must be specified."` — if no flags are provided.

---

### `rex project update-status <ITEM> <STATUS>`

Updates the status of a project item in the active project's `project-status.json`.

```
rex project update-status goal completed
rex project update-status scope in-progress
```

**Arguments:**

| Argument | Description                                          |
|----------|------------------------------------------------------|
| `ITEM`   | Item name (e.g., `goal`, `scope`, `uat`, `research`) |
| `STATUS` | One of `not-started`, `in-progress`, `not-required`, `completed` |

**Behavior:**

- Loads the active project's `rex/<project-id>/project-status.json`.
- Searches the `onboarding`, `user_support`, and `design` lists for the matching item.
- Updates the item's `status` field and saves.

**Error cases:**

- `"No active project."` — if no project is currently active.
- `"Item X not found in project status."` — if the item name doesn't match any entry.

---

### `rex project next-item`

Outputs the next actionable item from the active project's `project-status.json` as a JSON object.

```
rex project next-item
```

**Algorithm:**

1. Loads the active project's `rex/<project-id>/project-status.json`.
2. Flattens all tasks into a single ordered list. For the current grouped format (object with `user_support`, `onboarding`, `design` keys), tasks are collected in workflow order and each is tagged with a `"phase"` field (e.g., `"user-support"`, `"onboarding"`, `"design"`). A future flat array format (where tasks already contain a `"phase"` field) is also supported.
3. Iterates through the flattened list and returns the **first** task whose `status` is **not** `completed` and **not** `not-required`.

**Output:**

A single JSON object containing all fields of the task plus an injected `"phase"` field:

```json
{
  "item": "goal",
  "phase": "onboarding",
  "stop-on-finish": false,
  "agent": {
    "count": 1,
    "effort": "high",
    "model": "opus",
    "skills": ["rex-onboarding-goal"]
  },
  "inputs": [],
  "outputs": ["rex/my-project/onboarding/goal.md"],
  "status": "not-started"
}
```

If all items are completed or not required, prints an informational message instead.

**Error cases:**

- `"No active project."` — if no project is currently active.
- `"Failed to read project-status.json: ..."` — if the file is missing or unreadable.
- `"project-status.json has unexpected format."` — if the file is neither an object nor an array.

---

### `rex project lock`

Locks the active project. When locked, agents must not work on the project and the operator will skip it immediately.

```
rex project lock
```

**Behavior:**

- Sets `locked: true` on the active project in `rex/projects.json`.

**Error cases:**

- `"No active project."` — if no project is currently active.

---

### `rex project unlock`

Unlocks the active project, allowing the operator and agents to work on it again.

```
rex project unlock
```

**Behavior:**

- Sets `locked: false` on the active project in `rex/projects.json`.

**Error cases:**

- `"No active project."` — if no project is currently active.

---

### `rex project get-completion-percent`

Returns the completion percentage of the active project as JSON.

```
rex project get-completion-percent
```

**Output:**

```json
{
  "project-items-percent": 33.33,
  "project-tasks-percent": 50.0
}
```

| Field                   | Description                                                                                      |
|-------------------------|--------------------------------------------------------------------------------------------------|
| `project-items-percent` | Percentage of project-status items completed out of actionable items (excludes `not-required`).   |
| `project-tasks-percent` | Percentage of planning tasks completed out of total tasks in `planning.json`.                     |

**Behavior:**

- If all project-status items are completed or not-required, both fields return `100.0`.
- If there are no planning tasks, `project-tasks-percent` is `0.0`.

**Error cases:**

- `"No active project."` — if no project is currently active.
- `"Failed to read project-status.json: ..."` — if the file is missing or unreadable.
