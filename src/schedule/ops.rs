//! CRUD mutations for phases, chunks, and tasks — the 12 schedule operations.

use crate::error::RexError;

use super::addressing::{
    find_chunk, find_phase, find_task, rewrite_blocked_by_at_chunk_level,
    rewrite_blocked_by_at_phase_level,
};
use super::types::{Chunk, ChunkEdit, Phase, PhaseEdit, Schedule, Task, TaskEdit};

// ── Phase mutations ───────────────────────────────────────────────────────────

/// Append a new phase, returning the persisted phase.
pub fn add_phase(s: &mut Schedule, phase: Phase) -> Phase {
    s.phases.push(phase.clone());
    phase
}

/// Update a phase identified by `addr`.
///
/// Renames rewrite all phase-level `blocked_by` references across all phases.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match any phase.
/// - [`RexError::DuplicateSlug`] when `edit.new_id` collides with an existing phase.
pub fn update_phase(s: &mut Schedule, addr: &str, edit: PhaseEdit) -> Result<Phase, RexError> {
    let (idx, _) = find_phase(s, addr)?;

    if let Some(ref new_id) = edit.new_id {
        let collision = s
            .phases
            .iter()
            .enumerate()
            .any(|(i, p)| i != idx && p.id == *new_id);
        if collision {
            return Err(RexError::DuplicateSlug {
                addr: new_id.clone(),
            });
        }
        let old_id = s.phases[idx].id.clone();
        rewrite_blocked_by_at_phase_level(s, &old_id, Some(new_id));
        s.phases[idx].id = new_id.clone();
    }
    if let Some(desc) = edit.description {
        s.phases[idx].description = desc;
    }
    if let Some(state) = edit.state {
        s.phases[idx].state = state;
    }
    if let Some(blocked_by) = edit.blocked_by {
        s.phases[idx].blocked_by = blocked_by;
    }

    Ok(s.phases[idx].clone())
}

/// Remove a phase identified by `addr`, dropping dangling `blocked_by` references.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match any phase.
pub fn remove_phase(s: &mut Schedule, addr: &str) -> Result<Phase, RexError> {
    let (idx, _) = find_phase(s, addr)?;
    let removed = s.phases.remove(idx);
    rewrite_blocked_by_at_phase_level(s, &removed.id, None);
    Ok(removed)
}

/// Move a phase to 1-indexed position `to`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::ScheduleAddrNotFound`] when `to` is out of range.
pub fn move_phase(s: &mut Schedule, addr: &str, to: usize) -> Result<Phase, RexError> {
    let (from_idx, _) = find_phase(s, addr)?;
    let target_idx = to.saturating_sub(1);
    if target_idx >= s.phases.len() {
        return Err(RexError::ScheduleAddrNotFound {
            addr: format!("{to}"),
        });
    }
    let phase = s.phases.remove(from_idx);
    s.phases.insert(target_idx, phase);
    Ok(s.phases[target_idx].clone())
}

// ── Chunk mutations ───────────────────────────────────────────────────────────

/// Append a chunk to the phase identified by `phase_addr`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `phase_addr` does not match.
pub fn add_chunk(s: &mut Schedule, phase_addr: &str, chunk: Chunk) -> Result<Chunk, RexError> {
    let (phase_idx, _) = find_phase(s, phase_addr)?;
    s.phases[phase_idx].chunks.push(chunk.clone());
    Ok(chunk)
}

/// Update a chunk identified by `addr`.
///
/// Renames rewrite chunk-level `blocked_by` within the same phase.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::DuplicateSlug`] when `edit.new_id` collides within the same phase.
/// - [`RexError::AmbiguousAddr`] when `addr` is a bare slug matching multiple phases.
pub fn update_chunk(s: &mut Schedule, addr: &str, edit: ChunkEdit) -> Result<Chunk, RexError> {
    let (phase_idx, chunk_idx, _) = find_chunk(s, addr)?;

    if let Some(ref new_id) = edit.new_id {
        let collision = s.phases[phase_idx]
            .chunks
            .iter()
            .enumerate()
            .any(|(i, c)| i != chunk_idx && c.id == *new_id);
        if collision {
            return Err(RexError::DuplicateSlug {
                addr: new_id.clone(),
            });
        }
        let old_id = s.phases[phase_idx].chunks[chunk_idx].id.clone();
        rewrite_blocked_by_at_chunk_level(s, phase_idx, &old_id, Some(new_id));
        s.phases[phase_idx].chunks[chunk_idx].id = new_id.clone();
    }
    if let Some(desc) = edit.description {
        s.phases[phase_idx].chunks[chunk_idx].description = desc;
    }
    if let Some(state) = edit.state {
        s.phases[phase_idx].chunks[chunk_idx].state = state;
    }
    if let Some(blocked_by) = edit.blocked_by {
        s.phases[phase_idx].chunks[chunk_idx].blocked_by = blocked_by;
    }
    if let Some(scenarios) = edit.scenarios {
        s.phases[phase_idx].chunks[chunk_idx].scenarios = scenarios;
    }
    if let Some(spec_refs) = edit.spec_refs {
        s.phases[phase_idx].chunks[chunk_idx].spec_refs = spec_refs;
    }

    Ok(s.phases[phase_idx].chunks[chunk_idx].clone())
}

/// Remove a chunk identified by `addr`, dropping dangling `blocked_by` references
/// within its parent phase.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::AmbiguousAddr`] when `addr` is an ambiguous bare slug.
pub fn remove_chunk(s: &mut Schedule, addr: &str) -> Result<Chunk, RexError> {
    let (phase_idx, chunk_idx, _) = find_chunk(s, addr)?;
    let removed = s.phases[phase_idx].chunks.remove(chunk_idx);
    rewrite_blocked_by_at_chunk_level(s, phase_idx, &removed.id, None);
    Ok(removed)
}

/// Move a chunk to a new position and optionally a new parent phase.
///
/// `to_phase_addr` re-parents; `to` sets 1-indexed position within the destination phase.
/// If neither is supplied the call is a no-op and the current chunk is returned.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when any address does not match.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn move_chunk(
    s: &mut Schedule,
    addr: &str,
    to_phase_addr: Option<&str>,
    to: Option<usize>,
) -> Result<Chunk, RexError> {
    let (src_phase_idx, src_chunk_idx, _) = find_chunk(s, addr)?;

    // No flags supplied — no-op, return the chunk as-is.
    if to_phase_addr.is_none() && to.is_none() {
        return Ok(s.phases[src_phase_idx].chunks[src_chunk_idx].clone());
    }

    let dst_phase_idx = if let Some(pa) = to_phase_addr {
        find_phase(s, pa)?.0
    } else {
        src_phase_idx
    };

    let chunk = s.phases[src_phase_idx].chunks.remove(src_chunk_idx);

    // Adjust dst_chunk_idx for possible index shift if same phase.
    let dst_len = s.phases[dst_phase_idx].chunks.len();
    let insert_idx = if let Some(pos) = to {
        (pos - 1).min(dst_len)
    } else {
        dst_len
    };

    s.phases[dst_phase_idx].chunks.insert(insert_idx, chunk);
    Ok(s.phases[dst_phase_idx].chunks[insert_idx].clone())
}

// ── Task mutations ────────────────────────────────────────────────────────────

/// Append a task to the chunk identified by `chunk_addr`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `chunk_addr` does not match.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn add_task(s: &mut Schedule, chunk_addr: &str, task: Task) -> Result<Task, RexError> {
    let (phase_idx, chunk_idx, _) = find_chunk(s, chunk_addr)?;
    s.phases[phase_idx].chunks[chunk_idx]
        .tasks
        .push(task.clone());
    Ok(task)
}

/// Update a task identified by `addr`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::DuplicateSlug`] when `edit.new_id` collides within the same chunk.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn update_task(s: &mut Schedule, addr: &str, edit: TaskEdit) -> Result<Task, RexError> {
    let (phase_idx, chunk_idx, task_idx, _) = find_task(s, addr)?;

    if let Some(ref new_id) = edit.new_id {
        let collision = s.phases[phase_idx].chunks[chunk_idx]
            .tasks
            .iter()
            .enumerate()
            .any(|(i, t)| i != task_idx && t.id == *new_id);
        if collision {
            return Err(RexError::DuplicateSlug {
                addr: new_id.clone(),
            });
        }
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].id = new_id.clone();
    }
    if let Some(desc) = edit.description {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].description = desc;
    }
    if let Some(state) = edit.state {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].state = state;
    }
    if let Some(skill) = edit.skill {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].skill = skill;
    }
    if let Some(inputs) = edit.inputs {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].inputs = inputs;
    }
    if let Some(outputs) = edit.outputs {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].outputs = outputs;
    }

    Ok(s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].clone())
}

/// Remove a task identified by `addr`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn remove_task(s: &mut Schedule, addr: &str) -> Result<Task, RexError> {
    let (phase_idx, chunk_idx, task_idx, _) = find_task(s, addr)?;
    Ok(s.phases[phase_idx].chunks[chunk_idx].tasks.remove(task_idx))
}

/// Move a task to a new position and optionally a new parent chunk.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when any address does not match.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn move_task(
    s: &mut Schedule,
    addr: &str,
    to_chunk_addr: Option<&str>,
    to: Option<usize>,
) -> Result<Task, RexError> {
    let (src_phase_idx, src_chunk_idx, src_task_idx, _) = find_task(s, addr)?;

    // No flags supplied — no-op, return the task as-is.
    if to_chunk_addr.is_none() && to.is_none() {
        return Ok(s.phases[src_phase_idx].chunks[src_chunk_idx].tasks[src_task_idx].clone());
    }

    let (dst_phase_idx, dst_chunk_idx) = if let Some(ca) = to_chunk_addr {
        let (pi, ci, _) = find_chunk(s, ca)?;
        (pi, ci)
    } else {
        (src_phase_idx, src_chunk_idx)
    };

    let task = s.phases[src_phase_idx].chunks[src_chunk_idx]
        .tasks
        .remove(src_task_idx);

    let dst_len = s.phases[dst_phase_idx].chunks[dst_chunk_idx].tasks.len();
    let insert_idx = if let Some(pos) = to {
        (pos - 1).min(dst_len)
    } else {
        dst_len
    };

    s.phases[dst_phase_idx].chunks[dst_chunk_idx]
        .tasks
        .insert(insert_idx, task);

    Ok(s.phases[dst_phase_idx].chunks[dst_chunk_idx].tasks[insert_idx].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectId;
    use crate::schedule::types::{Chunk, Phase, ScheduleState, Task};

    fn task(id: &str, state: ScheduleState) -> Task {
        Task { id: id.to_owned(), description: id.to_owned(), state, skill: None, inputs: None, outputs: None }
    }

    fn chunk(id: &str, blocked_by: Vec<String>, state: ScheduleState, tasks: Vec<Task>) -> Chunk {
        Chunk { id: id.to_owned(), description: id.to_owned(), scenarios: vec![], spec_refs: vec![], blocked_by, state, tasks }
    }

    fn make_schedule() -> Schedule {
        Schedule {
            project: ProjectId::parse("test-project").unwrap(),
            phases: vec![
                Phase {
                    id: "phase-a".to_owned(),
                    description: "Phase A".to_owned(),
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    chunks: vec![
                        chunk("chunk-1", vec![], ScheduleState::Pending, vec![
                            task("task-a", ScheduleState::Pending),
                            task("task-b", ScheduleState::Done),
                        ]),
                        chunk("chunk-2", vec!["chunk-1".to_owned()], ScheduleState::Done, vec![]),
                    ],
                },
                Phase {
                    id: "phase-b".to_owned(),
                    description: "Phase B".to_owned(),
                    blocked_by: vec!["phase-a".to_owned()],
                    state: ScheduleState::Pending,
                    chunks: vec![chunk("chunk-1", vec![], ScheduleState::Pending, vec![
                        task("task-a", ScheduleState::Pending),
                    ])],
                },
            ],
        }
    }

    // ── phase mutations ───────────────────────────────────────────────────────

    #[test]
    fn add_phase_appends() {
        let mut s = make_schedule();
        let new_phase = Phase {
            id: "phase-c".to_owned(),
            description: "Phase C".to_owned(),
            blocked_by: vec![],
            state: ScheduleState::Pending,
            chunks: vec![],
        };
        add_phase(&mut s, new_phase);
        assert_eq!(s.phases.len(), 3);
        assert_eq!(s.phases[2].id, "phase-c");
    }

    #[test]
    fn update_phase_renames_and_rewrites_refs() {
        let mut s = make_schedule();
        let edit = PhaseEdit {
            new_id: Some("phase-alpha".to_owned()),
            ..Default::default()
        };
        update_phase(&mut s, "phase-a", edit).unwrap();
        assert_eq!(s.phases[0].id, "phase-alpha");
        // phase-b blocked_by must now say phase-alpha
        assert!(s.phases[1].blocked_by.contains(&"phase-alpha".to_owned()));
    }

    #[test]
    fn remove_phase_drops_dangling_refs() {
        let mut s = make_schedule();
        remove_phase(&mut s, "phase-a").unwrap();
        assert_eq!(s.phases.len(), 1);
        assert!(s.phases[0].blocked_by.is_empty());
    }

    #[test]
    fn move_phase_reorders() {
        let mut s = make_schedule();
        move_phase(&mut s, "phase-b", 1).unwrap();
        assert_eq!(s.phases[0].id, "phase-b");
        assert_eq!(s.phases[1].id, "phase-a");
    }

    // ── chunk mutations ───────────────────────────────────────────────────────

    #[test]
    fn add_chunk_under_phase() {
        let mut s = make_schedule();
        let chunk = Chunk {
            id: "chunk-new".to_owned(),
            description: "New chunk".to_owned(),
            scenarios: vec![],
            spec_refs: vec![],
            blocked_by: vec![],
            state: ScheduleState::Pending,
            tasks: vec![],
        };
        add_chunk(&mut s, "phase-a", chunk).unwrap();
        assert_eq!(s.phases[0].chunks.len(), 3);
        assert_eq!(s.phases[0].chunks[2].id, "chunk-new");
    }

    #[test]
    fn remove_chunk_cascades_and_drops_refs() {
        let mut s = make_schedule();
        // chunk-2 blocked_by chunk-1; removing chunk-1 should drop that ref
        remove_chunk(&mut s, "1.1").unwrap();
        assert!(s.phases[0].chunks[0].blocked_by.is_empty());
    }

    #[test]
    fn move_chunk_to_other_phase() {
        let mut s = make_schedule();
        // Move chunk-2 (1.2) to phase-b
        move_chunk(&mut s, "1.2", Some("phase-b"), None).unwrap();
        assert_eq!(s.phases[0].chunks.len(), 1);
        assert_eq!(s.phases[1].chunks.len(), 2);
    }

    // ── task mutations ────────────────────────────────────────────────────────

    #[test]
    fn add_task_under_chunk() {
        let mut s = make_schedule();
        let task = Task {
            id: "task-new".to_owned(),
            description: "New task".to_owned(),
            state: ScheduleState::Pending,
            skill: None,
            inputs: None,
            outputs: None,
        };
        add_task(&mut s, "1.1", task).unwrap();
        assert_eq!(s.phases[0].chunks[0].tasks.len(), 3);
        assert_eq!(s.phases[0].chunks[0].tasks[2].id, "task-new");
    }

    #[test]
    fn update_task_changes_state() {
        let mut s = make_schedule();
        let edit = TaskEdit {
            state: Some(ScheduleState::Done),
            ..Default::default()
        };
        let updated = update_task(&mut s, "1.1.1", edit).unwrap();
        assert_eq!(updated.state, ScheduleState::Done);
    }

    #[test]
    fn remove_task_removes_correct() {
        let mut s = make_schedule();
        let removed = remove_task(&mut s, "1.1.1").unwrap();
        assert_eq!(removed.id, "task-a");
        assert_eq!(s.phases[0].chunks[0].tasks.len(), 1);
    }

    #[test]
    fn move_task_to_other_chunk() {
        let mut s = make_schedule();
        // Move task-a (1.1.1) to chunk 2.1 (phase-b, chunk-1)
        move_task(&mut s, "1.1.1", Some("2.1"), None).unwrap();
        assert_eq!(s.phases[0].chunks[0].tasks.len(), 1);
        assert_eq!(s.phases[1].chunks[0].tasks.len(), 2);
    }
}
