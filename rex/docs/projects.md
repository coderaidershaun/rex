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
| `title`       | yes      | Project title                                    |
| `subtitle`    | yes      | Short one-line summary                           |
| `description` | yes      | Detailed project description                     |
| `directory`   | yes      | Absolute path to the project's working directory |
| `user_name`   | no       | Name of the project owner                        |

---

## Commands

All commands that edit a project (`update-directory`, `update-status`) operate on the **active** project. Use `get-active` to check which project is active, and `activate` to switch.

### `rex project create`

Interactively creates a new project and sets it as **active**.

```
rex project create
```

**Prompts (in order):**

1. **Project ID** — must be unique across all projects (active and inactive).
2. **Category** — select from `binary`, `library`, `refactor`.
3. **Title** — project title.
4. **Subtitle** — short summary.
5. **Description** — detailed description.
6. **Directory** — the project's working directory. If a folder matching the project ID exists in the current working directory, it is auto-detected and offered as the default. Otherwise the user types a path manually.
7. **User Name** — optional, press Enter to skip.

After all prompts, a **summary** is displayed and the user must **confirm** before anything is written.

**Behavior on confirm:**

- If there is already an active project, it is moved to `inactive`.
- The new project becomes `active`.
- The directory `rex/<project-id>/` is created.
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

### `rex project update-directory <DIRECTORY>`

Updates the active project's directory path.

```
rex project update-directory /path/to/new/directory
```

**Behavior:**

- Updates the `directory` field of the active project in `rex/projects.json`.
- Prints the old and new directory paths.

**Error cases:**

- `"No active project."` — if no project is currently active.

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
- Searches both `onboarding` and `user_support` lists for the matching item.
- Updates the item's `status` field and saves.

**Error cases:**

- `"No active project."` — if no project is currently active.
- `"Item X not found in project status."` — if the item name doesn't match any entry.
