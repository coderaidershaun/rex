use crate::models::project::{Category, Project, ProjectRegistry};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use inquire::Text;
use std::fs;

fn print_field(label: &str, value: &str) {
    println!("  {:<16} {}", style(format!("{label}:")).dim(), value);
}

fn print_project(project: &Project) {
    print_field("ID", &project.id);
    print_field("Category", &project.category.to_string());
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
            style("→").cyan().bold(),
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

    let dir: String = Input::with_theme(theme)
        .with_prompt("  Directory")
        .interact_text()?;

    Ok(dir)
}

pub fn create() -> Result<(), Box<dyn std::error::Error>> {
    let theme = ColorfulTheme::default();

    println!();
    println!("  {}", style("Create New Project").bold().cyan());
    println!("  {}", style("─".repeat(40)).dim());
    println!();

    // --- Project ID ---
    let id = Text::new("  Project ID ›")
        .with_placeholder("some-brief-name")
        .with_validator(inquire::required!("Project ID cannot be empty"))
        .prompt()?;

    // Check for duplicate
    let registry = ProjectRegistry::load().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    if registry.has_project(&id) {
        println!(
            "\n  {} A project with ID \"{}\" already exists.",
            style("✗").red().bold(),
            id
        );
        return Ok(());
    }

    // --- Category ---
    let cat_idx = Select::with_theme(&theme)
        .with_prompt("  Category")
        .items(&Category::ALL)
        .default(0)
        .interact()?;
    let category = Category::from_index(cat_idx);

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
    println!();
    let user_name_input: String = Input::with_theme(&theme)
        .with_prompt("  User Name (optional, press Enter to skip)")
        .allow_empty(true)
        .interact_text()?;
    let user_name = if user_name_input.is_empty() {
        None
    } else {
        Some(user_name_input)
    };

    // --- Summary ---
    println!();
    println!("  {}", style("Summary").bold().underlined());
    println!();

    let project = Project {
        id: id.clone(),
        category,
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

    // --- Persist ---
    let mut registry =
        ProjectRegistry::load().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let prev_active_id = registry.active.as_ref().map(|p| p.id.clone());
    registry.set_active(project);
    registry
        .save()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    // Create rex/<project-id>/ directory
    let project_dir = format!("rex/{id}");
    fs::create_dir_all(&project_dir)?;

    // --- Success output ---
    println!();
    println!(
        "  {} Project \"{}\" created and set as active.",
        style("✓").green().bold(),
        id
    );
    println!(
        "  {} Created project directory: {}/",
        style("✓").green().bold(),
        project_dir
    );
    if let Some(prev_id) = prev_active_id {
        println!(
            "  {} Previous active project \"{}\" moved to inactive.",
            style("ℹ").blue().bold(),
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
            println!("  {}", style("─".repeat(40)).dim());
            println!();
            print_project(&project);
            println!();
        }
        None => {
            println!();
            println!(
                "  {} No active project.",
                style("ℹ").blue().bold()
            );
            println!();
        }
    }

    Ok(())
}
