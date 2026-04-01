use crate::models::planning::{
    apply_all_list_mods, ListMods, Milestone, PlanningStatus, PlanningStore,
};
use crate::models::project::ProjectRegistry;
use console::style;
use std::path::Path;

fn load_store() -> Result<(String, PlanningStore), Box<dyn std::error::Error>> {
    let registry = ProjectRegistry::load()?;
    let project = registry.active.ok_or("No active project.")?;
    let project_dir = format!("rex/{}", project.id);
    let store = PlanningStore::load(Path::new(&project_dir))?;
    Ok((project_dir, store))
}

pub fn upsert(
    id: &str,
    title: Option<String>,
    description: Option<String>,
    status: Option<PlanningStatus>,
    mods: ListMods,
) -> Result<(), Box<dyn std::error::Error>> {
    let (project_dir, mut store) = load_store()?;

    let is_new = !store.milestones.iter().any(|m| m.id == id);

    if is_new {
        let title = title.ok_or("--title is required when creating a new milestone.")?;
        let description =
            description.ok_or("--description is required when creating a new milestone.")?;

        let mut milestone = Milestone {
            id: id.to_string(),
            title,
            description,
            status: status.unwrap_or(PlanningStatus::NotStarted),
            references: Vec::new(),
            outputs: Vec::new(),
            checklist: Vec::new(),
            objectives: Vec::new(),
            upstream: Vec::new(),
            downstream: Vec::new(),
        };

        apply_all_list_mods(
            &mut milestone.references,
            &mut milestone.outputs,
            &mut milestone.upstream,
            &mut milestone.downstream,
            &mut milestone.checklist,
            &mods,
        )?;

        store.milestones.push(milestone);
    } else {
        let milestone = store.milestones.iter_mut().find(|m| m.id == id).unwrap();

        if let Some(t) = title {
            milestone.title = t;
        }
        if let Some(d) = description {
            milestone.description = d;
        }
        if let Some(s) = status {
            milestone.status = s;
        }

        apply_all_list_mods(
            &mut milestone.references,
            &mut milestone.outputs,
            &mut milestone.upstream,
            &mut milestone.downstream,
            &mut milestone.checklist,
            &mods,
        )?;
    }

    // Bidirectional upstream/downstream maintenance
    let id_string = id.to_string();
    for up_id in &mods.add_upstream {
        if let Some(up) = store.milestones.iter_mut().find(|m| m.id == *up_id) {
            if !up.downstream.contains(&id_string) {
                up.downstream.push(id_string.clone());
            }
        }
    }
    for up_id in &mods.remove_upstream {
        if let Some(up) = store.milestones.iter_mut().find(|m| m.id == *up_id) {
            up.downstream.retain(|x| x != &id_string);
        }
    }
    for down_id in &mods.add_downstream {
        if let Some(down) = store.milestones.iter_mut().find(|m| m.id == *down_id) {
            if !down.upstream.contains(&id_string) {
                down.upstream.push(id_string.clone());
            }
        }
    }
    for down_id in &mods.remove_downstream {
        if let Some(down) = store.milestones.iter_mut().find(|m| m.id == *down_id) {
            down.upstream.retain(|x| x != &id_string);
        }
    }

    store.save(Path::new(&project_dir))?;

    let milestone = store.milestones.iter().find(|m| m.id == id).unwrap();
    let action = if is_new { "Created" } else { "Updated" };

    eprintln!(
        "\n  {} {action} milestone \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{}", serde_json::to_string_pretty(milestone)?);

    Ok(())
}

pub fn get(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (_project_dir, store) = load_store()?;

    let milestone = store
        .milestones
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("Milestone \"{id}\" not found."))?;

    println!("{}", serde_json::to_string_pretty(milestone)?);
    Ok(())
}

pub fn list(status: Option<PlanningStatus>) -> Result<(), Box<dyn std::error::Error>> {
    let (_project_dir, store) = load_store()?;

    let milestones: Vec<&Milestone> = store
        .milestones
        .iter()
        .filter(|m| status.map_or(true, |s| m.status == s))
        .collect();

    println!("{}", serde_json::to_string_pretty(&milestones)?);
    Ok(())
}

pub fn remove(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (project_dir, mut store) = load_store()?;

    let pos = store
        .milestones
        .iter()
        .position(|m| m.id == id)
        .ok_or_else(|| format!("Milestone \"{id}\" not found."))?;

    store.milestones.remove(pos);

    // Clean up upstream/downstream references in sibling milestones
    for m in &mut store.milestones {
        m.upstream.retain(|x| x != id);
        m.downstream.retain(|x| x != id);
    }

    // Identify orphaned objectives
    let orphaned: Vec<String> = store
        .objectives
        .iter()
        .filter(|o| o.milestone_id == id)
        .map(|o| o.id.clone())
        .collect();

    store.save(Path::new(&project_dir))?;

    eprintln!(
        "\n  {} Removed milestone \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );

    if !orphaned.is_empty() {
        eprintln!(
            "  {} Orphaned objectives: {}\n",
            style("\u{26a0}").yellow().bold(),
            orphaned.join(", ")
        );
    }

    // Output removed id as JSON for machine consumption
    println!("{{\"removed\": \"{id}\"}}");
    Ok(())
}
