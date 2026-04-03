use crate::errors::{RexError, RexResult};
use crate::models::planning::{
    apply_all_list_mods, ListMods, PlanningStatus, PlanningStore, Task,
};
use crate::models::project::ProjectRegistry;
use crate::models::project_status::Agent;
use console::style;
use std::collections::{HashSet, VecDeque};
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

/// Build an `Agent` from optional CLI fields. Returns `None` if no agent
/// fields were provided. When updating, merges with the existing agent.
fn build_agent(
    existing: Option<&Agent>,
    model: Option<String>,
    effort: Option<String>,
    skills: Vec<String>,
    count: Option<u32>,
) -> Option<Agent> {
    let has_input = model.is_some() || effort.is_some() || !skills.is_empty() || count.is_some();

    if !has_input {
        return existing.cloned();
    }

    match existing {
        Some(prev) => Some(Agent {
            count: count.unwrap_or(prev.count),
            effort: effort.unwrap_or_else(|| prev.effort.clone()),
            model: model.unwrap_or_else(|| prev.model.clone()),
            skills: if skills.is_empty() {
                prev.skills.clone()
            } else {
                skills
            },
        }),
        None => Some(Agent {
            count: count.unwrap_or(1),
            effort: effort.unwrap_or_else(|| "high".to_string()),
            model: model.unwrap_or_else(|| "sonnet".to_string()),
            skills,
        }),
    }
}

pub fn upsert(
    id: &str,
    objective: Option<String>,
    title: Option<String>,
    description: Option<String>,
    status: Option<PlanningStatus>,
    agent_model: Option<String>,
    agent_effort: Option<String>,
    agent_skills: Vec<String>,
    agent_count: Option<u32>,
    mods: ListMods,
) -> RexResult<()> {
    let (project_dir, mut store) = load_store()?;

    let is_new = !store.tasks.iter().any(|t| t.id == id);

    if is_new {
        let objective_id = objective.ok_or_else(|| {
            RexError::Validation("--objective is required when creating a new task.".into())
        })?;
        let title = title.ok_or_else(|| {
            RexError::Validation("--title is required when creating a new task.".into())
        })?;
        let description = description.ok_or_else(|| {
            RexError::Validation("--description is required when creating a new task.".into())
        })?;

        // Validate parent objective exists
        if !store.objectives.iter().any(|o| o.id == objective_id) {
            return Err(RexError::NotFound(format!(
                "Objective \"{objective_id}\" not found."
            )));
        }

        let agent = build_agent(None, agent_model, agent_effort, agent_skills, agent_count);

        let mut task = Task {
            id: id.to_string(),
            objective_id: objective_id.clone(),
            title,
            description,
            status: status.unwrap_or(PlanningStatus::NotStarted),
            agent,
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
        let task = store
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .expect("verified by is_new check");

        // Handle re-parenting
        if let Some(ref new_objective) = objective {
            if *new_objective != task.objective_id {
                if !store.objectives.iter().any(|o| o.id == *new_objective) {
                    return Err(RexError::NotFound(format!(
                        "Objective \"{new_objective}\" not found."
                    )));
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

        let task = store
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .expect("verified by is_new check");

        if let Some(t) = title {
            task.title = t;
        }
        if let Some(d) = description {
            task.description = d;
        }
        if let Some(s) = status {
            task.status = s;
        }

        task.agent = build_agent(
            task.agent.as_ref(),
            agent_model,
            agent_effort,
            agent_skills,
            agent_count,
        );

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

    let task = store
        .tasks
        .iter()
        .find(|t| t.id == id)
        .expect("just created or modified");
    let action = if is_new { "Created" } else { "Updated" };

    eprintln!(
        "\n  {} {action} task \"{id}\".\n",
        style("\u{2713}").green().bold(),
    );
    println!("{}", serde_json::to_string_pretty(task)?);

    Ok(())
}

pub fn get(id: &str) -> RexResult<()> {
    let (_project_dir, store) = load_store()?;

    let task = store
        .tasks
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| RexError::NotFound(format!("Task \"{id}\" not found.")))?;

    println!("{}", serde_json::to_string_pretty(task)?);
    Ok(())
}

pub fn list(
    objective: Option<String>,
    status: Option<PlanningStatus>,
) -> RexResult<()> {
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

pub fn next() -> RexResult<()> {
    let (_project_dir, store) = load_store()?;

    if store.tasks.is_empty() {
        return Err(RexError::NotFound(
            "No tasks exist in the planning store.".into(),
        ));
    }

    // --- Build eligibility sets at milestone and objective level ---

    // A milestone is eligible if it's not blocked and all its upstream milestones are completed.
    let milestone_eligible: HashSet<&str> = store
        .milestones
        .iter()
        .filter(|m| m.status != PlanningStatus::Blocked)
        .filter(|m| {
            m.upstream.iter().all(|up_id| {
                store
                    .milestones
                    .iter()
                    .find(|m2| m2.id == *up_id)
                    .map_or(true, |m2| m2.status == PlanningStatus::Completed)
            })
        })
        .map(|m| m.id.as_str())
        .collect();

    // An objective is eligible if it's not blocked, its parent milestone is eligible,
    // and all its upstream objectives are completed.
    let objective_eligible: HashSet<&str> = store
        .objectives
        .iter()
        .filter(|o| o.status != PlanningStatus::Blocked)
        .filter(|o| milestone_eligible.contains(o.milestone_id.as_str()))
        .filter(|o| {
            o.upstream.iter().all(|up_id| {
                store
                    .objectives
                    .iter()
                    .find(|o2| o2.id == *up_id)
                    .map_or(true, |o2| o2.status == PlanningStatus::Completed)
            })
        })
        .map(|o| o.id.as_str())
        .collect();

    // --- Find eligible tasks ---
    // A task is eligible if:
    //  - It's in-progress (always eligible — resume unfinished work), OR
    //  - It's not-started AND all its upstream tasks are completed
    //  - Its parent objective is eligible (not blocked, deps met)

    let candidates: Vec<(usize, &Task)> = store
        .tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            if t.status == PlanningStatus::Completed || t.status == PlanningStatus::Blocked {
                return false;
            }
            if !objective_eligible.contains(t.objective_id.as_str()) {
                return false;
            }
            if t.status == PlanningStatus::NotStarted {
                let upstream_met = t.upstream.iter().all(|up_id| {
                    store
                        .tasks
                        .iter()
                        .find(|t2| t2.id == *up_id)
                        .map_or(true, |t2| t2.status == PlanningStatus::Completed)
                });
                if !upstream_met {
                    return false;
                }
            }
            true
        })
        .collect();

    if candidates.is_empty() {
        let all_done = store
            .tasks
            .iter()
            .all(|t| t.status == PlanningStatus::Completed);
        if all_done {
            return Err(RexError::Validation(
                "NO TASKS - Please mark as item complete".into(),
            ));
        }
        return Err(RexError::Validation(
            "No eligible tasks. Remaining tasks are blocked by unmet dependencies.".into(),
        ));
    }

    // --- Score candidates ---
    //
    // Priority tiers (lower = better):
    //   0: task is in-progress (resume unfinished work)
    //   1: objective in-progress, milestone in-progress (finish current work)
    //   2: objective not-started, milestone in-progress (continue current milestone)
    //   3: objective in-progress, milestone not-in-progress (finish scattered objective)
    //   4: everything else (start fresh work)
    //
    // Within a tier, sort by:
    //   - Transitive downstream impact (descending — more unblocked = better)
    //   - Milestone array position (ascending — earlier = higher priority)
    //   - Objective array position (ascending)
    //   - Task array position (ascending)

    let downstream_counts: Vec<usize> = candidates
        .iter()
        .map(|(_, t)| transitive_downstream_count(&t.id, &store.tasks))
        .collect();
    let max_downstream = downstream_counts.iter().max().copied().unwrap_or(0);

    // Pre-compute position lookups
    let ms_pos = |id: &str| -> usize {
        store
            .milestones
            .iter()
            .position(|m| m.id == id)
            .unwrap_or(usize::MAX)
    };
    let obj_pos = |id: &str| -> usize {
        store
            .objectives
            .iter()
            .position(|o| o.id == id)
            .unwrap_or(usize::MAX)
    };

    let mut scored: Vec<(usize, (usize, usize, usize, usize, usize))> = candidates
        .iter()
        .enumerate()
        .map(|(ci, (task_idx, task))| {
            let obj = store.objectives.iter().find(|o| o.id == task.objective_id);
            let ms_id = obj.map(|o| o.milestone_id.as_str()).unwrap_or("");
            let ms = store.milestones.iter().find(|m| m.id == ms_id);

            let obj_status = obj
                .map(|o| o.status)
                .unwrap_or(PlanningStatus::NotStarted);
            let ms_status = ms
                .map(|m| m.status)
                .unwrap_or(PlanningStatus::NotStarted);

            let tier = if task.status == PlanningStatus::InProgress {
                0
            } else if obj_status == PlanningStatus::InProgress
                && ms_status == PlanningStatus::InProgress
            {
                1
            } else if ms_status == PlanningStatus::InProgress {
                2
            } else if obj_status == PlanningStatus::InProgress {
                3
            } else {
                4
            };

            // Invert so higher downstream count → lower sort key
            let impact_inverted = max_downstream - downstream_counts[ci];

            (
                ci,
                (tier, impact_inverted, ms_pos(ms_id), obj_pos(&task.objective_id), *task_idx),
            )
        })
        .collect();

    scored.sort_by_key(|(_, score)| *score);

    let best_ci = scored[0].0;
    let (_, best_task) = candidates[best_ci];

    let objective = store
        .objectives
        .iter()
        .find(|o| o.id == best_task.objective_id);
    let milestone = objective
        .and_then(|o| store.milestones.iter().find(|m| m.id == o.milestone_id));

    let output = serde_json::json!({
        "task": best_task,
        "objective": objective,
        "milestone": milestone,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Count how many tasks are transitively downstream of `task_id` (BFS).
fn transitive_downstream_count(task_id: &str, tasks: &[Task]) -> usize {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    if let Some(task) = tasks.iter().find(|t| t.id == task_id) {
        for down in &task.downstream {
            if visited.insert(down.clone()) {
                queue.push_back(down.clone());
            }
        }
    }
    while let Some(id) = queue.pop_front() {
        if let Some(task) = tasks.iter().find(|t| t.id == id) {
            for down in &task.downstream {
                if visited.insert(down.clone()) {
                    queue.push_back(down.clone());
                }
            }
        }
    }
    visited.len()
}

pub fn remove(id: &str) -> RexResult<()> {
    let (project_dir, mut store) = load_store()?;

    let pos = store
        .tasks
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| RexError::NotFound(format!("Task \"{id}\" not found.")))?;

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
