use std::path::Path;

use crate::{
    error::RexError,
    project::{ProjectMeta, ProjectStore, current_incomplete_step},
    schedule::{mark_task_done, next_pending_chunk, prior_chunk},
};

fn status_json(status: &'static str) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&serde_json::json!({ "status": status }))
}

/// Print the full active project as pretty JSON to stdout.
///
/// # Errors
/// - [`RexError::NoActiveProject`] when `rex/active/project.yaml` is absent
/// - [`RexError::Io`] for filesystem failures
/// - [`RexError::Yaml`] if the project YAML is malformed
/// - [`RexError::JsonSerialize`] if serialization fails
pub fn run_show(cwd: &Path) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let project = store.read_active()?;
    let json = serde_json::to_string_pretty(&project)?;
    println!("{json}");
    Ok(())
}

/// Print project metadata (no `steps`) as pretty JSON to stdout.
///
/// # Errors
/// Same as [`run_show`].
pub fn run_meta(cwd: &Path) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let project = store.read_active()?;
    let meta = ProjectMeta::from(&project);
    let json = serde_json::to_string_pretty(&meta)?;
    println!("{json}");
    Ok(())
}

/// Print the first incomplete step as pretty JSON to stdout.
///
/// When every step is complete, prints `{"status": "all-steps-complete"}` and
/// exits 0 so callers can detect the terminal state without non-zero exit.
///
/// # Errors
/// Same as [`run_show`].
pub fn run_step(cwd: &Path) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let project = store.read_active()?;
    let json = match current_incomplete_step(&project) {
        Some(step) => serde_json::to_string_pretty(step)?,
        None => status_json("all-steps-complete")?,
    };
    println!("{json}");
    Ok(())
}

/// Mark the first incomplete step as `completed = true` in `project.yaml`.
///
/// Sets the top-level `completed` flag when every required step is done.
/// Prints the just-completed step as pretty JSON, or `{"status":"all-steps-complete"}`
/// when no incomplete step remains.
///
/// # Errors
/// Same as [`run_show`].
pub fn run_step_complete(cwd: &Path) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let mut project = store.read_active()?;

    let incomplete_idx = project.steps.iter().position(|s| !s.completed);
    let json = match incomplete_idx {
        None => status_json("all-steps-complete")?,
        Some(idx) => {
            project.steps[idx].completed = true;
            let all_required_done = project.steps.iter().all(|s| !s.required || s.completed);
            if all_required_done {
                project.completed = true;
            }
            let step = project.steps[idx].clone();
            store.write_active(&project)?;
            serde_json::to_string_pretty(&step)?
        }
    };
    println!("{json}");
    Ok(())
}

/// Print the next pending chunk from `schedule.json` as pretty JSON.
///
/// Read-only and idempotent: calling this N times returns the same chunk until
/// `task complete` advances state.
///
/// Prints `{"status":"all-chunks-complete"}` when no pending chunk remains.
///
/// # Errors
/// - [`RexError::ScheduleNotFound`] when `schedule.json` is absent
/// - [`RexError::Io`] for filesystem failures
/// - [`RexError::JsonParse`] if the JSON is malformed
pub fn run_chunk_next(cwd: &Path) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let schedule = store.read_schedule()?;
    let json = match next_pending_chunk(&schedule) {
        Some(chunk) => serde_json::to_string_pretty(chunk)?,
        None => status_json("all-chunks-complete")?,
    };
    println!("{json}");
    Ok(())
}

/// Print the most recently completed chunk from `schedule.json` as pretty JSON.
///
/// Read-only and idempotent.
///
/// Prints `{"status":"no-prior-chunk"}` when no chunk has reached `Done`.
///
/// # Errors
/// Same as [`run_chunk_next`].
pub fn run_chunk_prior(cwd: &Path) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let schedule = store.read_schedule()?;
    let json = match prior_chunk(&schedule) {
        Some(chunk) => serde_json::to_string_pretty(chunk)?,
        None => status_json("no-prior-chunk")?,
    };
    println!("{json}");
    Ok(())
}

/// Update the human-readable metadata fields on the active `project.yaml`.
///
/// Each `Some(value)` overwrites the corresponding field; `None` leaves it
/// untouched. Pass `Some("")` to clear a field back to `null`. The CLI rejects
/// invocations with no fields supplied via a clap arg group, so the no-op case
/// never reaches this function.
///
/// Prints the updated [`ProjectMeta`] as pretty JSON.
///
/// # Errors
/// - [`RexError::NoActiveProject`] when `rex/active/project.yaml` is absent
/// - [`RexError::Io`] / [`RexError::Yaml`] / [`RexError::JsonSerialize`] for I/O failures
pub fn run_update(
    cwd: &Path,
    title: Option<String>,
    subtitle: Option<String>,
    description: Option<String>,
) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let mut project = store.read_active()?;

    if let Some(t) = title {
        project.title = nullable(t);
    }
    if let Some(s) = subtitle {
        project.subtitle = nullable(s);
    }
    if let Some(d) = description {
        project.description = nullable(d);
    }

    store.write_active(&project)?;

    let meta = ProjectMeta::from(&project);
    let json = serde_json::to_string_pretty(&meta)?;
    println!("{json}");
    Ok(())
}

fn nullable(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

/// Mark the current task done in `schedule.json`, increment counters in `project.yaml`.
///
/// Auto-promotes the parent chunk when its last task completes, and the parent
/// phase when its last chunk completes. Increments `chunks_completed` in
/// `project.yaml` on chunk promotion.
///
/// Prints the updated task as pretty JSON, or `{"status":"no-active-task"}` when
/// every task is already done.
///
/// # Errors
/// Same as [`run_chunk_next`].
pub fn run_task_complete(cwd: &Path) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    let mut schedule = store.read_schedule()?;

    let json = match mark_task_done(&mut schedule) {
        None => status_json("no-active-task")?,
        Some(completion) => {
            store.write_schedule_with_counters(&schedule)?;
            serde_json::to_string_pretty(&completion.task)?
        }
    };
    println!("{json}");
    Ok(())
}
