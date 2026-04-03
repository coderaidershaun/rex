# Rex Mono

Initialize a Cargo workspace monorepo with the rex harness pre-configured — a single command to create a shared repository for multiple projects.

## Usage

```bash
rex mono init --name my-workspace
```

## What It Does

Creates a new directory with everything needed to start building multiple Rust crates in one repository:

1. **Directory** — creates `<name>/`
2. **Workspace Cargo.toml** — configures a Cargo workspace with `resolver = "2"` and `members = ["libs/*"]`
3. **libs/** — empty directory (with `.gitkeep`) where project crates will live
4. **.gitignore** — includes `/target`, `.env`, and `.env.*`
5. **git init** — initializes a git repository
6. **rex init** — runs the full rex harness initialization inside the new directory (skills, hooks, docs, registry)

### Resulting Structure

```
my-workspace/
  Cargo.toml              # [workspace] with members = ["libs/*"]
  .gitignore              # /target, .env, .env.*
  .git/
  .claude/
    skills/               # All rex and rust agent skills
    hooks/                # commit-and-push.sh
    settings.json         # Hook configuration
  rex/
    docs/                 # CLI and process documentation
    projects.json         # Empty project registry
  libs/
    .gitkeep
  CLAUDE.md
```

## Adding Projects

Once the monorepo is created, add project crates under `libs/`:

```bash
cd my-workspace
rex project create
# When prompted for directory, use: libs/my-crate
```

The workspace `Cargo.toml` already has `members = ["libs/*"]`, so any crate created under `libs/` is automatically included in the workspace.

### Shared Dependencies

Add shared dependency versions at the workspace level:

```toml
# my-workspace/Cargo.toml
[workspace]
resolver = "2"
members = ["libs/*"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

Then reference them from individual crates:

```toml
# libs/my-crate/Cargo.toml
[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
```

## Error Handling

| Error | Cause |
|-------|-------|
| `Directory "<name>" already exists.` | A directory with that name already exists in the current directory |
| `git init failed: ...` | Git is not installed or the directory is inside a repository that forbids nested inits |
| `rex init failed.` | The rex initialization step failed — check the output above the error for details |
