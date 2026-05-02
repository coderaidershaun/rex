//! Derive `project.yaml` counter values from the current schedule state.

use super::types::{Schedule, ScheduleCounters};

/// Derive the four counter values from the current schedule state.
///
/// Called after every mutation to keep `project.yaml` in sync.
pub fn counters_for(s: &Schedule) -> ScheduleCounters {
    let mut chunks_required: u32 = 0;
    let mut tasks_required: u32 = 0;
    let mut chunks_completed: u32 = 0;
    let mut tasks_completed: u32 = 0;

    for phase in &s.phases {
        for chunk in &phase.chunks {
            chunks_required += 1;
            if chunk.state.is_done() {
                chunks_completed += 1;
            }
            for task in &chunk.tasks {
                tasks_required += 1;
                if task.state.is_done() {
                    tasks_completed += 1;
                }
            }
        }
    }

    ScheduleCounters {
        chunks_required,
        tasks_required,
        chunks_completed,
        tasks_completed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectId;
    use crate::schedule::types::{Chunk, Phase, ScheduleState, Task};

    fn task(id: &str, state: ScheduleState) -> Task {
        Task {
            id: id.to_owned(),
            description: id.to_owned(),
            state,
            skill: None,
            inputs: None,
            outputs: None,
        }
    }

    fn chunk(id: &str, state: ScheduleState, tasks: Vec<Task>) -> Chunk {
        Chunk {
            id: id.to_owned(),
            description: id.to_owned(),
            scenarios: vec![],
            spec_refs: vec![],
            blocked_by: vec![],
            state,
            tasks,
        }
    }

    #[test]
    fn counters_for_counts_pending_and_done_correctly() {
        let s = Schedule {
            project: ProjectId::parse("test-project").unwrap(),
            phases: vec![
                Phase {
                    id: "phase-a".to_owned(),
                    description: "Phase A".to_owned(),
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    chunks: vec![
                        chunk(
                            "chunk-1",
                            ScheduleState::Pending,
                            vec![
                                task("task-a", ScheduleState::Pending),
                                task("task-b", ScheduleState::Done),
                            ],
                        ),
                        chunk("chunk-2", ScheduleState::Done, vec![]),
                    ],
                },
                Phase {
                    id: "phase-b".to_owned(),
                    description: "Phase B".to_owned(),
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    chunks: vec![chunk(
                        "chunk-1",
                        ScheduleState::Pending,
                        vec![task("task-a", ScheduleState::Pending)],
                    )],
                },
            ],
        };

        let c = counters_for(&s);
        assert_eq!(c.chunks_required, 3);
        assert_eq!(c.chunks_completed, 1);
        assert_eq!(c.tasks_required, 3);
        assert_eq!(c.tasks_completed, 1);
    }
}
