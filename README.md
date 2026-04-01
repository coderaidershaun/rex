<p align="center">
  <img src="static/logo.png" alt="Rex" width="200" />
</p>

<h1 align="center">Rex</h1>

<p align="center">
  A Rust CLI for harness management — structure, plan, and execute Rust exclusive projects with AI agents.
</p>

<p align="center">
  <a href="https://crates.io/crates/rex-cli"><img src="https://img.shields.io/crates/v/rex-cli.svg" alt="crates.io" /></a>
  <a href="https://github.com/coderaidershaun/rex/blob/main/LICENSE"><img src="https://img.shields.io/crates/l/rex-cli.svg" alt="license" /></a>
</p>

## Getting Started

### Install

```bash
cargo install rex-cli
```

### Initialize

Set up the rex harness in your project directory:

```bash
rex-cli init
```

### Create a Project

```bash
rex-cli project create
```

### Run the Operator

From within **Claude Code** or **Cursor**, invoke the rex operator skill:

```
/rex-operator
```

The operator takes it from there — walking you through onboarding, design, planning, and build phases step by step.

## How It Works

Rex manages projects through a structured pipeline:

1. **Onboarding** — define your goal, scope, risks, resources, and success measures
2. **Design** — architecture, modules, error handling, integration tests, library review
3. **Planning** — milestones, objectives, and tasks with dependency tracking
4. **Build** — agents execute tasks guided by the harness

Rex gives AI agents the scaffolding they need to build real software — tracking state across sessions, enforcing phase gates, and keeping work on the rails.

## CLI Reference

| Command | Description |
|---|---|
| `rex-cli init` | Initialize the harness in the current directory |
| `rex-cli project create` | Create a new project interactively |
| `rex-cli project get-active` | Show the current active project |
| `rex-cli project next-item` | Get the next actionable item |
| `rex-cli milestone upsert` | Create or update a milestone |
| `rex-cli objective upsert` | Create or update an objective |
| `rex-cli task upsert` | Create or update a task |
| `rex-cli task next` | Get the next task to work on |
| `rex-cli checklist list` | List checklist items |
| `rex-cli history list` | View session history |

Run `rex-cli --help` or `rex-cli <command> --help` for full usage details.

## License

[MIT](LICENSE)
