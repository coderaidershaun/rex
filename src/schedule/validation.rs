//! Schedule invariant checks: cycle detection, slug uniqueness, and state regression guards.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::RexError;

use super::types::Schedule;

/// Check for cycles in phase-level `blocked_by` references using Kahn's algorithm.
///
/// Returns `Err(BlockedByCycle)` naming one member of the cycle.
pub fn validate_no_phase_cycles(s: &Schedule) -> Result<(), RexError> {
    let ids: Vec<&str> = s.phases.iter().map(|p| p.id.as_str()).collect();
    let id_set: HashSet<&str> = ids.iter().copied().collect();
    let mut in_degree: HashMap<&str, usize> = ids.iter().map(|&id| (id, 0)).collect();
    let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();

    for phase in &s.phases {
        for dep in &phase.blocked_by {
            let dep = dep.as_str();
            if id_set.contains(dep) {
                *in_degree.entry(phase.id.as_str()).or_insert(0) += 1;
                edges.entry(dep).or_default().push(phase.id.as_str());
            }
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(id, _)| *id)
        .collect();

    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        if let Some(dependents) = edges.get(node) {
            for &dep in dependents {
                let count = in_degree.get_mut(dep).unwrap();
                *count -= 1;
                if *count == 0 {
                    queue.push_back(dep);
                }
            }
        }
    }

    if visited < ids.len() {
        let cycle_member = in_degree
            .iter()
            .find(|(_, deg)| **deg > 0)
            .map(|(id, _)| *id)
            .expect("Kahn's algorithm guarantees a node with non-zero in-degree when visited < ids.len()");
        return Err(RexError::BlockedByCycle {
            addr: cycle_member.to_owned(),
        });
    }
    Ok(())
}

/// Check for cycles in chunk-level `blocked_by` within each phase.
pub fn validate_no_chunk_cycles(s: &Schedule) -> Result<(), RexError> {
    for phase in &s.phases {
        let ids: Vec<&str> = phase.chunks.iter().map(|c| c.id.as_str()).collect();
        let id_set: HashSet<&str> = ids.iter().copied().collect();
        let mut in_degree: HashMap<&str, usize> = ids.iter().map(|&id| (id, 0)).collect();
        let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();

        for chunk in &phase.chunks {
            for dep in &chunk.blocked_by {
                let dep = dep.as_str();
                if id_set.contains(dep) {
                    *in_degree.entry(chunk.id.as_str()).or_insert(0) += 1;
                    edges.entry(dep).or_default().push(chunk.id.as_str());
                }
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut visited = 0usize;
        while let Some(node) = queue.pop_front() {
            visited += 1;
            if let Some(dependents) = edges.get(node) {
                for &dep in dependents {
                    let count = in_degree.get_mut(dep).unwrap();
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }

        if visited < ids.len() {
            let cycle_member = in_degree
                .iter()
                .find(|(_, deg)| **deg > 0)
                .map(|(id, _)| *id)
                .expect("Kahn's algorithm guarantees a node with non-zero in-degree when visited < ids.len()");
            return Err(RexError::BlockedByCycle {
                addr: cycle_member.to_owned(),
            });
        }
    }
    Ok(())
}

/// Validate that phase slugs are globally unique, chunk slugs are unique within
/// their phase, and task slugs are unique within their chunk.
pub fn validate_slug_uniqueness(s: &Schedule) -> Result<(), RexError> {
    let mut phase_ids = HashSet::new();
    for phase in &s.phases {
        if !phase_ids.insert(phase.id.as_str()) {
            return Err(RexError::DuplicateSlug {
                addr: phase.id.clone(),
            });
        }
        let mut chunk_ids = HashSet::new();
        for chunk in &phase.chunks {
            if !chunk_ids.insert(chunk.id.as_str()) {
                return Err(RexError::DuplicateSlug {
                    addr: chunk.id.clone(),
                });
            }
            let mut task_ids = HashSet::new();
            for task in &chunk.tasks {
                if !task_ids.insert(task.id.as_str()) {
                    return Err(RexError::DuplicateSlug {
                        addr: task.id.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Check that replacing `old` with `new` would not regress any `done` item to non-done.
///
/// Phase, chunk, and task identities are matched by id within their respective scopes.
pub fn validate_no_state_regression(old: &Schedule, new: &Schedule) -> Result<(), RexError> {
    let mut offenders: Vec<String> = Vec::new();

    // Build maps of existing done ids at each tier.
    let done_phases: HashSet<&str> = old
        .phases
        .iter()
        .filter(|p| p.state.is_done())
        .map(|p| p.id.as_str())
        .collect();
    let done_chunks: HashMap<(&str, &str), ()> = old
        .phases
        .iter()
        .flat_map(|p| {
            p.chunks
                .iter()
                .filter(|c| c.state.is_done())
                .map(move |c| ((p.id.as_str(), c.id.as_str()), ()))
        })
        .collect();
    let done_tasks: HashMap<(&str, &str, &str), ()> = old
        .phases
        .iter()
        .flat_map(|p| {
            p.chunks.iter().flat_map(move |c| {
                c.tasks
                    .iter()
                    .filter(|t| t.state.is_done())
                    .map(move |t| ((p.id.as_str(), c.id.as_str(), t.id.as_str()), ()))
            })
        })
        .collect();

    for phase in &new.phases {
        if done_phases.contains(phase.id.as_str()) && !phase.state.is_done() {
            offenders.push(format!("phase:{}", phase.id));
        }
        for chunk in &phase.chunks {
            let key = (phase.id.as_str(), chunk.id.as_str());
            if done_chunks.contains_key(&key) && !chunk.state.is_done() {
                offenders.push(format!("chunk:{}/{}", phase.id, chunk.id));
            }
            for task in &chunk.tasks {
                let key = (phase.id.as_str(), chunk.id.as_str(), task.id.as_str());
                if done_tasks.contains_key(&key) && !task.state.is_done() {
                    offenders.push(format!("task:{}/{}/{}", phase.id, chunk.id, task.id));
                }
            }
        }
    }

    if offenders.is_empty() {
        Ok(())
    } else {
        Err(RexError::ReplaceWouldRegressState {
            offenders: offenders.join(", "),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectId;
    use crate::schedule::types::{Chunk, Phase, ScheduleState, Task};

    fn pid() -> ProjectId {
        ProjectId::parse("test-project").unwrap()
    }

    fn make_schedule() -> Schedule {
        Schedule {
            project: pid(),
            phases: vec![
                Phase {
                    id: "phase-a".to_owned(),
                    description: "Phase A".to_owned(),
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    chunks: vec![
                        Chunk {
                            id: "chunk-1".to_owned(),
                            description: "Chunk 1".to_owned(),
                            scenarios: vec![],
                            spec_refs: vec![],
                            blocked_by: vec![],
                            state: ScheduleState::Pending,
                            tasks: vec![
                                Task {
                                    id: "task-a".to_owned(),
                                    description: "Task A".to_owned(),
                                    state: ScheduleState::Pending,
                                    skill: None,
                                    inputs: None,
                                    outputs: None,
                                },
                                Task {
                                    id: "task-b".to_owned(),
                                    description: "Task B".to_owned(),
                                    state: ScheduleState::Done,
                                    skill: None,
                                    inputs: None,
                                    outputs: None,
                                },
                            ],
                        },
                        Chunk {
                            id: "chunk-2".to_owned(),
                            description: "Chunk 2".to_owned(),
                            scenarios: vec![],
                            spec_refs: vec![],
                            blocked_by: vec!["chunk-1".to_owned()],
                            state: ScheduleState::Done,
                            tasks: vec![],
                        },
                    ],
                },
                Phase {
                    id: "phase-b".to_owned(),
                    description: "Phase B".to_owned(),
                    blocked_by: vec!["phase-a".to_owned()],
                    state: ScheduleState::Pending,
                    chunks: vec![Chunk {
                        id: "chunk-1".to_owned(),
                        description: "Chunk 1 in phase B".to_owned(),
                        scenarios: vec![],
                        spec_refs: vec![],
                        blocked_by: vec![],
                        state: ScheduleState::Pending,
                        tasks: vec![Task {
                            id: "task-a".to_owned(),
                            description: "Task A in phase B".to_owned(),
                            state: ScheduleState::Pending,
                            skill: None,
                            inputs: None,
                            outputs: None,
                        }],
                    }],
                },
            ],
        }
    }

    // ── validate_no_state_regression ─────────────────────────────────────────

    #[test]
    fn validate_no_regression_passes_when_clean() {
        let old = make_schedule();
        let new = make_schedule();
        assert!(validate_no_state_regression(&old, &new).is_ok());
    }

    #[test]
    fn validate_regression_fails_when_done_becomes_pending() {
        let old = make_schedule();
        let mut new = make_schedule();
        // task-b in phase-a/chunk-1 is Done in old; set to Pending in new
        new.phases[0].chunks[0].tasks[1].state = ScheduleState::Pending;
        let err = validate_no_state_regression(&old, &new).unwrap_err();
        assert!(matches!(err, RexError::ReplaceWouldRegressState { .. }));
    }
}
