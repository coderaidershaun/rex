# Rex CLI

## Maintenance notes

- **`COMMANDS_HELP` in `src/bin/main.rs`**: This is a manually maintained constant that lists every CLI command and subcommand. It powers `rex --commands` and the `rex --help` output. Whenever you add, remove, or rename a command, update `COMMANDS_HELP` to match.
