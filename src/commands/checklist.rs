use crate::errors::{RexError, RexResult};
use crate::models::checklist::{Checklist, ChecklistCategory, ChecklistItem, Phase};
use crate::models::project::ProjectRegistry;
use console::style;
use std::path::PathBuf;
use std::process::Command;

fn checklist_path() -> RexResult<(String, PathBuf)> {
    let registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .ok_or(RexError::NotFound("No active project.".into()))?;
    let path = PathBuf::from(format!("rex/{}/onboarding/checklist.json", project.id));
    Ok((project.id, path))
}

fn load_checklist() -> RexResult<(String, PathBuf, Checklist)> {
    let (project_id, path) = checklist_path()?;
    if !path.exists() {
        return Err(RexError::NotFound(format!(
            "No checklist.json found for project \"{project_id}\". Run `rex checklist init` first."
        )));
    }
    let checklist = Checklist::load(&path)?;
    Ok((project_id, path, checklist))
}

fn print_field(label: &str, value: impl std::fmt::Display) {
    println!("  {:<16} {value}", style(format!("{label}:")).dim());
}

pub fn init(date: Option<String>) -> RexResult<()> {
    let (project_id, path) = checklist_path()?;
    if path.exists() {
        return Err(RexError::AlreadyExists(format!(
            "checklist.json already exists for project \"{project_id}\"."
        )));
    }

    let date = match date {
        Some(d) => d,
        None => {
            let output = Command::new("date").arg("+%Y-%m-%d").output()?;
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
    };

    let checklist = Checklist::new(&date);
    checklist.save(&path)?;

    println!(
        "\n  {} Initialized checklist.json for project \"{}\".\n",
        style("\u{2713}").green().bold(),
        project_id
    );
    Ok(())
}

pub fn add(
    category: ChecklistCategory,
    id: &str,
    title: &str,
    description: &str,
    phase: Option<Phase>,
) -> RexResult<()> {
    let (project_id, path, mut checklist) = load_checklist()?;

    if checklist.has_id(id) {
        return Err(RexError::AlreadyExists(format!(
            "Item with ID \"{id}\" already exists in the checklist."
        )));
    }

    if category == ChecklistCategory::OutOfScope && phase.is_some() {
        return Err(RexError::Validation(
            "Out-of-scope items should not have a phase.".into(),
        ));
    }
    if category != ChecklistCategory::OutOfScope && phase.is_none() {
        return Err(RexError::Validation(
            "--phase (design or planning) is required for non out-of-scope items.".into(),
        ));
    }

    let item = ChecklistItem {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        complete: if category == ChecklistCategory::OutOfScope {
            None
        } else {
            Some(false)
        },
        phase,
    };

    checklist.items_mut(category).push(item);
    checklist.save(&path)?;

    println!(
        "\n  {} Added \"{}\" to {} in project \"{}\".\n",
        style("\u{2713}").green().bold(),
        id,
        style(category.label()).cyan(),
        project_id
    );
    Ok(())
}

pub fn list(
    category_filter: Option<ChecklistCategory>,
    phase_filter: Option<Phase>,
    show_complete: bool,
    show_incomplete: bool,
) -> RexResult<()> {
    let (project_id, _, checklist) = load_checklist()?;

    println!();
    println!(
        "  {} \u{2014} \"{}\"",
        style("Checklist").bold().cyan(),
        project_id
    );
    println!("  {}", style("\u{2500}".repeat(40)).dim());

    let categories = match category_filter {
        Some(cat) => vec![cat],
        None => ChecklistCategory::ALL.to_vec(),
    };

    let mut total = 0;
    for cat in &categories {
        let items: Vec<&ChecklistItem> = checklist
            .items(*cat)
            .iter()
            .filter(|item| {
                if let Some(phase) = phase_filter {
                    if item.phase != Some(phase) {
                        return false;
                    }
                }
                if show_complete && item.complete != Some(true) {
                    return false;
                }
                if show_incomplete && item.complete == Some(true) {
                    return false;
                }
                true
            })
            .collect();

        if items.is_empty() {
            continue;
        }

        total += items.len();
        println!();
        println!("  {} ({})", style(cat.label()).bold(), items.len());

        for item in &items {
            let check = if item.complete == Some(true) {
                style("\u{2713}").green().to_string()
            } else if item.complete.is_some() {
                style("\u{25CB}").dim().to_string()
            } else {
                style("\u{2013}").dim().to_string()
            };
            let phase_str = item
                .phase
                .map(|p| style(p).cyan().to_string())
                .unwrap_or_default();
            println!(
                "    {}  {:<28} {}  {}",
                check,
                style(&item.id).dim(),
                item.title,
                phase_str
            );
        }
    }

    if total == 0 {
        println!(
            "\n  {} No items match the given filters.\n",
            style("\u{2139}").blue().bold()
        );
    } else {
        println!("\n  {} total items\n", style(total).dim());
    }

    Ok(())
}

pub fn get(id: &str) -> RexResult<()> {
    let (_project_id, _, checklist) = load_checklist()?;

    let cat = checklist
        .find_category(id)
        .ok_or_else(|| RexError::NotFound(format!("Item \"{id}\" not found in checklist.")))?;

    let item = checklist
        .items(cat)
        .iter()
        .find(|i| i.id == id)
        .expect("verified by prior check");

    println!();
    print_field("ID", &item.id);
    print_field("Title", &item.title);
    print_field("Description", &item.description);
    print_field("Category", cat.label());
    if let Some(complete) = item.complete {
        print_field("Complete", complete);
    }
    if let Some(phase) = item.phase {
        print_field("Phase", phase);
    }
    println!();

    Ok(())
}

pub fn update(
    id: &str,
    title: Option<String>,
    description: Option<String>,
    phase: Option<Phase>,
) -> RexResult<()> {
    if title.is_none() && description.is_none() && phase.is_none() {
        return Err(RexError::Validation(
            "At least one of --title, --description, or --phase must be provided.".into(),
        ));
    }

    let (project_id, path, mut checklist) = load_checklist()?;

    let cat = checklist
        .find_category(id)
        .ok_or_else(|| RexError::NotFound(format!("Item \"{id}\" not found in checklist.")))?;

    if phase.is_some() && cat == ChecklistCategory::OutOfScope {
        return Err(RexError::Validation(
            "Cannot set phase on out-of-scope items.".into(),
        ));
    }

    let item = checklist
        .items_mut(cat)
        .iter_mut()
        .find(|i| i.id == id)
        .expect("verified by prior check");

    let mut changes = Vec::new();

    if let Some(ref new_title) = title {
        changes.push(format!("title: \"{}\" \u{2192} \"{}\"", item.title, new_title));
        item.title = new_title.clone();
    }
    if let Some(ref new_desc) = description {
        changes.push("description updated".to_string());
        item.description = new_desc.clone();
    }
    if let Some(new_phase) = phase {
        let old = item
            .phase
            .map(|p| p.to_string())
            .unwrap_or_else(|| "none".to_string());
        changes.push(format!("phase: {old} \u{2192} {new_phase}"));
        item.phase = Some(new_phase);
    }

    checklist.save(&path)?;

    println!(
        "\n  {} Updated \"{}\" in project \"{}\".",
        style("\u{2713}").green().bold(),
        id,
        project_id
    );
    for change in &changes {
        println!("  {:<16} {}", "", style(change).dim());
    }
    println!();

    Ok(())
}

pub fn complete(id: &str) -> RexResult<()> {
    let (project_id, path, mut checklist) = load_checklist()?;

    let cat = checklist
        .find_category(id)
        .ok_or_else(|| RexError::NotFound(format!("Item \"{id}\" not found in checklist.")))?;

    if cat == ChecklistCategory::OutOfScope {
        return Err(RexError::Validation(
            "Cannot mark out-of-scope items as complete.".into(),
        ));
    }

    let item = checklist
        .items_mut(cat)
        .iter_mut()
        .find(|i| i.id == id)
        .expect("verified by prior check");

    item.complete = Some(true);
    checklist.save(&path)?;

    println!(
        "\n  {} Marked \"{}\" as complete in project \"{}\".\n",
        style("\u{2713}").green().bold(),
        id,
        project_id
    );
    Ok(())
}

pub fn uncomplete(id: &str) -> RexResult<()> {
    let (project_id, path, mut checklist) = load_checklist()?;

    let cat = checklist
        .find_category(id)
        .ok_or_else(|| RexError::NotFound(format!("Item \"{id}\" not found in checklist.")))?;

    if cat == ChecklistCategory::OutOfScope {
        return Err(RexError::Validation(
            "Cannot toggle completion on out-of-scope items.".into(),
        ));
    }

    let item = checklist
        .items_mut(cat)
        .iter_mut()
        .find(|i| i.id == id)
        .expect("verified by prior check");

    item.complete = Some(false);
    checklist.save(&path)?;

    println!(
        "\n  {} Marked \"{}\" as incomplete in project \"{}\".\n",
        style("\u{2713}").green().bold(),
        id,
        project_id
    );
    Ok(())
}

pub fn remove(id: &str) -> RexResult<()> {
    let (project_id, path, mut checklist) = load_checklist()?;

    let cat = checklist
        .find_category(id)
        .ok_or_else(|| RexError::NotFound(format!("Item \"{id}\" not found in checklist.")))?;

    checklist.items_mut(cat).retain(|i| i.id != id);
    checklist.save(&path)?;

    println!(
        "\n  {} Removed \"{}\" from {} in project \"{}\".\n",
        style("\u{2713}").green().bold(),
        id,
        style(cat.label()).cyan(),
        project_id
    );
    Ok(())
}

pub fn set_context(context: &str) -> RexResult<()> {
    let (project_id, path, mut checklist) = load_checklist()?;

    checklist.project_checklist.context = context.to_string();
    checklist.save(&path)?;

    println!(
        "\n  {} Updated context for checklist in project \"{}\".\n",
        style("\u{2713}").green().bold(),
        project_id
    );
    Ok(())
}
