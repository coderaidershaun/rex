use crate::errors::{RexError, RexResult};
use crate::models::project::RepoVisibility;
use console::style;
use std::path::Path;
use std::process::Command;

fn check_gh_installed() -> RexResult<()> {
    let result = Command::new("gh").arg("--version").output();
    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(RexError::Subprocess {
            command: "gh --version".into(),
            detail: "The GitHub CLI (gh) is not installed or not on PATH. \
                     Install it from https://cli.github.com/"
                .into(),
        }),
    }
}

pub fn is_git_repo(dir: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn ensure_git_init(dir: &Path) -> RexResult<bool> {
    if is_git_repo(dir) {
        return Ok(false);
    }
    let output = Command::new("git").arg("init").current_dir(dir).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RexError::Subprocess {
            command: "git init".into(),
            detail: stderr.to_string(),
        });
    }
    Ok(true)
}

pub fn create_github_repo(
    name: &str,
    visibility: RepoVisibility,
    dir: &Path,
) -> RexResult<()> {
    check_gh_installed()?;

    let output = Command::new("gh")
        .args([
            "repo",
            "create",
            name,
            visibility.gh_flag(),
            "--source=.",
            "--remote=origin",
        ])
        .current_dir(dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RexError::Subprocess {
            command: format!("gh repo create {name}"),
            detail: stderr.to_string(),
        });
    }

    println!(
        "  {} Created {} GitHub repo: {name}",
        style("\u{2713}").green().bold(),
        visibility,
    );
    Ok(())
}
