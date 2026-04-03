use crate::errors::{RexError, RexResult};
use console::style;
use include_dir::{include_dir, Dir};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

static SKILLS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/skills");
static HOOKS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/hooks");
static DOCS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/rex/docs");

// Claude Code: hooks live inside settings.json
static CLAUDE_SETTINGS_JSON: &str = include_str!("../../.claude/settings.json");

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
";

pub fn init() -> RexResult<()> {
    println!();
    println!("  {}", style("Rex Init").bold().cyan());
    println!("  {}", style("\u{2500}".repeat(40)).dim());
    println!();

    let cwd = std::env::current_dir()?;
    let config_dir = cwd.join(".claude");
    let skills_dir = config_dir.join("skills");
    let hooks_dir = config_dir.join("hooks");
    let rex_dir = cwd.join("rex");
    let docs_dir = rex_dir.join("docs");
    let root_file = cwd.join("CLAUDE.md");

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
    write_claude_settings(&config_dir, &mut created, &mut skipped)?;

    // 5. Copy rex/docs/*
    fs::create_dir_all(&docs_dir)?;
    copy_embedded_dir(&DOCS_DIR, &docs_dir, &mut created, &mut skipped)?;

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

    // 7. Create or update CLAUDE.md
    if !root_file.exists() {
        fs::write(&root_file, ROOT_FILE_CONTENT)?;
        created.push("CLAUDE.md".into());
    } else {
        let existing = fs::read_to_string(&root_file)?;
        if existing.contains("rex/docs/README.md") {
            skipped.push("CLAUDE.md (rex section already present)".into());
        } else {
            let mut content = existing;
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push('\n');
            content.push_str(ROOT_FILE_CONTENT);
            fs::write(&root_file, content)?;
            created.push("CLAUDE.md (appended rex section)".into());
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
        style("Claude Code").cyan().bold()
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
// Claude Code: hooks inside .claude/settings.json
// ---------------------------------------------------------------------------

fn write_claude_settings(
    config_dir: &Path,
    created: &mut Vec<String>,
    skipped: &mut Vec<String>,
) -> RexResult<()> {
    let settings_path = config_dir.join("settings.json");
    if !settings_path.exists() {
        fs::create_dir_all(config_dir)?;
        fs::write(&settings_path, CLAUDE_SETTINGS_JSON)?;
        created.push(".claude/settings.json".into());
    } else {
        let merged = merge_claude_settings(&settings_path)?;
        if let Some(new_content) = merged {
            fs::write(&settings_path, new_content)?;
            created.push(".claude/settings.json (merged hooks)".into());
        } else {
            skipped.push(".claude/settings.json (hooks already present)".into());
        }
    }
    Ok(())
}

/// Merge rex hooks into an existing Claude Code settings.json.
/// Claude Code format nests hooks under event keys with matcher + hooks arrays:
///
/// ```json
/// { "hooks": { "Stop": [{ "matcher": "", "hooks": [{ "type": "command", "command": "..." }] }] } }
/// ```
fn merge_claude_settings(
    existing_path: &Path,
) -> RexResult<Option<String>> {
    let existing_str = fs::read_to_string(existing_path)?;
    if existing_str.contains("commit-and-push") {
        return Ok(None);
    }

    let mut existing: serde_json::Value = serde_json::from_str(&existing_str)?;
    let rex: serde_json::Value = serde_json::from_str(CLAUDE_SETTINGS_JSON)?;

    let existing_obj = existing
        .as_object_mut()
        .ok_or_else(|| RexError::Validation("settings.json is not an object".into()))?;

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
