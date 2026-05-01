use std::path::Path;

use crate::{
    error::RexError,
    project::ProjectStore,
    schedule::{
        Chunk, ChunkEdit, Phase, PhaseEdit, Schedule, ScheduleState, Task, TaskEdit, add_chunk,
        add_phase, add_task, move_chunk, move_phase, move_task, remove_chunk, remove_phase,
        remove_task, slug_from_description, unique_slug, update_chunk, update_phase, update_task,
        validate_no_chunk_cycles, validate_no_phase_cycles, validate_no_state_regression,
        validate_slug_uniqueness,
    },
};

// ── Command input structs ─────────────────────────────────────────────────────

/// Input for `run_chunk_update` — avoids exceeding the 7-argument clippy limit.
pub struct ChunkUpdateInput<'a> {
    pub addr: &'a str,
    pub description: Option<&'a str>,
    pub new_id: Option<&'a str>,
    pub state: Option<ScheduleState>,
    pub scenarios: Option<&'a [String]>,
    pub spec_refs: Option<&'a [String]>,
    pub blocked_by: Option<&'a [String]>,
}

/// Input for `run_task_update` — avoids exceeding the 7-argument clippy limit.
pub struct TaskUpdateInput<'a> {
    pub addr: &'a str,
    pub description: Option<&'a str>,
    pub new_id: Option<&'a str>,
    pub state: Option<ScheduleState>,
    pub skill: Option<Option<String>>,
    pub inputs: Option<Option<String>>,
    pub outputs: Option<Option<String>>,
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn status_json(status: &'static str) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&serde_json::json!({ "status": status }))
}

fn pretty<T: serde::Serialize>(value: &T) -> Result<String, RexError> {
    serde_json::to_string_pretty(value).map_err(RexError::JsonSerialize)
}

fn validate_no_dot_in_id(id: &str) -> Result<(), RexError> {
    if id.contains('.') {
        return Err(RexError::InvalidSlug {
            reason: format!(
                "id '{id}' must not contain dots — reserved for dotted position addressing"
            ),
        });
    }
    Ok(())
}

// ── show ──────────────────────────────────────────────────────────────────────

/// Print the full schedule as pretty JSON.
///
/// # Errors
/// - [`RexError::NoActiveProject`] when no project is active.
/// - [`RexError::ScheduleNotFound`] when `schedule.json` is absent.
pub fn run_schedule_show(cwd: &Path) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let schedule = store.read_schedule()?;
    println!("{}", pretty(&schedule)?);
    Ok(())
}

// ── replace ───────────────────────────────────────────────────────────────────

/// Replace the full schedule atomically from a JSON file.
///
/// Validates: no cycles, slug uniqueness, and no state regression vs the current
/// schedule. Writes both `schedule.json` and updated counters in `project.yaml`.
///
/// # Errors
/// - [`RexError::NoActiveProject`] when no project is active.
/// - [`RexError::ScheduleNotFound`] when the current `schedule.json` is absent.
/// - [`RexError::JsonParse`] when `file` is malformed.
/// - [`RexError::BlockedByCycle`] when cycles are detected.
/// - [`RexError::DuplicateSlug`] when slugs are not unique within scope.
/// - [`RexError::ReplaceWouldRegressState`] when the new schedule regresses done items.
pub fn run_schedule_replace(cwd: &Path, file: &Path) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);

    let raw = std::fs::read_to_string(file).map_err(|source| RexError::Io {
        path: file.to_path_buf(),
        source,
    })?;
    let new_schedule: Schedule =
        serde_json::from_str(&raw).map_err(|source| RexError::JsonParse {
            path: file.to_path_buf(),
            source,
        })?;

    validate_slug_uniqueness(&new_schedule)?;
    validate_no_phase_cycles(&new_schedule)?;
    validate_no_chunk_cycles(&new_schedule)?;

    // Only check regression if an existing schedule exists.
    if let Ok(old_schedule) = store.read_schedule() {
        validate_no_state_regression(&old_schedule, &new_schedule)?;
    }

    store.write_schedule_with_counters(&new_schedule)?;
    println!("{}", status_json("ok")?);
    Ok(())
}

// ── phase add ─────────────────────────────────────────────────────────────────

/// Append a new phase.
///
/// # Errors
/// - [`RexError::NoActiveProject`] / [`RexError::ScheduleNotFound`] for missing files.
pub fn run_phase_add(
    cwd: &Path,
    description: &str,
    id: Option<&str>,
    blocked_by: &[String],
) -> Result<(), RexError> {
    if let Some(id) = id {
        validate_no_dot_in_id(id)?;
    }

    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;

    let existing_ids: Vec<&str> = schedule.phases.iter().map(|p| p.id.as_str()).collect();
    let slug = if let Some(id) = id {
        unique_slug(&existing_ids, id)
    } else {
        slug_from_description(&existing_ids, description)
    };

    let phase = Phase {
        id: slug,
        description: description.to_owned(),
        blocked_by: blocked_by.to_vec(),
        state: ScheduleState::Pending,
        chunks: vec![],
    };
    let added = add_phase(&mut schedule, phase);
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&added)?);
    Ok(())
}

// ── phase update ──────────────────────────────────────────────────────────────

/// Update a phase identified by slug or dotted position.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::DuplicateSlug`] when `new_id` collides.
pub fn run_phase_update(
    cwd: &Path,
    addr: &str,
    description: Option<&str>,
    new_id: Option<&str>,
    state: Option<ScheduleState>,
    blocked_by: Option<&[String]>,
) -> Result<(), RexError> {
    if let Some(id) = new_id {
        validate_no_dot_in_id(id)?;
    }

    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;

    let edit = PhaseEdit {
        description: description.map(str::to_owned),
        new_id: new_id.map(str::to_owned),
        state,
        blocked_by: blocked_by.map(|v| v.to_vec()),
    };
    let updated = update_phase(&mut schedule, addr, edit)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&updated)?);
    Ok(())
}

// ── phase remove ──────────────────────────────────────────────────────────────

/// Remove a phase and all its chunks/tasks.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
pub fn run_phase_remove(cwd: &Path, addr: &str) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;
    let removed = remove_phase(&mut schedule, addr)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&removed)?);
    Ok(())
}

// ── phase move ────────────────────────────────────────────────────────────────

/// Move a phase to 1-indexed position `to`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` or `to` is out of range.
pub fn run_phase_move(cwd: &Path, addr: &str, to: usize) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;
    let moved = move_phase(&mut schedule, addr, to)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&moved)?);
    Ok(())
}

// ── chunk add ─────────────────────────────────────────────────────────────────

/// Append a chunk to a phase.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `phase_addr` does not match.
pub fn run_chunk_add(
    cwd: &Path,
    phase_addr: &str,
    description: &str,
    id: Option<&str>,
    scenarios: &[String],
    spec_refs: &[String],
    blocked_by: &[String],
) -> Result<(), RexError> {
    if let Some(id) = id {
        validate_no_dot_in_id(id)?;
    }

    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;

    let (phase_idx, _) = crate::schedule::find_phase(&schedule, phase_addr)?;
    let existing_ids: Vec<&str> = schedule.phases[phase_idx]
        .chunks
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    let slug = if let Some(id) = id {
        unique_slug(&existing_ids, id)
    } else {
        slug_from_description(&existing_ids, description)
    };

    let chunk = Chunk {
        id: slug,
        description: description.to_owned(),
        scenarios: scenarios.to_vec(),
        spec_refs: spec_refs.to_vec(),
        blocked_by: blocked_by.to_vec(),
        state: ScheduleState::Pending,
        tasks: vec![],
    };
    let added = add_chunk(&mut schedule, phase_addr, chunk)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&added)?);
    Ok(())
}

// ── chunk update ──────────────────────────────────────────────────────────────

/// Update a chunk identified by slug or dotted position.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] / [`RexError::AmbiguousAddr`] for bad address.
/// - [`RexError::DuplicateSlug`] when `new_id` collides within the phase.
pub fn run_chunk_update(cwd: &Path, input: ChunkUpdateInput<'_>) -> Result<(), RexError> {
    if let Some(id) = input.new_id {
        validate_no_dot_in_id(id)?;
    }

    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;

    let edit = ChunkEdit {
        description: input.description.map(str::to_owned),
        new_id: input.new_id.map(str::to_owned),
        state: input.state,
        blocked_by: input.blocked_by.map(|v| v.to_vec()),
        scenarios: input.scenarios.map(|v| v.to_vec()),
        spec_refs: input.spec_refs.map(|v| v.to_vec()),
    };
    let updated = update_chunk(&mut schedule, input.addr, edit)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&updated)?);
    Ok(())
}

// ── chunk remove ──────────────────────────────────────────────────────────────

/// Remove a chunk and all its tasks.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] / [`RexError::AmbiguousAddr`] for bad address.
pub fn run_chunk_remove(cwd: &Path, addr: &str) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;
    let removed = remove_chunk(&mut schedule, addr)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&removed)?);
    Ok(())
}

// ── chunk move ────────────────────────────────────────────────────────────────

/// Move a chunk to a new position and optionally a new parent phase.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] / [`RexError::AmbiguousAddr`] for bad address.
pub fn run_chunk_move(
    cwd: &Path,
    addr: &str,
    to_phase: Option<&str>,
    to: Option<usize>,
) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;
    let moved = move_chunk(&mut schedule, addr, to_phase, to)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&moved)?);
    Ok(())
}

// ── task add ──────────────────────────────────────────────────────────────────

/// Append a task to a chunk.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] / [`RexError::AmbiguousAddr`] for bad chunk address.
pub fn run_task_add(
    cwd: &Path,
    chunk_addr: &str,
    description: &str,
    id: Option<&str>,
    skill: Option<&str>,
    inputs: Option<&str>,
    outputs: Option<&str>,
) -> Result<(), RexError> {
    if let Some(id) = id {
        validate_no_dot_in_id(id)?;
    }

    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;

    let (phase_idx, chunk_idx, _) = crate::schedule::find_chunk(&schedule, chunk_addr)?;
    let existing_ids: Vec<&str> = schedule.phases[phase_idx].chunks[chunk_idx]
        .tasks
        .iter()
        .map(|t| t.id.as_str())
        .collect();
    let slug = if let Some(id) = id {
        unique_slug(&existing_ids, id)
    } else {
        slug_from_description(&existing_ids, description)
    };

    let task = Task {
        id: slug,
        description: description.to_owned(),
        state: ScheduleState::Pending,
        skill: skill.map(str::to_owned),
        inputs: inputs.map(str::to_owned),
        outputs: outputs.map(str::to_owned),
    };
    let added = add_task(&mut schedule, chunk_addr, task)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&added)?);
    Ok(())
}

// ── task update ───────────────────────────────────────────────────────────────

/// Update a task identified by slug or dotted position.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] / [`RexError::AmbiguousAddr`] for bad address.
/// - [`RexError::DuplicateSlug`] when `new_id` collides within the chunk.
pub fn run_task_update(cwd: &Path, input: TaskUpdateInput<'_>) -> Result<(), RexError> {
    if let Some(id) = input.new_id {
        validate_no_dot_in_id(id)?;
    }

    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;

    let edit = TaskEdit {
        description: input.description.map(str::to_owned),
        new_id: input.new_id.map(str::to_owned),
        state: input.state,
        skill: input.skill,
        inputs: input.inputs,
        outputs: input.outputs,
    };
    let updated = update_task(&mut schedule, input.addr, edit)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&updated)?);
    Ok(())
}

// ── task remove ───────────────────────────────────────────────────────────────

/// Remove a task.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] / [`RexError::AmbiguousAddr`] for bad address.
pub fn run_task_remove(cwd: &Path, addr: &str) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;
    let removed = remove_task(&mut schedule, addr)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&removed)?);
    Ok(())
}

// ── task move ─────────────────────────────────────────────────────────────────

/// Move a task to a new position and optionally a new parent chunk.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] / [`RexError::AmbiguousAddr`] for bad address.
pub fn run_task_move(
    cwd: &Path,
    addr: &str,
    to_chunk: Option<&str>,
    to: Option<usize>,
) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;
    let moved = move_task(&mut schedule, addr, to_chunk, to)?;
    store.write_schedule_with_counters(&schedule)?;
    println!("{}", pretty(&moved)?);
    Ok(())
}
