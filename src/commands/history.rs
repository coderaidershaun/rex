use crate::errors::{RexError, RexResult};
use crate::models::history::{History, HistoryEntry};
use crate::models::project::ProjectRegistry;
use console::style;
use std::path::Path;

fn load_history() -> RexResult<(String, History)> {
    let registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;
    let project_dir = format!("rex/{}", project.id);
    let history = History::load(Path::new(&project_dir))?;
    Ok((project_dir, history))
}

fn build_entry(
    id: &str,
    timestamp: &str,
    summary: &str,
    entities: Vec<String>,
    files: Vec<String>,
    session: Option<String>,
) -> HistoryEntry {
    HistoryEntry {
        id: id.to_string(),
        timestamp: timestamp.to_string(),
        summary: summary.to_string(),
        entities,
        files,
        session,
    }
}

pub fn insert(
    id: &str,
    timestamp: &str,
    summary: &str,
    entities: Vec<String>,
    files: Vec<String>,
    session: Option<String>,
    archived: bool,
) -> RexResult<()> {
    let (project_dir, mut history) = load_history()?;
    let section_name = if archived { "archived" } else { "recent" };
    let section = if archived {
        &history.archived
    } else {
        &history.recent
    };

    if section.iter().any(|e| e.id == id) {
        return Err(RexError::AlreadyExists(format!(
            "{} entry \"{id}\" already exists.",
            if archived { "Archived" } else { "Recent" }
        )));
    }

    let entry = build_entry(id, timestamp, summary, entities, files, session);
    if archived {
        history.archived.push(entry.clone());
    } else {
        history.recent.push(entry.clone());
    }
    history.save(Path::new(&project_dir))?;

    eprintln!(
        "\n  {} Inserted {section_name} entry \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{}", serde_json::to_string_pretty(&entry)?);
    Ok(())
}

pub fn remove(id: &str, archived: bool) -> RexResult<()> {
    let (project_dir, mut history) = load_history()?;
    let section_name = if archived { "archived" } else { "recent" };
    let section = if archived {
        &mut history.archived
    } else {
        &mut history.recent
    };

    let before = section.len();
    section.retain(|e| e.id != id);

    if section.len() == before {
        return Err(RexError::NotFound(format!(
            "{} entry \"{id}\" not found.",
            if archived { "Archived" } else { "Recent" }
        )));
    }

    history.save(Path::new(&project_dir))?;

    eprintln!(
        "\n  {} Removed {section_name} entry \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{{\"removed\": \"{id}\"}}");
    Ok(())
}

pub fn get_recent() -> RexResult<()> {
    let (_project_dir, history) = load_history()?;
    println!("{}", serde_json::to_string_pretty(&history.recent)?);
    Ok(())
}

pub fn list() -> RexResult<()> {
    let (_project_dir, history) = load_history()?;
    println!("{}", serde_json::to_string_pretty(&history)?);
    Ok(())
}
