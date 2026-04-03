use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::{info, warn};

use super::types::{AutorunPhase, AutorunState};

/// Write state atomically: write to `.json.tmp`, fsync, then rename.
pub fn write_state_atomic(path: &Path, state: &AutorunState) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_string_pretty(state)
        .context("failed to serialize autorun state")?;
    let mut file = std::fs::File::create(&tmp)
        .context("failed to create state tmp file")?;
    file.write_all(data.as_bytes())
        .context("failed to write state tmp file")?;
    file.sync_all()
        .context("failed to fsync state tmp file")?;
    std::fs::rename(&tmp, path)
        .context("failed to rename state tmp file")?;
    Ok(())
}

/// Load and parse the state file. Returns `None` on missing or corrupt file.
pub fn read_state(path: &Path) -> Option<AutorunState> {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return None,
    };
    match serde_json::from_str::<AutorunState>(&data) {
        Ok(state) => Some(state),
        Err(e) => {
            warn!("corrupt state file, ignoring: {e}");
            None
        }
    }
}

/// Remove both the state file and its tmp companion.
pub fn delete_state(path: &Path) {
    let _ = std::fs::remove_file(path);
    let tmp = path.with_extension("json.tmp");
    let _ = std::fs::remove_file(tmp);
}

/// Recovery action determined from the state file on startup.
#[derive(Debug)]
pub enum RecoveryAction {
    /// No state file — clean start.
    CleanStart,
    /// Phase was `running` — orphan killed or already dead, start fresh.
    StartFresh { stats: super::types::RunStats, invocation_count: u32 },
    /// Phase was `pending_input` — re-send question to Telegram, wait for reply.
    ResumePendingInput {
        session_id: String,
        question: String,
        telegram_update_offset: Option<i64>,
        stats: super::types::RunStats,
        invocation_count: u32,
    },
}

/// Full recovery matrix: check PID alive, classify phase, kill orphans if needed.
pub fn recover_state(path: &Path) -> RecoveryAction {
    // Clean up any stale tmp file
    let tmp = path.with_extension("json.tmp");
    if tmp.exists() && !path.exists() {
        info!("found stale .json.tmp without .json — deleting");
        let _ = std::fs::remove_file(&tmp);
    }

    let state = match read_state(path) {
        Some(s) => s,
        None => return RecoveryAction::CleanStart,
    };

    match state.phase {
        AutorunPhase::Running => {
            // Kill orphan process group if still alive
            if let Some(pgid) = state.claude_pgid {
                if is_process_alive(state.claude_pid) {
                    info!(pgid, "killing orphan claude process group");
                    kill_process_group_sync(pgid);
                } else {
                    info!("orphan claude process already dead");
                }
            }
            delete_state(path);
            RecoveryAction::StartFresh {
                stats: state.stats,
                invocation_count: state.invocation_count,
            }
        }
        AutorunPhase::PendingInput => {
            match state.session_id {
                Some(session_id) if state.pending_question.is_some() => {
                    // Kill any leftover claude process just in case
                    if let Some(pgid) = state.claude_pgid {
                        if is_process_alive(state.claude_pid) {
                            info!(pgid, "killing leftover claude process from pending_input state");
                            kill_process_group_sync(pgid);
                        }
                    }
                    RecoveryAction::ResumePendingInput {
                        session_id,
                        question: state.pending_question.unwrap(),
                        telegram_update_offset: state.telegram_update_offset,
                        stats: state.stats,
                        invocation_count: state.invocation_count,
                    }
                }
                _ => {
                    warn!("pending_input state without session_id or question — corrupt");
                    delete_state(path);
                    RecoveryAction::StartFresh {
                        stats: state.stats,
                        invocation_count: state.invocation_count,
                    }
                }
            }
        }
    }
}

/// Check if a process is alive via `kill(pid, 0)`.
fn is_process_alive(pid: Option<u32>) -> bool {
    match pid {
        Some(pid) => unsafe { libc::kill(pid as i32, 0) == 0 },
        None => false,
    }
}

/// Synchronous kill of a process group: SIGTERM, wait briefly, SIGKILL.
fn kill_process_group_sync(pgid: i32) {
    unsafe {
        libc::killpg(pgid, libc::SIGTERM);
    }
    std::thread::sleep(std::time::Duration::from_secs(2));
    unsafe {
        libc::killpg(pgid, libc::SIGKILL);
    }
}
