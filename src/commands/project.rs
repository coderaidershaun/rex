use crate::models::project::{Category, Complexity, Project, ProjectRegistry};
use crate::models::project_status::ProjectStatus;
use crate::ui::tab_select::tab_select;
use crate::ui::text_input::text_input;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::fs;
use std::path::Path;
use std::process::Command;

fn print_field(label: &str, value: &str) {
    println!("  {:<16} {}", style(format!("{label}:")).dim(), value);
}

fn print_project(project: &Project) {
    print_field("ID", &project.id);
    print_field("Category", &project.category.to_string());
    print_field("Complexity", &project.complexity.to_string());
    print_field("Title", &project.title);
    print_field("Subtitle", &project.subtitle);
    print_field("Description", &project.description);
    print_field("Directory", &project.directory);
    if let Some(ref name) = project.user_name {
        print_field("User", name);
    }
}

fn resolve_directory(
    theme: &ColorfulTheme,
    id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let candidate = cwd.join(id);

    if candidate.is_dir() {
        let display_path = candidate.display().to_string();
        println!();
        println!(
            "  {} Detected matching directory: {}",
            style("\u{2192}").cyan().bold(),
            style(&display_path).green()
        );
        let use_detected = Confirm::with_theme(theme)
            .with_prompt("  Use this directory?")
            .default(true)
            .interact()?;

        if use_detected {
            return Ok(display_path);
        }
    }

    let dir = text_input("  Directory \u{203a}", id, None)?;

    Ok(dir)
}

pub fn create() -> Result<(), Box<dyn std::error::Error>> {
    let theme = ColorfulTheme::default();

    println!();
    println!("  {}", style("Create New Project").bold().cyan());
    println!("  {}", style("\u{2500}".repeat(40)).dim());
    println!();

    // --- Project ID ---
    let id = text_input(
        "  Project ID \u{203a}",
        "some-brief-name",
        Some(&|input: &str| {
            if input.is_empty() {
                return Some("Project ID cannot be empty".into());
            }
            if !input.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                return Some("Only lowercase letters and hyphens allowed".into());
            }
            None
        }),
    )?;

    // Check for duplicate
    let registry = ProjectRegistry::load().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    if registry.has_project(&id) {
        println!(
            "\n  {} A project with ID \"{}\" already exists.",
            style("\u{2717}").red().bold(),
            id
        );
        return Ok(());
    }

    // --- Complexity ---
    let complexity_idx = Select::with_theme(&theme)
        .with_prompt("  Complexity")
        .items(&Complexity::ALL)
        .default(1)
        .interact()?;
    let complexity = Complexity::from_index(complexity_idx);

    // --- Title ---
    let title: String = Input::with_theme(&theme)
        .with_prompt("  Title")
        .interact_text()?;

    // --- Subtitle ---
    let subtitle: String = Input::with_theme(&theme)
        .with_prompt("  Subtitle")
        .interact_text()?;

    // --- Description ---
    let description: String = Input::with_theme(&theme)
        .with_prompt("  Description")
        .interact_text()?;

    // --- Directory ---
    let directory = resolve_directory(&theme, &id)?;

    // --- User Name (optional) ---
    let user_name_input: String = Input::with_theme(&theme)
        .with_prompt("  User Name (optional, press Enter to skip)")
        .allow_empty(true)
        .interact_text()?;
    let user_name = if user_name_input.is_empty() {
        None
    } else {
        Some(user_name_input)
    };

    // --- Category & Onboarding (tab widget) ---
    let tab_result = tab_select(&complexity)?;

    // --- Summary ---
    println!();
    println!("  {}", style("Summary").bold().underlined());
    println!();

    let project = Project {
        id: id.clone(),
        category: tab_result.category,
        complexity,
        title,
        subtitle,
        description,
        directory,
        user_name,
    };

    print_project(&project);
    println!();

    // --- Confirm ---
    let confirmed = Confirm::with_theme(&theme)
        .with_prompt("  Create this project?")
        .default(true)
        .interact()?;

    if !confirmed {
        println!("\n  {}", style("Cancelled.").yellow());
        return Ok(());
    }

    // Ensure the source directory exists, scaffold with cargo if not
    if !Path::new(&project.directory).is_dir() {
        let cargo_flag = match &project.category {
            Category::Library | Category::Refactor => "--lib",
            Category::Binary => "--bin",
        };
        let output = Command::new("cargo")
            .args(["new", cargo_flag, &project.directory])
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("cargo new failed: {stderr}").into());
        }
        println!(
            "  {} Scaffolded new Rust project at {}",
            style("\u{2713}").green().bold(),
            &project.directory
        );
    }

    // --- Persist ---
    let mut registry =
        ProjectRegistry::load().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let prev_active_id = registry.active.as_ref().map(|p| p.id.clone());
    registry.set_active(project);

    // Create rex/<project-id>/ directory, subdirectories, and project-status.json
    let project_dir = format!("rex/{id}");
    for sub in &["onboarding", "user-support", "planning", "execution", "uat"] {
        fs::create_dir_all(format!("{project_dir}/{sub}"))?;
    }

    let status = ProjectStatus::new(&tab_result.selected_items);
    status
        .save(Path::new(&project_dir))
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    registry
        .save()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    // --- Success output ---
    println!();
    println!(
        "  {} Project \"{}\" created and set as active.",
        style("\u{2713}").green().bold(),
        id
    );
    println!(
        "  {} Created project directory: {}/",
        style("\u{2713}").green().bold(),
        project_dir
    );
    if let Some(prev_id) = prev_active_id {
        println!(
            "  {} Previous active project \"{}\" moved to inactive.",
            style("\u{2139}").blue().bold(),
            prev_id
        );
    }
    println!();

    Ok(())
}

pub fn get_active() -> Result<(), Box<dyn std::error::Error>> {
    let registry =
        ProjectRegistry::load().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    match registry.active {
        Some(project) => {
            println!();
            println!("  {}", style("Active Project").bold().cyan());
            println!("  {}", style("\u{2500}".repeat(40)).dim());
            println!();
            print_project(&project);
            println!();
        }
        None => {
            println!();
            println!(
                "  {} No active project.",
                style("\u{2139}").blue().bold()
            );
            println!();
        }
    }

    Ok(())
}
