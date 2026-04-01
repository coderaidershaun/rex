use console::style;
use std::fs;
use std::process::Command;

use super::init::AgentOs;

pub fn init(name: &str, agent_os: Option<AgentOs>) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("  {}", style("Rex Mono Init").bold().cyan());
    println!("  {}", style("\u{2500}".repeat(40)).dim());
    println!();

    let cwd = std::env::current_dir()?;
    let repo_dir = cwd.join(name);

    if repo_dir.exists() {
        return Err(format!("Directory \"{name}\" already exists.").into());
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
        return Err(format!("git init failed: {stderr}").into());
    }
    println!(
        "  {} Initialized git repository",
        style("\u{2713}").green().bold(),
    );

    // 6. Run rex init inside the new directory
    let exe = std::env::current_exe()?;
    let mut rex_args = vec!["init".to_string()];
    match &agent_os {
        Some(AgentOs::Claude) => rex_args.push("--claude".into()),
        Some(AgentOs::Cursor) => rex_args.push("--cursor".into()),
        None => {}
    }

    let rex_status = Command::new(&exe)
        .args(&rex_args)
        .current_dir(&repo_dir)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !rex_status.success() {
        return Err("rex init failed.".into());
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
