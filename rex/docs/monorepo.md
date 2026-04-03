# Rex Mono

Create and manage Cargo workspace monorepos — either with the full rex harness pre-configured, or as an empty workspace for manual setup.

## Commands

### `rex mono --name <name> [--no-harness] [--with-git-repo <public|private>]`

Creates a Cargo workspace monorepo. By default, the rex harness (skills, hooks, docs, project registry) is included. Pass `--no-harness` to create a bare workspace without it. Pass `--with-git-repo` to create a GitHub repository via the `gh` CLI and add it as the `origin` remote.

```bash
rex mono --name my-workspace                            # With rex harness
rex mono --name my-workspace --no-harness               # Empty workspace
rex mono --name my-workspace --with-git-repo private    # With private GitHub repo
```

**What it does:**

1. **Directory** — creates `<name>/`
2. **Workspace Cargo.toml** — configures a Cargo workspace with `resolver = "2"` and `members = ["libs/*"]`
3. **libs/** — empty directory (with `.gitkeep`) where project crates will live
4. **.gitignore** — includes `/target`, `.env`, and `.env.*`
5. **git init** — initializes a git repository
6. **GitHub repo** *(optional)* — if `--with-git-repo` is passed, creates a GitHub repository (public or private) via `gh repo create` and adds it as the `origin` remote. Requires the [GitHub CLI](https://cli.github.com/) to be installed and authenticated.
7. **rex init** — runs the full rex harness initialization inside the new directory (skills, hooks, docs, registry). *Skipped when `--no-harness` is passed.*

**Resulting structure (default):**

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

With `--no-harness`, step 6 is skipped and the resulting structure contains only the workspace files — no `.claude/`, `rex/`, or `CLAUDE.md`. Use this when you want a workspace without agent orchestration, or when you plan to run `rex init` and `rex project create` separately (e.g., initializing rex inside individual project directories rather than at the workspace root).

## Adding Projects

Once the monorepo is created, add project crates under `libs/`:

```bash
cd my-workspace
rex project create
# When prompted for directory, use: libs/my-crate
```

The workspace `Cargo.toml` already has `members = ["libs/*"]`, so any crate created under `libs/` is automatically included in the workspace.

When using `rex mono --name <name> --no-harness`, you can initialize the rex harness inside each project directory during `rex project create` — the create command will prompt you to choose whether to run `rex init` inside the project.

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
| `gh repo create failed: ...` | The GitHub CLI (`gh`) is not installed, not authenticated, or a repo with that name already exists |
