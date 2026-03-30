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
