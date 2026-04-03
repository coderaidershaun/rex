<p align="center">
  <img src="static/logo.png" alt="Rex" width="200" />
</p>

<h1 align="center">Rex-Cli</h1>

<p align="center">
  A CLI for Rust harness management — structure, plan, and execute Rust exclusive projects with AI agents.
</p>

<p align="center">
  <a href="https://crates.io/crates/rex-cli"><img src="https://img.shields.io/crates/v/rex-cli.svg" alt="crates.io" /></a>
  <a href="https://github.com/coderaidershaun/rex/blob/main/LICENSE"><img src="https://img.shields.io/crates/l/rex-cli.svg" alt="license" /></a>
</p>

## Getting Started

### 1. Install

```bash
cargo install rex-cli
```

### 2. Create a Project

```bash
rex project create
```

Interactive prompts walk you through project ID, complexity, title, category, and which onboarding/design items to include. You'll be asked whether to initialize the rex harness **inside** the project directory — creating a fully self-contained project with its own skills, hooks, and `rex/projects.json`. No separate `rex init` step needed.

If you prefer to initialize the harness first (e.g., at a monorepo root), you can run `rex init` before creating a project.

### 3. Run the Operator

From within **Claude Code** or **Cursor**, invoke the rex operator skill:

```
/rex-operator
```

The operator takes it from there — walking you through onboarding, design, planning, and build phases step by step. Each invocation processes one work item, then stops.

### 4. Autorun (Headless Autopilot)

Run the entire project autonomously with Telegram notifications for status updates and human input prompts.

**Telegram environment variables must be available:**

```bash
export TELEGRAM_BOT_TOKEN="your-bot-token-from-botfather"
export TELEGRAM_CHAT_ID="your-numeric-chat-id"
```

Or create a `.env` file in your project root:

```env
TELEGRAM_BOT_TOKEN=your-bot-token-from-botfather
TELEGRAM_CHAT_ID=your-numeric-chat-id
```

Then start autorun:

```bash
rex-autorun
```

Autorun options:

| Flag | Default | Description |
|------|---------|-------------|
| `--project-dir <PATH>` | `.` | Rex project root directory |
| `--max-budget-usd <AMOUNT>` | `50.0` | Max USD per single Claude invocation |
| `--max-total-budget-usd <AMOUNT>` | `500.0` | Hard stop for total spend |
| `--max-turns <N>` | `200` | Max agentic turns per invocation |
| `--process-timeout-mins <N>` | `60` | Max minutes per Claude process |
| `--max-retries <N>` | `5` | Max retries for transient failures |
| `--human-timeout-days <N>` | `7` | Max days to wait for Telegram reply |
| `--log-file <PATH>` | `.rex-autorun.log` | Path to JSONL log file |

Autorun recovers from crashes automatically, respects budget limits, and exits cleanly when the project is done.

## How It Works

Rex manages projects through a structured pipeline:

1. **Onboarding** — define your goal, scope, risks, resources, and success measures
2. **Design** — architecture, modules, error handling, integration tests, library review
3. **Planning** — milestones, objectives, and tasks with dependency tracking
4. **Build** — agents execute tasks guided by the harness

Rex gives AI agents the scaffolding they need to build real software — tracking state across sessions, enforcing phase gates, and keeping work on the rails.

## CLI Reference

### Project Management

| Command | Description |
|---|---|
| `rex init` | Initialize the harness in the current directory |
| `rex project create` | Create a new project interactively |
| `rex project get-active` | Show the current active project |
| `rex project activate <ID>` | Switch to a different project |
| `rex project remove <ID>` | Remove a project |
| `rex project next-item` | Get the next actionable work item (JSON) |
| `rex project lock` | Lock the active project |
| `rex project unlock` | Unlock the active project |
| `rex project update-title <TITLE>` | Update project title |
| `rex project update-subtitle <SUBTITLE>` | Update project subtitle |
| `rex project update-description <DESC>` | Update project description |
| `rex project update-directory <PATH>` | Change project directory |
| `rex project update-status <ITEM> <STATUS>` | Update a work item's status |
| `rex project update-category <CATEGORY>` | Set category (binary/library/refactor) |
| `rex project update-complexity <COMPLEXITY>` | Set complexity (low/medium/high) |

### Planning Tree

| Command | Description |
|---|---|
| `rex milestone upsert` | Create or update a milestone |
| `rex milestone get <ID>` | Get a milestone by ID |
| `rex milestone list` | List milestones |
| `rex milestone remove <ID>` | Remove a milestone |
| `rex objective upsert` | Create or update an objective |
| `rex objective get <ID>` | Get an objective by ID |
| `rex objective list` | List objectives |
| `rex objective remove <ID>` | Remove an objective |
| `rex task upsert` | Create or update a task |
| `rex task get <ID>` | Get a task by ID |
| `rex task list` | List tasks |
| `rex task next` | Get the next task to work on |
| `rex task remove <ID>` | Remove a task |

### Checklist & History

| Command | Description |
|---|---|
| `rex checklist init` | Initialize an empty checklist |
| `rex checklist add` | Add a checklist item |
| `rex checklist list` | List checklist items |
| `rex checklist complete <ID>` | Mark a checklist item as done |
| `rex checklist uncomplete <ID>` | Mark a checklist item as not done |
| `rex checklist set-context <CTX>` | Set checklist context text |
| `rex history list` | View all session history |
| `rex history get-recent` | View recent history entries |

### Monorepo

| Command | Description |
|---|---|
| `rex mono init --name <NAME>` | Create a Cargo workspace monorepo with rex harness and git |
| `rex mono empty --name <NAME>` | Create an empty Cargo workspace (no rex or claude folders) |

Run `rex --help`, `rex --commands`, or `rex <command> --help` for full usage details.

## License

[MIT](LICENSE)
