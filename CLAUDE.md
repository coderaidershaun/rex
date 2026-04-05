# Rex CLI

## Model and thinking level assignment

Always invoke /rex-model-router whenever assigned a task to ensure the correct agent works on the correct task. Confirm for the user what model, thinking level and context is working on the task.

## Maintenance notes

- **`COMMANDS_HELP` in `src/bin/main.rs`**: This is a manually maintained constant that lists every CLI command and subcommand. It powers `rex --commands` and the `rex --help` output. Whenever you add, remove, or rename a command, update `COMMANDS_HELP` to match.
