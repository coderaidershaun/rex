use crate::errors::{RexError, RexResult};
use console::style;
use std::fs;
use std::process::Command;

pub fn empty(name: &str) -> RexResult<()> {
    println!();
    println!("  {}", style("Rex Mono Empty").bold().cyan());
    println!("  {}", style("\u{2500}".repeat(40)).dim());
    println!();

    let cwd = std::env::current_dir()?;
    let repo_dir = cwd.join(name);

    if repo_dir.exists() {
        return Err(RexError::AlreadyExists(format!(
            "Directory \"{name}\" already exists."
        )));
    }

    // 1. Create the monorepo directory
    fs::create_dir_all(&repo_dir)?;
    println!(
        "  {} Created directory: {name}/",
        style("\u{2713}").green().bold(),
    );

    // 2. Create workspace Cargo.toml
    fs::write(
        repo_dir.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"libs/*\"]\n",
    )?;
    println!(
        "  {} Created workspace Cargo.toml",
        style("\u{2713}").green().bold(),
    );

    // 3. Create libs/ directory with .gitkeep
    let libs_dir = repo_dir.join("libs");
    fs::create_dir_all(&libs_dir)?;
    fs::write(libs_dir.join(".gitkeep"), "")?;
    println!(
        "  {} Created libs/",
        style("\u{2713}").green().bold(),
    );

    // 4. Create .gitignore
    fs::write(repo_dir.join(".gitignore"), "/target\n.env\n.env.*\n")?;
    println!(
        "  {} Created .gitignore",
        style("\u{2713}").green().bold(),
    );

    // 5. git init
    let git_output = Command::new("git")
        .arg("init")
        .current_dir(&repo_dir)
        .output()?;
    if !git_output.status.success() {
        let stderr = String::from_utf8_lossy(&git_output.stderr);
        return Err(RexError::Subprocess {
            command: "git init".into(),
            detail: stderr.to_string(),
        });
    }
    println!(
        "  {} Initialized git repository",
        style("\u{2713}").green().bold(),
    );

    // 6. Summary
    println!();
    println!(
        "  {} Empty monorepo \"{}\" is ready.",
        style("\u{2713}").green().bold(),
        name
    );
    println!(
        "  {} cd {name}",
        style("\u{2192}").cyan(),
    );
    println!();

    Ok(())
}

pub fn init(name: &str) -> RexResult<()> {
    println!();
    println!("  {}", style("Rex Mono Init").bold().cyan());
    println!("  {}", style("\u{2500}".repeat(40)).dim());
    println!();

    let cwd = std::env::current_dir()?;
    let repo_dir = cwd.join(name);

    if repo_dir.exists() {
        return Err(RexError::AlreadyExists(format!(
            "Directory \"{name}\" already exists."
        )));
    }

    // 1. Create the monorepo directory
    fs::create_dir_all(&repo_dir)?;
    println!(
        "  {} Created directory: {name}/",
        style("\u{2713}").green().bold(),
    );

    // 2. Create workspace Cargo.toml
    fs::write(
        repo_dir.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"libs/*\"]\n",
    )?;
    println!(
        "  {} Created workspace Cargo.toml",
        style("\u{2713}").green().bold(),
    );

    // 3. Create libs/ directory with .gitkeep
    let libs_dir = repo_dir.join("libs");
    fs::create_dir_all(&libs_dir)?;
    fs::write(libs_dir.join(".gitkeep"), "")?;
    println!(
        "  {} Created libs/",
        style("\u{2713}").green().bold(),
    );

    // 4. Create .gitignore
    fs::write(repo_dir.join(".gitignore"), "/target\n.env\n.env.*\n")?;
    println!(
        "  {} Created .gitignore",
        style("\u{2713}").green().bold(),
    );

    // 5. git init
    let git_output = Command::new("git")
        .arg("init")
        .current_dir(&repo_dir)
        .output()?;
    if !git_output.status.success() {
        let stderr = String::from_utf8_lossy(&git_output.stderr);
        return Err(RexError::Subprocess {
            command: "git init".into(),
            detail: stderr.to_string(),
        });
    }
    println!(
        "  {} Initialized git repository",
        style("\u{2713}").green().bold(),
    );

    // 6. Run rex init inside the new directory
    let exe = std::env::current_exe()?;

    let rex_status = Command::new(&exe)
        .arg("init")
        .current_dir(&repo_dir)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !rex_status.success() {
        return Err(RexError::Subprocess {
            command: "rex init".into(),
            detail: "non-zero exit".into(),
        });
    }

    // 7. Summary
    println!(
        "  {} Monorepo \"{}\" is ready.",
        style("\u{2713}").green().bold(),
        name
    );
    println!(
        "  {} cd {name} && rex project create",
        style("\u{2192}").cyan(),
    );
    println!();

    Ok(())
}
