//! Schedule domain — split into focused siblings re-exported here for a stable public path.

mod addressing;
mod counters;
mod ops;
mod promotion;
mod types;
mod validation;

pub use addressing::{
    find_chunk, find_phase, find_task, rewrite_blocked_by_at_chunk_level,
    rewrite_blocked_by_at_phase_level, slug_from_description, unique_slug,
};
pub use counters::counters_for;
pub use ops::{
    add_chunk, add_phase, add_task, move_chunk, move_phase, move_task, remove_chunk, remove_phase,
    remove_task, update_chunk, update_phase, update_task,
};
pub use promotion::{mark_task_done, next_pending_chunk, prior_chunk};
pub use types::{
    Chunk, ChunkEdit, Phase, PhaseEdit, Schedule, ScheduleCounters, ScheduleState, Task,
    TaskCompletion, TaskEdit,
};
pub use validation::{
    validate_no_chunk_cycles, validate_no_phase_cycles, validate_no_state_regression,
    validate_slug_uniqueness,
};
