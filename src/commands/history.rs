use crate::models::history::{History, HistoryEntry};
use crate::models::project::ProjectRegistry;
use console::style;
use std::path::Path;

fn load_history() -> Result<(String, History), Box<dyn std::error::Error>> {
    let registry = ProjectRegistry::load()?;
    let project = registry.active.ok_or("No active project.")?;
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

pub fn insert_recent(
    id: &str,
    timestamp: &str,
    summary: &str,
    entities: Vec<String>,
    files: Vec<String>,
    session: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (project_dir, mut history) = load_history()?;

    if history.recent.iter().any(|e| e.id == id) {
        return Err(format!("Recent entry \"{id}\" already exists.").into());
    }

    let entry = build_entry(id, timestamp, summary, entities, files, session);
    history.recent.push(entry.clone());
    history.save(Path::new(&project_dir))?;

    eprintln!(
        "\n  {} Inserted recent entry \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{}", serde_json::to_string_pretty(&entry)?);
    Ok(())
}

pub fn remove_from_recent(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (project_dir, mut history) = load_history()?;

    let before = history.recent.len();
    history.recent.retain(|e| e.id != id);

    if history.recent.len() == before {
        return Err(format!("Recent entry \"{id}\" not found.").into());
    }

    history.save(Path::new(&project_dir))?;

    eprintln!(
        "\n  {} Removed recent entry \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{{\"removed\": \"{id}\"}}");
    Ok(())
}

pub fn insert_compacted(
    id: &str,
    timestamp: &str,
    summary: &str,
    entities: Vec<String>,
    files: Vec<String>,
    session: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (project_dir, mut history) = load_history()?;

    if history.archived.iter().any(|e| e.id == id) {
        return Err(format!("Archived entry \"{id}\" already exists.").into());
    }

    let entry = build_entry(id, timestamp, summary, entities, files, session);
    history.archived.push(entry.clone());
    history.save(Path::new(&project_dir))?;

    eprintln!(
        "\n  {} Inserted archived entry \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{}", serde_json::to_string_pretty(&entry)?);
    Ok(())
}

pub fn remove_from_compacted(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (project_dir, mut history) = load_history()?;

    let before = history.archived.len();
    history.archived.retain(|e| e.id != id);

    if history.archived.len() == before {
        return Err(format!("Archived entry \"{id}\" not found.").into());
    }

    history.save(Path::new(&project_dir))?;

    eprintln!(
        "\n  {} Removed archived entry \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{{\"removed\": \"{id}\"}}");
    Ok(())
}

pub fn list() -> Result<(), Box<dyn std::error::Error>> {
    let (_project_dir, history) = load_history()?;
    println!("{}", serde_json::to_string_pretty(&history)?);
    Ok(())
}
