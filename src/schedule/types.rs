//! Core data types for the schedule domain: states, tasks, chunks, phases, and edit structs.

use serde::{Deserialize, Serialize};

use crate::project::ProjectId;

/// Lifecycle state of a task, chunk, or phase.
///
/// `InProgress` is reserved vocabulary for agents who hand-edit `schedule.json`
/// to mark in-flight work; no CLI path writes it today. `chunk-next` and
/// `task complete` both treat `InProgress` as selectable so a hand-edit cannot
/// hide a chunk from the work queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduleState {
    Pending,
    InProgress,
    Done,
    Blocked,
}

impl ScheduleState {
    /// `true` for states that should still be picked up by the work queue
    /// (`Pending` or `InProgress`).
    pub(super) fn is_open(&self) -> bool {
        matches!(self, Self::Pending | Self::InProgress)
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}

impl std::str::FromStr for ScheduleState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "in-progress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            "blocked" => Ok(Self::Blocked),
            other => Err(format!(
                "unknown state '{other}'; expected pending|in-progress|done|blocked"
            )),
        }
    }
}

/// One atomic unit of work within a chunk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Task {
    pub id: String,
    pub description: String,
    pub state: ScheduleState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<String>,
}

/// A vertical slice of work — one agent session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Chunk {
    pub id: String,
    pub description: String,
    pub scenarios: Vec<String>,
    pub spec_refs: Vec<String>,
    pub blocked_by: Vec<String>,
    pub state: ScheduleState,
    pub tasks: Vec<Task>,
}

/// A PRD capability milestone — contains one or more chunks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Phase {
    pub id: String,
    pub description: String,
    pub blocked_by: Vec<String>,
    pub state: ScheduleState,
    pub chunks: Vec<Chunk>,
}

/// The full autopilot work queue for a project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Schedule {
    pub project: ProjectId,
    pub phases: Vec<Phase>,
}

/// Result of marking a task as done, including auto-promotion flags.
pub struct TaskCompletion {
    /// The task that was just marked done.
    pub task: Task,
    /// `true` when the parent chunk was auto-promoted to `Done`.
    pub chunk_promoted: bool,
    /// `true` when the parent phase was auto-promoted to `Done`.
    pub phase_promoted: bool,
}

/// Counter values derived from a schedule's current state.
///
/// Mirrors the four counter fields in `project.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleCounters {
    pub chunks_required: u32,
    pub tasks_required: u32,
    pub chunks_completed: u32,
    pub tasks_completed: u32,
}

/// Optional fields for updating a phase.
#[derive(Debug, Default)]
pub struct PhaseEdit {
    pub description: Option<String>,
    pub new_id: Option<String>,
    pub state: Option<ScheduleState>,
    pub blocked_by: Option<Vec<String>>,
}

/// Optional fields for updating a chunk.
#[derive(Debug, Default)]
pub struct ChunkEdit {
    pub description: Option<String>,
    pub new_id: Option<String>,
    pub state: Option<ScheduleState>,
    pub blocked_by: Option<Vec<String>>,
    pub scenarios: Option<Vec<String>>,
    pub spec_refs: Option<Vec<String>>,
}

/// Optional fields for updating a task.
#[derive(Debug, Default)]
pub struct TaskEdit {
    pub description: Option<String>,
    pub new_id: Option<String>,
    pub state: Option<ScheduleState>,
    pub skill: Option<Option<String>>,
    pub inputs: Option<Option<String>>,
    pub outputs: Option<Option<String>>,
}
