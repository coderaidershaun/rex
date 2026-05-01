use std::{fs, path::Path};

use crate::{
    bundle::{Bundle, apply_bundle},
    error::RexError,
};

/// Run `rex init` against `cwd`.
///
/// Extracts the bundle with three-way merge, ensures `rex/active/` exists,
/// and generates `CLAUDE.md` if absent.
///
/// # Errors
/// - [`RexError::Io`] for filesystem failures
/// - [`RexError::JsonParse`] / [`RexError::JsonSerialize`] for manifest I/O
pub fn run(cwd: &Path, bundle: &Bundle, force: bool) -> Result<(), RexError> {
    let summary = apply_bundle(bundle, cwd, force)?;

    let active_dir = cwd.join("rex/active");
    if !active_dir.exists() {
        fs::create_dir_all(&active_dir).map_err(|source| RexError::Io {
            path: active_dir.clone(),
            source,
        })?;
    }

    generate_claude_md(cwd, bundle)?;

    println!(
        "rex init: {} written, {} upgraded, {} preserved, {} conflicts, {} unchanged",
        summary.written, summary.upgraded, summary.preserved, summary.conflicts, summary.noops,
    );

    if summary.conflicts > 0 {
        println!(
            "  {} conflict(s): bundle version written to .rex-new sibling files",
            summary.conflicts
        );
    }

    Ok(())
}

fn generate_claude_md(cwd: &Path, bundle: &Bundle) -> Result<(), RexError> {
    let claude_md = cwd.join("CLAUDE.md");
    if claude_md.exists() {
        return Ok(());
    }

    let tmpl_bytes = bundle.read_file(Path::new("templates/CLAUDE.md.tmpl"))?;
    let tmpl = String::from_utf8_lossy(&tmpl_bytes);

    let title_placeholder = "My Project";
    let top_level_dirs = top_level_dir_list(cwd);

    let content = tmpl
        .replace("{{PROJECT_TITLE}}", title_placeholder)
        .replace("{{TOP_LEVEL_DIRS}}", &top_level_dirs);

    fs::write(&claude_md, content.as_bytes()).map_err(|source| RexError::Io {
        path: claude_md,
        source,
    })
}

fn top_level_dir_list(cwd: &Path) -> String {
    let Ok(entries) = fs::read_dir(cwd) else {
        return String::new();
    };

    let mut dirs: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                None
            } else {
                Some(format!("- `{}/`", name))
            }
        })
        .collect();

    dirs.sort();
    dirs.join("\n")
}
