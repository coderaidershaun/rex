# rex

`rex` is a Rust CLI that manages the `.claude/` harness and project pipeline for AI-assisted development. It initialises the skill bundle in a repo, creates and activates projects, and exposes read-only JSON commands so pipeline agents can fetch project state without ad-hoc YAML parsing.

## Install

```sh
cargo install rex-cli
```

## User commands

| Command | Purpose |
|---|---|
| `rex init` | Extract or update the `.claude/` bundle in the current directory. |
| `rex init --force` | Overwrite all bundle files regardless of local modifications. |
| `rex create` | Create a new project interactively (prompts for title, category, complexity). |
| `rex activate <id>` | Activate an inactive project by ID, archiving the current active first. |

## Agent commands

These commands are agent-facing. They print JSON to stdout and exit 0 on success, non-zero when no active project exists.

| Command | Output |
|---|---|
| `rex project` | Full `rex/active/project.yaml` as JSON, including every step. |
| `rex project meta` | Project metadata only — no `steps` key. Matches the project envelope shape. |
| `rex project step` | First incomplete step as JSON, or `{"status":"all-steps-complete"}` when done. |
| `rex project step complete` | Mark the first incomplete step in `rex/active/project.yaml` as completed. Prints the just-completed step as JSON, or `{"status":"all-steps-complete"}` when nothing remains. |
| `rex project chunk-next` | Print the next pending chunk from `schedule.json` as JSON, or `{"status":"all-chunks-complete"}`. |
| `rex project chunk-prior` | Print the most recently completed chunk from `schedule.json`, or `{"status":"no-prior-chunk"}`. |
| `rex project task complete` | Mark the current task done in `schedule.json` and increment counters in `project.yaml`. Auto-promotes the parent chunk and phase when their tasks/chunks all reach `done`. Prints the updated task as JSON. |

## Schedule editing

CLI for editing `rex/active/<id>/schedule.json`. Every mutation auto-rewrites
`blocked_by` references and recomputes counters in `project.yaml`. Agents must
not edit `schedule.json` directly.

| Command | Action |
|---|---|
| `rex project schedule show` | Print the full schedule as JSON. |
| `rex project schedule replace --file <path>` | Atomically replace the schedule (used by the initial scheduler agent). |
| `rex project schedule phase add ...` | Append a phase. Returns the new phase as JSON. |
| `rex project schedule phase update <addr> ...` | Update phase fields. Renames rewrite refs. |
| `rex project schedule phase remove <addr>` | Remove phase + every chunk/task it owns. Drops dangling `blocked_by`. |
| `rex project schedule phase move <addr> --to <pos>` | Reorder phases. |
| `rex project schedule chunk add --phase <addr> ...` | Append a chunk to a phase. |
| `rex project schedule chunk update <addr> ...` | Update chunk fields. |
| `rex project schedule chunk remove <addr>` | Remove chunk + tasks. Drops dangling refs. |
| `rex project schedule chunk move <addr> [--to-phase <addr>] [--to <pos>]` | Reorder / re-parent. |
| `rex project schedule task add --chunk <addr> ...` | Append a task to a chunk. |
| `rex project schedule task update <addr> ...` | Update task fields. |
| `rex project schedule task remove <addr>` | Remove task. Drops dangling refs. |
| `rex project schedule task move <addr> [--to-chunk <addr>] [--to <pos>]` | Reorder / re-parent. |

`<addr>` is either a slug id or a 1-indexed dotted position (`1`, `1.2`, `1.2.3`).

## Further reading

- `.claude/skills/` — the agent skill set loaded by pipeline steps.
- `rex/pipeline.yaml` — the pipeline template used when creating new projects.
