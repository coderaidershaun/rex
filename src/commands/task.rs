use crate::models::planning::{
    apply_all_list_mods, ListMods, PlanningStatus, PlanningStore, Task,
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
    objective: Option<String>,
    title: Option<String>,
    description: Option<String>,
    status: Option<PlanningStatus>,
    mods: ListMods,
) -> Result<(), Box<dyn std::error::Error>> {
    let (project_dir, mut store) = load_store()?;

    let is_new = !store.tasks.iter().any(|t| t.id == id);

    if is_new {
        let objective_id =
            objective.ok_or("--objective is required when creating a new task.")?;
        let title = title.ok_or("--title is required when creating a new task.")?;
        let description =
            description.ok_or("--description is required when creating a new task.")?;

        // Validate parent objective exists
        if !store.objectives.iter().any(|o| o.id == objective_id) {
            return Err(format!("Objective \"{objective_id}\" not found.").into());
        }

        let mut task = Task {
            id: id.to_string(),
            objective_id: objective_id.clone(),
            title,
            description,
            status: status.unwrap_or(PlanningStatus::NotStarted),
            references: Vec::new(),
            outputs: Vec::new(),
            checklist: Vec::new(),
            upstream: Vec::new(),
            downstream: Vec::new(),
        };

        apply_all_list_mods(
            &mut task.references,
            &mut task.outputs,
            &mut task.upstream,
            &mut task.downstream,
            &mut task.checklist,
            &mods,
        )?;

        store.tasks.push(task);

        // Register in parent objective's tasks list
        if let Some(o) = store.objectives.iter_mut().find(|o| o.id == objective_id) {
            if !o.tasks.contains(&id.to_string()) {
                o.tasks.push(id.to_string());
            }
        }
    } else {
        let task = store.tasks.iter_mut().find(|t| t.id == id).unwrap();

        // Handle re-parenting
        if let Some(ref new_objective) = objective {
            if *new_objective != task.objective_id {
                if !store.objectives.iter().any(|o| o.id == *new_objective) {
                    return Err(format!("Objective \"{new_objective}\" not found.").into());
                }
                let old_objective = task.objective_id.clone();
                task.objective_id = new_objective.clone();

                // Remove from old parent
                if let Some(o) = store.objectives.iter_mut().find(|o| o.id == old_objective) {
                    o.tasks.retain(|x| x != id);
                }
                // Add to new parent
                if let Some(o) = store.objectives.iter_mut().find(|o| o.id == *new_objective) {
                    if !o.tasks.contains(&id.to_string()) {
                        o.tasks.push(id.to_string());
                    }
                }
            }
        }

        let task = store.tasks.iter_mut().find(|t| t.id == id).unwrap();

        if let Some(t) = title {
            task.title = t;
        }
        if let Some(d) = description {
            task.description = d;
        }
        if let Some(s) = status {
            task.status = s;
        }

        apply_all_list_mods(
            &mut task.references,
            &mut task.outputs,
            &mut task.upstream,
            &mut task.downstream,
            &mut task.checklist,
            &mods,
        )?;
    }

    // Bidirectional upstream/downstream maintenance
    let id_string = id.to_string();
    for up_id in &mods.add_upstream {
        if let Some(up) = store.tasks.iter_mut().find(|t| t.id == *up_id) {
            if !up.downstream.contains(&id_string) {
                up.downstream.push(id_string.clone());
            }
        }
    }
    for up_id in &mods.remove_upstream {
        if let Some(up) = store.tasks.iter_mut().find(|t| t.id == *up_id) {
            up.downstream.retain(|x| x != &id_string);
        }
    }
    for down_id in &mods.add_downstream {
        if let Some(down) = store.tasks.iter_mut().find(|t| t.id == *down_id) {
            if !down.upstream.contains(&id_string) {
                down.upstream.push(id_string.clone());
            }
        }
    }
    for down_id in &mods.remove_downstream {
        if let Some(down) = store.tasks.iter_mut().find(|t| t.id == *down_id) {
            down.upstream.retain(|x| x != &id_string);
        }
    }

    store.save(Path::new(&project_dir))?;

    let task = store.tasks.iter().find(|t| t.id == id).unwrap();
    let action = if is_new { "Created" } else { "Updated" };

    eprintln!(
        "\n  {} {action} task \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{}", serde_json::to_string_pretty(task)?);

    Ok(())
}

pub fn get(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (_project_dir, store) = load_store()?;

    let task = store
        .tasks
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("Task \"{id}\" not found."))?;

    println!("{}", serde_json::to_string_pretty(task)?);
    Ok(())
}

pub fn list(
    objective: Option<String>,
    status: Option<PlanningStatus>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_project_dir, store) = load_store()?;

    let tasks: Vec<&Task> = store
        .tasks
        .iter()
        .filter(|t| objective.as_ref().map_or(true, |o| t.objective_id == *o))
        .filter(|t| status.map_or(true, |s| t.status == s))
        .collect();

    println!("{}", serde_json::to_string_pretty(&tasks)?);
    Ok(())
}

pub fn remove(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (project_dir, mut store) = load_store()?;

    let pos = store
        .tasks
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| format!("Task \"{id}\" not found."))?;

    let objective_id = store.tasks[pos].objective_id.clone();
    store.tasks.remove(pos);

    // Remove from parent objective's tasks list
    if let Some(o) = store.objectives.iter_mut().find(|o| o.id == objective_id) {
        o.tasks.retain(|x| x != id);
    }

    // Clean up upstream/downstream references in sibling tasks
    for t in &mut store.tasks {
        t.upstream.retain(|x| x != id);
        t.downstream.retain(|x| x != id);
    }

    store.save(Path::new(&project_dir))?;

    eprintln!(
        "\n  {} Removed task \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{{\"removed\": \"{id}\"}}");
    Ok(())
}
