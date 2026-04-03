use crate::errors::{RexError, RexResult};
use crate::models::planning::{PlanningStatus, PlanningStore};
use crate::models::project::{Category, Complexity, Project, ProjectRegistry};
use crate::models::project_status::{ProjectStatus, Status};
use crate::ui::design_select::design_select;
use crate::ui::tab_select::tab_select;
use crate::ui::text_input::text_input;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;

fn print_field(label: &str, value: impl fmt::Display) {
    println!("  {:<16} {value}", style(format!("{label}:")).dim());
}

fn print_project(project: &Project) {
    print_field("ID", &project.id);
    print_field("Category", &project.category);
    print_field("Complexity", &project.complexity);
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
) -> RexResult<String> {
    let cwd = std::env::current_dir()?;
    let libs_dir = cwd.join("libs");
    let candidate = if libs_dir.is_dir() {
        libs_dir.join(id)
    } else {
        cwd.join(id)
    };

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

    let cwd_for_validator = cwd.clone();
    let default_dir = if libs_dir.is_dir() {
        format!("libs/{id}")
    } else {
        id.to_string()
    };
    let directory = text_input(
        "  Directory \u{203a}",
        &default_dir,
        Some(&move |input: &str| {
            let path = Path::new(input);
            let resolved = if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd_for_validator.join(path)
            };

            // If the directory already exists, it's valid
            if resolved.is_dir() {
                return None;
            }

            // Check that the parent directory exists
            match resolved.parent() {
                Some(parent) if parent.is_dir() => None,
                _ => Some(format!(
                    "Parent directory does not exist: {}",
                    resolved.parent().map(|p| p.display().to_string()).unwrap_or_default()
                )),
            }
        }),
    )?;

    // Resolve relative paths to absolute so cargo new always gets a valid path
    let path = Path::new(&directory);
    if path.is_absolute() {
        Ok(directory)
    } else {
        Ok(cwd.join(path).display().to_string())
    }
}

pub fn create() -> RexResult<()> {
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
    let registry = ProjectRegistry::load()?;
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
        .with_prompt("  Title (Enter to complete later)")
        .allow_empty(true)
        .interact_text()?;
    let title = if title.is_empty() { "Complete later".into() } else { title };

    // --- Subtitle ---
    let subtitle: String = Input::with_theme(&theme)
        .with_prompt("  Subtitle (Enter to complete later)")
        .allow_empty(true)
        .interact_text()?;
    let subtitle = if subtitle.is_empty() { "Complete later".into() } else { subtitle };

    // --- Description ---
    let description: String = Input::with_theme(&theme)
        .with_prompt("  Description (Enter to complete later)")
        .allow_empty(true)
        .interact_text()?;
    let description = if description.is_empty() { "Complete later".into() } else { description };

    // --- Directory ---
    let directory = resolve_directory(&theme, &id)?;

    // --- User Name (optional) ---
    let user_name_input: String = Input::with_theme(&theme)
        .with_prompt("  User Name (optional, press Enter to skip)")
        .allow_empty(true)
        .interact_text()?;
    let user_name = Some(user_name_input).filter(|s| !s.is_empty());

    // --- Category, Onboarding & Design (interactive widgets) with go-back support ---
    let (tab_result, design_result) = loop {
        let result = tab_select(&complexity)?;
        let design = design_select(&complexity, &result.category)?;

        // --- Summary ---
        println!();
        println!("  {}", style("Summary").bold().underlined());
        println!();

        let preview = Project {
            id: id.clone(),
            category: result.category.clone(),
            complexity: complexity.clone(),
            title: title.clone(),
            subtitle: subtitle.clone(),
            description: description.clone(),
            directory: directory.clone(),
            user_name: user_name.clone(),
            locked: false,
        };
        print_project(&preview);
        println!();

        // --- Confirm ---
        let action = Select::with_theme(&theme)
            .with_prompt("  Confirm")
            .items(&["Create project", "Go back", "Cancel"])
            .default(0)
            .interact()?;

        match action {
            0 => break (result, design),
            1 => continue,
            _ => {
                println!("\n  {}", style("Cancelled.").yellow());
                return Ok(());
            }
        }
    };

    let project = Project {
        id: id.clone(),
        category: tab_result.category,
        complexity,
        title,
        subtitle,
        description,
        directory,
        user_name,
        locked: false,
    };

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
            return Err(RexError::Subprocess {
                command: "cargo new".into(),
                detail: stderr.to_string(),
            });
        }
        println!(
            "  {} Scaffolded new Rust project at {}",
            style("\u{2713}").green().bold(),
            &project.directory
        );
    }

    // --- Initialize rex inside project? ---
    let has_outer_harness = Path::new("rex/projects.json").exists();
    let init_inside = Confirm::with_theme(&theme)
        .with_prompt("  Initialize rex harness inside the project directory?")
        .default(!has_outer_harness)
        .interact()?;

    if init_inside {
        let exe = std::env::current_exe()?;
        let rex_status = Command::new(&exe)
            .arg("init")
            .current_dir(&project.directory)
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
    }

    // --- Persist ---
    let category = project.category.clone();
    let project_directory = project.directory.clone();

    if init_inside {
        let base = Path::new(&project_directory);

        // Fresh registry for the inner harness
        let mut inner_registry = ProjectRegistry::default();
        inner_registry.set_active(project);

        // Create <base>/rex/{id}/ subdirectories and project-status.json
        let project_meta_dir = base.join(format!("rex/{id}"));
        for sub in &["onboarding", "user-support", "planning", "design", "execution", "uat"] {
            fs::create_dir_all(project_meta_dir.join(sub))?;
        }

        let status = ProjectStatus::new(&id, &tab_result.selected_items, &design_result.selected_items, &category);
        status.save(&project_meta_dir)?;

        // Write registry directly to <base>/rex/projects.json
        let registry_path = base.join("rex/projects.json");
        let json = serde_json::to_string_pretty(&inner_registry)
            .map_err(|e| RexError::JsonSerialize { context: "projects.json".into(), source: e })?;
        fs::write(&registry_path, format!("{json}\n"))
            .map_err(|e| RexError::FileWrite { path: registry_path.display().to_string(), source: e })?;

        // --- Success output ---
        println!();
        println!(
            "  {} Project \"{}\" created with rex harness inside {}",
            style("\u{2713}").green().bold(),
            id,
            &project_directory
        );
        println!(
            "  {} Created project metadata: rex/{id}/",
            style("\u{2713}").green().bold(),
        );
        println!(
            "  {} cd {} && rex project next-item",
            style("\u{2192}").cyan(),
            &project_directory
        );
        println!();
    } else {
        let mut registry = ProjectRegistry::load()?;
        let prev_active_id = registry.active.as_ref().map(|p| p.id.clone());
        registry.set_active(project);

        // Create rex/<project-id>/ directory, subdirectories, and project-status.json
        let project_dir = format!("rex/{id}");
        for sub in &["onboarding", "user-support", "planning", "design", "execution", "uat"] {
            fs::create_dir_all(format!("{project_dir}/{sub}"))?;
        }

        let status = ProjectStatus::new(&id, &tab_result.selected_items, &design_result.selected_items, &category);
        status.save(Path::new(&project_dir))?;

        registry.save()?;

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
    }

    Ok(())
}

pub fn update_status(item: &str, status: Status) -> RexResult<()> {
    let registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;

    let project_dir = format!("rex/{}", project.id);
    let mut project_status = ProjectStatus::load(Path::new(&project_dir))?;

    let step = project_status
        .onboarding
        .iter_mut()
        .chain(&mut project_status.user_support)
        .chain(&mut project_status.design)
        .chain(&mut project_status.planning)
        .chain(&mut project_status.execution)
        .find(|s| s.item == item)
        .ok_or_else(|| RexError::NotFound(format!("Item \"{item}\" not found in project status.")))?;

    step.status = status;
    project_status.save(Path::new(&project_dir))?;

    println!(
        "\n  {} Updated \"{}\" to {} in project \"{}\".\n",
        style("\u{2713}").green().bold(),
        item,
        style(status).cyan(),
        project.id
    );

    Ok(())
}

pub fn remove(id: &str) -> RexResult<()> {
    let theme = ColorfulTheme::default();
    let mut registry = ProjectRegistry::load()?;

    let project = registry
        .remove_project(id)
        .ok_or_else(|| RexError::NotFound(format!("Project \"{id}\" not found.")))?;

    // Remove rex/{id}/ project metadata directory
    let rex_project_dir = format!("rex/{id}");
    if Path::new(&rex_project_dir).is_dir() {
        fs::remove_dir_all(&rex_project_dir)?;
        println!(
            "\n  {} Removed {rex_project_dir}/",
            style("\u{2713}").green().bold()
        );
    }

    // Ask about removing the project source directory
    println!();
    println!(
        "  {} Do you also want the project source directory removed?",
        style("WARNING").yellow().bold()
    );
    let choice = Select::with_theme(&theme)
        .items(&[
            style("No").green().to_string(),
            style("Yes").yellow().to_string(),
        ])
        .default(0)
        .interact()?;

    if choice == 1 {
        println!();
        println!(
            "  {} This will delete the entire project code in directory {}.",
            style("WARNING").red().bold(),
            style(&project.directory).bold()
        );
        let certain = Confirm::with_theme(&theme)
            .with_prompt("  Are you certain?")
            .default(false)
            .interact()?;

        if certain {
            if Path::new(&project.directory).is_dir() {
                fs::remove_dir_all(&project.directory)?;
                println!(
                    "  {} Removed {}",
                    style("\u{2713}").green().bold(),
                    &project.directory
                );
            } else {
                println!(
                    "  {} Directory {} does not exist.",
                    style("\u{2139}").blue().bold(),
                    &project.directory
                );
            }
        }
    }

    registry.save()?;

    println!(
        "\n  {} Project \"{}\" has been removed.\n",
        style("\u{2713}").green().bold(),
        id
    );

    Ok(())
}

pub fn activate(id: &str) -> RexResult<()> {
    let mut registry = ProjectRegistry::load()?;

    let prev_active_id = registry.active.as_ref().map(|p| p.id.clone());
    registry.activate_project(id)?;
    registry.save()?;

    println!(
        "\n  {} Project \"{}\" is now the active project.",
        style("\u{2713}").green().bold(),
        id
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

pub fn update_directory(directory: &str) -> RexResult<()> {
    let mut registry = ProjectRegistry::load()?;

    let project = registry
        .active
        .as_mut()
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;
    let id = project.id.clone();
    let old_directory = project.directory.clone();
    project.directory = directory.to_owned();
    registry.save()?;

    println!(
        "\n  {} Updated directory for project \"{}\".",
        style("\u{2713}").green().bold(),
        id
    );
    println!(
        "  {:<16} {}",
        style("From:").dim(),
        old_directory
    );
    println!(
        "  {:<16} {}\n",
        style("To:").dim(),
        directory
    );

    Ok(())
}

pub fn update_title(title: &str) -> RexResult<()> {
    let mut registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .as_mut()
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;
    let id = project.id.clone();
    let old = project.title.clone();
    project.title = title.to_owned();
    registry.save()?;

    println!(
        "\n  {} Updated title for project \"{}\".",
        style("\u{2713}").green().bold(),
        id
    );
    println!("  {:<16} {}", style("From:").dim(), old);
    println!("  {:<16} {}\n", style("To:").dim(), title);
    Ok(())
}

pub fn update_subtitle(subtitle: &str) -> RexResult<()> {
    let mut registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .as_mut()
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;
    let id = project.id.clone();
    let old = project.subtitle.clone();
    project.subtitle = subtitle.to_owned();
    registry.save()?;

    println!(
        "\n  {} Updated subtitle for project \"{}\".",
        style("\u{2713}").green().bold(),
        id
    );
    println!("  {:<16} {}", style("From:").dim(), old);
    println!("  {:<16} {}\n", style("To:").dim(), subtitle);
    Ok(())
}

pub fn update_category(category: Category) -> RexResult<()> {
    let mut registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .as_mut()
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;
    let id = project.id.clone();
    let old = project.category.clone();
    project.category = category.clone();
    registry.save()?;

    println!(
        "\n  {} Updated category for project \"{}\".",
        style("\u{2713}").green().bold(),
        id
    );
    println!("  {:<16} {}", style("From:").dim(), old);
    println!("  {:<16} {}\n", style("To:").dim(), category);
    Ok(())
}

pub fn update_complexity(complexity: Complexity) -> RexResult<()> {
    let mut registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .as_mut()
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;
    let id = project.id.clone();
    let old = project.complexity.clone();
    project.complexity = complexity.clone();
    registry.save()?;

    println!(
        "\n  {} Updated complexity for project \"{}\".",
        style("\u{2713}").green().bold(),
        id
    );
    println!("  {:<16} {}", style("From:").dim(), old);
    println!("  {:<16} {}\n", style("To:").dim(), complexity);
    Ok(())
}

pub fn update_description(description: &str) -> RexResult<()> {
    let mut registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .as_mut()
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;
    let id = project.id.clone();
    let old = project.description.clone();
    project.description = description.to_owned();
    registry.save()?;

    println!(
        "\n  {} Updated description for project \"{}\".",
        style("\u{2713}").green().bold(),
        id
    );
    println!("  {:<16} {}", style("From:").dim(), old);
    println!("  {:<16} {}\n", style("To:").dim(), description);
    Ok(())
}

pub fn next_item() -> RexResult<()> {
    let registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;

    let project_dir = format!("rex/{}", project.id);
    let path = Path::new(&project_dir).join("project-status.json");
    let contents = fs::read_to_string(&path)
        .map_err(|e| RexError::FileRead { path: path.display().to_string(), source: e })?;
    let raw: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| RexError::JsonParse { context: "project-status.json".into(), source: e })?;

    let tasks = flatten_tasks(&raw)?;

    let next = tasks.iter().find(|task| {
        match task.get("status").and_then(|s| s.as_str()) {
            Some("completed") | Some("not-required") => false,
            _ => true,
        }
    });

    match next {
        Some(task) => {
            println!("{}", serde_json::to_string_pretty(task)?);
        }
        None => {
            println!(
                "\n  {} All items are completed or not required in project \"{}\".\n",
                style("\u{2139}").blue().bold(),
                project.id
            );
        }
    }

    Ok(())
}

/// Flattens project-status.json into an ordered list of tasks with a `phase` field.
///
/// Supports two formats:
/// - **Grouped (current):** an object with phase keys (`user_support`, `onboarding`, `design`)
///   each containing an array of task objects. A `"phase"` field is injected into each task.
/// - **Flat (future):** a top-level array of task objects that already contain a `"phase"` field.
fn flatten_tasks(
    raw: &serde_json::Value,
) -> RexResult<Vec<serde_json::Value>> {
    let mut tasks = Vec::new();

    if let Some(obj) = raw.as_object() {
        // Current format: object with phase keys containing arrays of tasks.
        // Process known phases in workflow order, then any remaining keys.
        let known_phases = ["user_support", "onboarding", "design", "planning", "execution"];
        for phase_key in &known_phases {
            if let Some(items) = obj.get(*phase_key).and_then(|v| v.as_array()) {
                let phase_name = phase_key.replace('_', "-");
                for item in items {
                    let mut task = item.clone();
                    if let Some(task_obj) = task.as_object_mut() {
                        task_obj.insert(
                            "phase".to_string(),
                            serde_json::Value::String(phase_name.clone()),
                        );
                    }
                    tasks.push(task);
                }
            }
        }
        for (key, value) in obj {
            if !known_phases.contains(&key.as_str()) {
                if let Some(items) = value.as_array() {
                    let phase_name = key.replace('_', "-");
                    for item in items {
                        let mut task = item.clone();
                        if let Some(task_obj) = task.as_object_mut() {
                            task_obj.insert(
                                "phase".to_string(),
                                serde_json::Value::String(phase_name.clone()),
                            );
                        }
                        tasks.push(task);
                    }
                }
            }
        }
    } else if let Some(arr) = raw.as_array() {
        // Future flat format: array of tasks with phase field already present.
        tasks = arr.clone();
    } else {
        return Err(RexError::Validation(
            "project-status.json has unexpected format.".into(),
        ));
    }

    Ok(tasks)
}

pub fn lock() -> RexResult<()> {
    let mut registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .as_mut()
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;

    if project.locked {
        println!(
            "\n  {} Project \"{}\" is already locked.\n",
            style("\u{2139}").blue().bold(),
            project.id
        );
        return Ok(());
    }

    project.locked = true;
    let id = project.id.clone();
    registry.save()?;

    println!(
        "\n  {} Project \"{}\" is now locked.\n",
        style("\u{2713}").green().bold(),
        id
    );
    Ok(())
}

pub fn unlock() -> RexResult<()> {
    let mut registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .as_mut()
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;

    if !project.locked {
        println!(
            "\n  {} Project \"{}\" is already unlocked.\n",
            style("\u{2139}").blue().bold(),
            project.id
        );
        return Ok(());
    }

    project.locked = false;
    let id = project.id.clone();
    registry.save()?;

    println!(
        "\n  {} Project \"{}\" is now unlocked.\n",
        style("\u{2713}").green().bold(),
        id
    );
    Ok(())
}

pub fn get_active() -> RexResult<()> {
    let registry = ProjectRegistry::load()?;

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

pub fn get_completion_percent() -> RexResult<()> {
    let registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;

    let project_dir = format!("rex/{}", project.id);
    let project_status = ProjectStatus::load(Path::new(&project_dir))?;

    // Gather all project-status items across every phase.
    let all_items: Vec<&Status> = project_status
        .user_support
        .iter()
        .chain(&project_status.onboarding)
        .chain(&project_status.design)
        .chain(&project_status.planning)
        .chain(&project_status.execution)
        .map(|s| &s.status)
        .collect();

    let project_complete = all_items.iter().all(|s| {
        matches!(s, Status::Completed | Status::NotRequired)
    });

    let (items_percent, tasks_percent) = if project_complete {
        (100.0_f64, 100.0_f64)
    } else {
        // project-items-percent: completed / actionable (excluding not-required)
        let actionable = all_items
            .iter()
            .filter(|s| !matches!(s, Status::NotRequired))
            .count();
        let completed = all_items
            .iter()
            .filter(|s| matches!(s, Status::Completed))
            .count();
        let items_pct = if actionable == 0 {
            100.0
        } else {
            (completed as f64 / actionable as f64) * 100.0
        };

        // project-tasks-percent: completed / total from planning.json
        let store = PlanningStore::load(Path::new(&project_dir))?;
        let total_tasks = store.tasks.len();
        let completed_tasks = store
            .tasks
            .iter()
            .filter(|t| matches!(t.status, PlanningStatus::Completed))
            .count();
        let tasks_pct = if total_tasks == 0 {
            0.0
        } else {
            (completed_tasks as f64 / total_tasks as f64) * 100.0
        };

        (items_pct, tasks_pct)
    };

    let output = serde_json::json!({
        "project-items-percent": (items_percent * 100.0).round() / 100.0,
        "project-tasks-percent": (tasks_percent * 100.0).round() / 100.0
    });
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
