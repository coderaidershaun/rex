//! Autopilot advancement: locate the active chunk/task and mark work done.

use super::types::{Chunk, Schedule, ScheduleState, TaskCompletion};

/// Return the first chunk that is still open — `Pending` or `InProgress` —
/// ignoring `blocked_by` (the agent resolves blocking externally).
///
/// Read-only and idempotent: calling this N times returns the same chunk until
/// [`mark_task_done`] advances state.
pub fn next_pending_chunk(schedule: &Schedule) -> Option<&Chunk> {
    schedule
        .phases
        .iter()
        .flat_map(|p| p.chunks.iter())
        .find(|c| c.state.is_open())
}

/// Return the last chunk with state `Done` across all phases, in phase→chunk order.
pub fn prior_chunk(schedule: &Schedule) -> Option<&Chunk> {
    schedule
        .phases
        .iter()
        .flat_map(|p| p.chunks.iter())
        .rfind(|c| c.state == ScheduleState::Done)
}

/// Mark the current task as `Done`, auto-promoting the parent chunk and phase
/// when their respective children are all `Done`.
///
/// Returns `None` when no pending task exists (schedule exhausted).
/// Returns the completed task plus promotion flags so the caller can update
/// counters in `project.yaml`.
pub fn mark_task_done(schedule: &mut Schedule) -> Option<TaskCompletion> {
    // Indices (not refs) so the post-mutation promotion checks can re-borrow.
    let (phase_idx, chunk_idx, task_idx) = find_current_task_indices(schedule)?;

    schedule.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].state = ScheduleState::Done;
    let task = schedule.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].clone();

    let chunk_promoted = if schedule.phases[phase_idx].chunks[chunk_idx]
        .tasks
        .iter()
        .all(|t| t.state == ScheduleState::Done)
    {
        schedule.phases[phase_idx].chunks[chunk_idx].state = ScheduleState::Done;
        true
    } else {
        false
    };

    let phase_promoted = if chunk_promoted
        && schedule.phases[phase_idx]
            .chunks
            .iter()
            .all(|c| c.state == ScheduleState::Done)
    {
        schedule.phases[phase_idx].state = ScheduleState::Done;
        true
    } else {
        false
    };

    Some(TaskCompletion {
        task,
        chunk_promoted,
        phase_promoted,
    })
}

// Chunk predicate matches `next_pending_chunk`: callers rely on `task complete`
// advancing exactly the chunk `chunk-next` reported.
fn find_current_task_indices(schedule: &Schedule) -> Option<(usize, usize, usize)> {
    for (phase_idx, phase) in schedule.phases.iter().enumerate() {
        for (chunk_idx, chunk) in phase.chunks.iter().enumerate() {
            if !chunk.state.is_open() {
                continue;
            }
            if let Some(task_idx) = chunk
                .tasks
                .iter()
                .position(|t| t.state != ScheduleState::Done)
            {
                return Some((phase_idx, chunk_idx, task_idx));
            }
        }
    }
    None
}
