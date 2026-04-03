use crate::errors::{RexError, RexResult};
use crate::models::planning::{
    apply_all_list_mods, ListMods, Objective, PlanningStatus, PlanningStore,
};
use crate::models::project::ProjectRegistry;
use console::style;
use std::path::Path;

fn load_store() -> RexResult<(String, PlanningStore)> {
    let registry = ProjectRegistry::load()?;
    let project = registry
        .active
        .ok_or_else(|| RexError::NotFound("No active project.".into()))?;
    let project_dir = format!("rex/{}", project.id);
    let store = PlanningStore::load(Path::new(&project_dir))?;
    Ok((project_dir, store))
}

pub fn upsert(
    id: &str,
    milestone: Option<String>,
    title: Option<String>,
    description: Option<String>,
    status: Option<PlanningStatus>,
    mods: ListMods,
) -> RexResult<()> {
    let (project_dir, mut store) = load_store()?;

    let is_new = !store.objectives.iter().any(|o| o.id == id);

    if is_new {
        let milestone_id = milestone.ok_or_else(|| {
            RexError::Validation("--milestone is required when creating a new objective.".into())
        })?;
        let title = title.ok_or_else(|| {
            RexError::Validation("--title is required when creating a new objective.".into())
        })?;
        let description = description.ok_or_else(|| {
            RexError::Validation("--description is required when creating a new objective.".into())
        })?;

        // Validate parent milestone exists
        if !store.milestones.iter().any(|m| m.id == milestone_id) {
            return Err(RexError::NotFound(format!(
                "Milestone \"{milestone_id}\" not found."
            )));
        }

        let mut objective = Objective {
            id: id.to_string(),
            milestone_id: milestone_id.clone(),
            title,
            description,
            status: status.unwrap_or(PlanningStatus::NotStarted),
            references: Vec::new(),
            outputs: Vec::new(),
            checklist: Vec::new(),
            tasks: Vec::new(),
            upstream: Vec::new(),
            downstream: Vec::new(),
        };

        apply_all_list_mods(
            &mut objective.references,
            &mut objective.outputs,
            &mut objective.upstream,
            &mut objective.downstream,
            &mut objective.checklist,
            &mods,
        )?;

        store.objectives.push(objective);

        // Register in parent milestone's objectives list
        if let Some(m) = store.milestones.iter_mut().find(|m| m.id == milestone_id) {
            if !m.objectives.contains(&id.to_string()) {
                m.objectives.push(id.to_string());
            }
        }
    } else {
        let objective = store
            .objectives
            .iter_mut()
            .find(|o| o.id == id)
            .expect("verified by is_new check");

        // Handle re-parenting
        if let Some(ref new_milestone) = milestone {
            if *new_milestone != objective.milestone_id {
                if !store.milestones.iter().any(|m| m.id == *new_milestone) {
                    return Err(RexError::NotFound(format!(
                        "Milestone \"{new_milestone}\" not found."
                    )));
                }
                let old_milestone = objective.milestone_id.clone();
                objective.milestone_id = new_milestone.clone();

                // Remove from old parent
                if let Some(m) = store.milestones.iter_mut().find(|m| m.id == old_milestone) {
                    m.objectives.retain(|x| x != id);
                }
                // Add to new parent
                if let Some(m) = store.milestones.iter_mut().find(|m| m.id == *new_milestone) {
                    if !m.objectives.contains(&id.to_string()) {
                        m.objectives.push(id.to_string());
                    }
                }
            }
        }

        let objective = store
            .objectives
            .iter_mut()
            .find(|o| o.id == id)
            .expect("verified by is_new check");

        if let Some(t) = title {
            objective.title = t;
        }
        if let Some(d) = description {
            objective.description = d;
        }
        if let Some(s) = status {
            objective.status = s;
        }

        apply_all_list_mods(
            &mut objective.references,
            &mut objective.outputs,
            &mut objective.upstream,
            &mut objective.downstream,
            &mut objective.checklist,
            &mods,
        )?;
    }

    // Bidirectional upstream/downstream maintenance
    let id_string = id.to_string();
    for up_id in &mods.add_upstream {
        if let Some(up) = store.objectives.iter_mut().find(|o| o.id == *up_id) {
            if !up.downstream.contains(&id_string) {
                up.downstream.push(id_string.clone());
            }
        }
    }
    for up_id in &mods.remove_upstream {
        if let Some(up) = store.objectives.iter_mut().find(|o| o.id == *up_id) {
            up.downstream.retain(|x| x != &id_string);
        }
    }
    for down_id in &mods.add_downstream {
        if let Some(down) = store.objectives.iter_mut().find(|o| o.id == *down_id) {
            if !down.upstream.contains(&id_string) {
                down.upstream.push(id_string.clone());
            }
        }
    }
    for down_id in &mods.remove_downstream {
        if let Some(down) = store.objectives.iter_mut().find(|o| o.id == *down_id) {
            down.upstream.retain(|x| x != &id_string);
        }
    }

    store.save(Path::new(&project_dir))?;

    let objective = store
        .objectives
        .iter()
        .find(|o| o.id == id)
        .expect("just created or modified");
    let action = if is_new { "Created" } else { "Updated" };

    eprintln!(
        "\n  {} {action} objective \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{}", serde_json::to_string_pretty(objective)?);

    Ok(())
}

pub fn get(id: &str) -> RexResult<()> {
    let (_project_dir, store) = load_store()?;

    let objective = store
        .objectives
        .iter()
        .find(|o| o.id == id)
        .ok_or_else(|| RexError::NotFound(format!("Objective \"{id}\" not found.")))?;

    println!("{}", serde_json::to_string_pretty(objective)?);
    Ok(())
}

pub fn list(
    milestone: Option<String>,
    status: Option<PlanningStatus>,
) -> RexResult<()> {
    let (_project_dir, store) = load_store()?;

    let objectives: Vec<&Objective> = store
        .objectives
        .iter()
        .filter(|o| milestone.as_ref().map_or(true, |m| o.milestone_id == *m))
        .filter(|o| status.map_or(true, |s| o.status == s))
        .collect();

    println!("{}", serde_json::to_string_pretty(&objectives)?);
    Ok(())
}

pub fn remove(id: &str) -> RexResult<()> {
    let (project_dir, mut store) = load_store()?;

    let pos = store
        .objectives
        .iter()
        .position(|o| o.id == id)
        .ok_or_else(|| RexError::NotFound(format!("Objective \"{id}\" not found.")))?;

    let milestone_id = store.objectives[pos].milestone_id.clone();
    store.objectives.remove(pos);

    // Remove from parent milestone's objectives list
    if let Some(m) = store.milestones.iter_mut().find(|m| m.id == milestone_id) {
        m.objectives.retain(|x| x != id);
    }

    // Clean up upstream/downstream references in sibling objectives
    for o in &mut store.objectives {
        o.upstream.retain(|x| x != id);
        o.downstream.retain(|x| x != id);
    }

    // Identify orphaned tasks
    let orphaned: Vec<String> = store
        .tasks
        .iter()
        .filter(|t| t.objective_id == id)
        .map(|t| t.id.clone())
        .collect();

    store.save(Path::new(&project_dir))?;

    eprintln!(
        "\n  {} Removed objective \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );

    if !orphaned.is_empty() {
        eprintln!(
            "  {} Orphaned tasks: {}\n",
            style("\u{26a0}").yellow().bold(),
            orphaned.join(", ")
        );
    }

    println!("{{\"removed\": \"{id}\"}}");
    Ok(())
}
