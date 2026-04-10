use crate::errors::{RexError, RexResult};
use console::style;
use include_dir::{include_dir, Dir};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

// Skills — same SKILL.md format for both Claude Code and Cursor.
// Always embedded from .claude/skills/ (canonical source); copied to the
// harness-appropriate target directory at init time.
static SKILLS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/skills");

// Hooks — harness-specific (different env vars in the shell scripts).
#[cfg(feature = "claude")]
static HOOKS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/hooks");
#[cfg(feature = "cursor")]
static HOOKS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/.cursor/hooks");

static DOCS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/rex/docs");

// Settings file — different format per harness.
#[cfg(feature = "claude")]
static HARNESS_SETTINGS: &str = include_str!("../../.claude/settings.json");
#[cfg(feature = "cursor")]
static HARNESS_SETTINGS: &str = include_str!("../../.cursor/hooks.json");

// Harness-specific constants.
#[cfg(feature = "claude")]
const CONFIG_DIR_NAME: &str = ".claude";
#[cfg(feature = "claude")]
const ROOT_FILE_NAME: &str = "CLAUDE.md";
#[cfg(feature = "claude")]
const HARNESS_LABEL: &str = "Claude Code";
#[cfg(feature = "claude")]
const SETTINGS_FILE_NAME: &str = "settings.json";

#[cfg(feature = "cursor")]
const CONFIG_DIR_NAME: &str = ".cursor";
#[cfg(feature = "cursor")]
const ROOT_FILE_NAME: &str = "AGENTS.md";
#[cfg(feature = "cursor")]
const HARNESS_LABEL: &str = "Cursor";
#[cfg(feature = "cursor")]
const SETTINGS_FILE_NAME: &str = "hooks.json";

#[cfg(feature = "claude")]
const ROOT_FILE_CONTENT: &str = "\
# Rex Harness

This project uses the **rex** agent-orchestrated development harness.

## Getting Started

See [rex/docs/README.md](rex/docs/README.md) for the complete end-to-end process — how the operator \
orchestrates onboarding, design, planning, and execution phases.

## Quick Reference

- `rex project create` — start a new project
- `rex project next-item` — see what's next
- `/rex-operator` — run the operator (advances the project one step)

## Research

Any research agents perform should be saved to `rex/docs/research/` unless the agent's instructions specify a different location.

## Agent Model Selection

When assigning agents to tasks, use the `rex-model-router` skill as the single source of truth for model, effort, and context decisions. The routing tables are optimised for Rust development but the same principles apply to all work: match the model to the task's complexity, escalate when prior attempts fail, and use 1M context when the task spans multiple files or modules.

## CLI Docs

| Document | Covers |
|----------|--------|
| [projects](rex/docs/projects.md) | Project lifecycle and registry |
| [checklist](rex/docs/checklist.md) | Onboarding/design/planning checklists |
| [milestones](rex/docs/milestones.md) | Major project checkpoints |
| [objectives](rex/docs/objectives.md) | Strategic outcomes under milestones |
| [tasks](rex/docs/tasks.md) | Atomic work units and priority scoring |
| [history](rex/docs/history.md) | Session history tracking |
| [operator](rex/docs/operator.md) | The orchestration heartbeat |
| [planning](rex/docs/planning-overview.md) | Three-level planning hierarchy |

## Table of Contents

When file structure changes, this MUST be kept up-to-date.

### Root

| File | Purpose |
|------|---------|
| `Cargo.toml` | Package manifest — dependencies, features, dev-dependencies |
| `CLAUDE.md` | This file — project context and quick reference |
| `.gitignore` | Git ignore rules — `/target`, `.env`, `.env.*` |

### `src/`

| File | Purpose |
|------|---------|
| `main.rs` or `lib.rs` | Crate entry point (binary or library depending on project category) |

### `.claude/`

| File | Purpose |
|------|---------|
| `settings.json` | Claude Code configuration — hooks and permissions |
| `hooks/commit-and-push.sh` | Auto-commit hook triggered on agent stop |
| `skills/` | Skill definitions used by the rex operator and agents |

### `rex/`

| File | Purpose |
|------|---------|
| `projects.json` | Project registry — active project and inactive list |
| `docs/README.md` | End-to-end process overview |
| `docs/projects.md` | Project lifecycle and registry docs |
| `docs/checklist.md` | Onboarding, design, and planning checklists |
| `docs/milestones.md` | Major project checkpoints |
| `docs/objectives.md` | Strategic outcomes under milestones |
| `docs/tasks.md` | Atomic work units and priority scoring |
| `docs/history.md` | Session history tracking |
| `docs/operator.md` | The orchestration heartbeat |
| `docs/planning-overview.md` | Three-level planning hierarchy |
| `docs/monorepo.md` | Monorepo support and configuration |
| `docs/init.md` | Harness initialization reference |
| `docs/rex-autorun.md` | Autorun loop documentation |
| `docs/telegram.md` | Telegram integration guide |
| `docs/research/` | Research output directory (initially empty) |

### `rex/<project-id>/`

| File | Purpose |
|------|---------|
| `project-status.json` | Full project status — onboarding, design, planning, execution steps |
| `onboarding/` | Onboarding phase outputs |
| `user-support/` | User support artifacts |
| `design/` | Design phase outputs |
| `planning/` | Planning phase outputs |
| `execution/` | Execution phase outputs |
| `uat/` | User acceptance testing outputs |
";

#[cfg(feature = "cursor")]
const ROOT_FILE_CONTENT: &str = "\
# Rex Harness

This project uses the **rex** agent-orchestrated development harness.

## Getting Started

See [rex/docs/README.md](rex/docs/README.md) for the complete end-to-end process — how the operator \
orchestrates onboarding, design, planning, and execution phases.

## Quick Reference

- `rex project create` — start a new project
- `rex project next-item` — see what's next
- `/rex-operator` — run the operator (advances the project one step)

## Research

Any research agents perform should be saved to `rex/docs/research/` unless the agent's instructions specify a different location.

## Agent Model Selection

When assigning agents to tasks, use the `rex-model-router` skill as the single source of truth for model, effort, and context decisions. The routing tables are optimised for Rust development but the same principles apply to all work: match the model to the task's complexity, escalate when prior attempts fail, and use 1M context when the task spans multiple files or modules.

## CLI Docs

| Document | Covers |
|----------|--------|
| [projects](rex/docs/projects.md) | Project lifecycle and registry |
| [checklist](rex/docs/checklist.md) | Onboarding/design/planning checklists |
| [milestones](rex/docs/milestones.md) | Major project checkpoints |
| [objectives](rex/docs/objectives.md) | Strategic outcomes under milestones |
| [tasks](rex/docs/tasks.md) | Atomic work units and priority scoring |
| [history](rex/docs/history.md) | Session history tracking |
| [operator](rex/docs/operator.md) | The orchestration heartbeat |
| [planning](rex/docs/planning-overview.md) | Three-level planning hierarchy |

## Table of Contents

When file structure changes, this MUST be kept up-to-date.

### Root

| File | Purpose |
|------|---------|
| `Cargo.toml` | Package manifest — dependencies, features, dev-dependencies |
| `AGENTS.md` | This file — project context and quick reference |
| `.gitignore` | Git ignore rules — `/target`, `.env`, `.env.*` |

### `src/`

| File | Purpose |
|------|---------|
| `main.rs` or `lib.rs` | Crate entry point (binary or library depending on project category) |

### `.cursor/`

| File | Purpose |
|------|---------|
| `hooks.json` | Cursor configuration — hooks |
| `hooks/commit-and-push.sh` | Auto-commit hook triggered on agent stop |
| `skills/` | Skill definitions used by the rex operator and agents |

### `rex/`

| File | Purpose |
|------|---------|
| `projects.json` | Project registry — active project and inactive list |
| `docs/README.md` | End-to-end process overview |
| `docs/projects.md` | Project lifecycle and registry docs |
| `docs/checklist.md` | Onboarding, design, and planning checklists |
| `docs/milestones.md` | Major project checkpoints |
| `docs/objectives.md` | Strategic outcomes under milestones |
| `docs/tasks.md` | Atomic work units and priority scoring |
| `docs/history.md` | Session history tracking |
| `docs/operator.md` | The orchestration heartbeat |
| `docs/planning-overview.md` | Three-level planning hierarchy |
| `docs/monorepo.md` | Monorepo support and configuration |
| `docs/init.md` | Harness initialization reference |
| `docs/rex-autorun.md` | Autorun loop documentation |
| `docs/telegram.md` | Telegram integration guide |
| `docs/research/` | Research output directory (initially empty) |

### `rex/<project-id>/`

| File | Purpose |
|------|---------|
| `project-status.json` | Full project status — onboarding, design, planning, execution steps |
| `onboarding/` | Onboarding phase outputs |
| `user-support/` | User support artifacts |
| `design/` | Design phase outputs |
| `planning/` | Planning phase outputs |
| `execution/` | Execution phase outputs |
| `uat/` | User acceptance testing outputs |
";

pub fn init() -> RexResult<()> {
    println!();
    println!("  {}", style("Rex Init").bold().cyan());
    println!("  {}", style("\u{2500}".repeat(40)).dim());
    println!();

    let cwd = std::env::current_dir()?;
    let config_dir = cwd.join(CONFIG_DIR_NAME);
    let skills_dir = config_dir.join("skills");
    let hooks_dir = config_dir.join("hooks");
    let rex_dir = cwd.join("rex");
    let docs_dir = rex_dir.join("docs");
    let root_file = cwd.join(ROOT_FILE_NAME);

    let mut created = Vec::new();
    let mut skipped = Vec::new();

    // 2. Copy skills (same SKILL.md format for both Claude Code and Cursor)
    copy_embedded_dir(&SKILLS_DIR, &skills_dir, &mut created, &mut skipped)?;

    // 3. Copy hook scripts and make them executable
    copy_embedded_dir(&HOOKS_DIR, &hooks_dir, &mut created, &mut skipped)?;
    for entry in HOOKS_DIR.files() {
        let target = hooks_dir.join(entry.path());
        if target.exists() {
            let mut perms = fs::metadata(&target)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&target, perms)?;
        }
    }

    // 4. Write hook/settings configuration
    write_harness_settings(&config_dir, &mut created, &mut skipped)?;

    // 5. Copy rex/docs/*
    fs::create_dir_all(&docs_dir)?;
    copy_embedded_dir(&DOCS_DIR, &docs_dir, &mut created, &mut skipped)?;

    // 5b. Ensure rex/docs/research/ exists (empty dirs aren't captured by include_dir)
    let research_dir = docs_dir.join("research");
    if !research_dir.exists() {
        fs::create_dir_all(&research_dir)?;
        created.push("rex/docs/research/".into());
    }

    // 6. Create empty rex/projects.json if missing
    let projects_path = rex_dir.join("projects.json");
    if !projects_path.exists() {
        fs::write(
            &projects_path,
            "{\n  \"active\": null,\n  \"inactive\": []\n}\n",
        )?;
        created.push("rex/projects.json".into());
    } else {
        skipped.push("rex/projects.json (already exists)".into());
    }

    // 7. Create or update root file (CLAUDE.md or AGENTS.md)
    if !root_file.exists() {
        fs::write(&root_file, ROOT_FILE_CONTENT)?;
        created.push(ROOT_FILE_NAME.into());
    } else {
        let existing = fs::read_to_string(&root_file)?;
        if existing.contains("rex/docs/README.md") {
            skipped.push(format!("{ROOT_FILE_NAME} (rex section already present)"));
        } else {
            let mut content = existing;
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push('\n');
            content.push_str(ROOT_FILE_CONTENT);
            fs::write(&root_file, content)?;
            created.push(format!("{ROOT_FILE_NAME} (appended rex section)"));
        }
    }

    // 8. Report results
    println!();
    if !created.is_empty() {
        println!(
            "  {} Created/updated {} items:",
            style("\u{2713}").green().bold(),
            created.len()
        );
        for item in &created {
            println!("    {} {item}", style("+").green());
        }
    }
    if !skipped.is_empty() {
        println!();
        println!(
            "  {} Skipped {} existing items:",
            style("\u{2139}").blue().bold(),
            skipped.len()
        );
        for item in &skipped {
            println!("    {} {item}", style("\u{2013}").dim());
        }
    }
    println!();
    println!(
        "  {} Rex harness initialized for {}.",
        style("\u{2713}").green().bold(),
        style(HARNESS_LABEL).cyan().bold()
    );
    println!(
        "  {} See {} for the full process.",
        style("\u{2192}").cyan(),
        style("rex/docs/README.md").underlined()
    );
    println!();

    Ok(())
}

// ---------------------------------------------------------------------------
// Settings: harness-specific configuration files
// ---------------------------------------------------------------------------

fn write_harness_settings(
    config_dir: &Path,
    created: &mut Vec<String>,
    skipped: &mut Vec<String>,
) -> RexResult<()> {
    let settings_path = config_dir.join(SETTINGS_FILE_NAME);
    if !settings_path.exists() {
        fs::create_dir_all(config_dir)?;
        fs::write(&settings_path, HARNESS_SETTINGS)?;
        created.push(format!("{CONFIG_DIR_NAME}/{SETTINGS_FILE_NAME}"));
    } else {
        let merged = merge_harness_settings(&settings_path)?;
        if let Some(new_content) = merged {
            fs::write(&settings_path, new_content)?;
            created.push(format!("{CONFIG_DIR_NAME}/{SETTINGS_FILE_NAME} (merged hooks)"));
        } else {
            skipped.push(format!("{CONFIG_DIR_NAME}/{SETTINGS_FILE_NAME} (hooks already present)"));
        }
    }
    Ok(())
}

/// Merge rex hooks into an existing harness settings file.
fn merge_harness_settings(
    existing_path: &Path,
) -> RexResult<Option<String>> {
    let existing_str = fs::read_to_string(existing_path)?;
    if existing_str.contains("commit-and-push") {
        return Ok(None);
    }

    let mut existing: serde_json::Value = serde_json::from_str(&existing_str)?;
    let rex: serde_json::Value = serde_json::from_str(HARNESS_SETTINGS)?;

    let existing_obj = existing
        .as_object_mut()
        .ok_or_else(|| RexError::Validation("settings file is not an object".into()))?;

    if let Some(rex_hooks) = rex.get("hooks") {
        if let Some(existing_hooks) = existing_obj.get_mut("hooks") {
            let existing_hooks_obj = existing_hooks
                .as_object_mut()
                .ok_or_else(|| RexError::Validation("hooks is not an object".into()))?;
            if let Some(rex_hooks_obj) = rex_hooks.as_object() {
                for (event, handlers) in rex_hooks_obj {
                    if !existing_hooks_obj.contains_key(event) {
                        existing_hooks_obj.insert(event.clone(), handlers.clone());
                    } else if let (Some(existing_arr), Some(new_arr)) = (
                        existing_hooks_obj
                            .get_mut(event)
                            .and_then(|v| v.as_array_mut()),
                        handlers.as_array(),
                    ) {
                        for handler in new_arr {
                            existing_arr.push(handler.clone());
                        }
                    }
                }
            }
        } else {
            existing_obj.insert("hooks".into(), rex_hooks.clone());
        }
    }

    let result = serde_json::to_string_pretty(&existing)? + "\n";
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Recursively copy an embedded directory into a target path, tracking created/skipped files.
fn copy_embedded_dir(
    embedded: &Dir,
    target: &Path,
    created: &mut Vec<String>,
    skipped: &mut Vec<String>,
) -> RexResult<()> {
    use include_dir::DirEntry;

    fn walk<'a>(
        entries: &'a [DirEntry<'a>],
        target: &Path,
        created: &mut Vec<String>,
        skipped: &mut Vec<String>,
    ) -> RexResult<()> {
        for entry in entries {
            match entry {
                DirEntry::File(file) => {
                    let dest = target.join(file.path());
                    if dest.exists() {
                        skipped.push(relative_display(&dest));
                    } else {
                        if let Some(parent) = dest.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::write(&dest, file.contents())?;
                        created.push(relative_display(&dest));
                    }
                }
                DirEntry::Dir(dir) => {
                    walk(dir.entries(), target, created, skipped)?;
                }
            }
        }
        Ok(())
    }

    walk(embedded.entries(), target, created, skipped)
}

/// Try to make a path relative to CWD for display, fall back to full path.
fn relative_display(path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = path.strip_prefix(&cwd) {
            return rel.display().to_string();
        }
    }
    path.display().to_string()
}
