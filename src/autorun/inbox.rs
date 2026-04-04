//! File-based IPC between rex-chat and rex-autorun.
//!
//! When rex-chat is running, it becomes the sole Telegram poller.
//! Autorun switches to watching inbox files for messages routed by rex-chat.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::errors::{RexError, RexResult};

// ── Inbox messages (rex-chat → autorun) ───────────────────────────────────

/// A message written by rex-chat into an autorun's inbox directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboxMessage {
    /// User's reply to a pending question.
    Reply { text: String },
    /// Kill command received.
    Kill,
}

/// Write an inbox message atomically to `<project_dir>/.rex-autorun-inbox/`.
///
/// Uses timestamp-based filenames to avoid overwrites when multiple messages
/// arrive before autorun reads them.
pub fn write_inbox(project_dir: &Path, msg: &InboxMessage) -> RexResult<()> {
    let inbox_dir = project_dir.join(".rex-autorun-inbox");
    std::fs::create_dir_all(&inbox_dir).map_err(|e| RexError::DirCreate {
        path: inbox_dir.display().to_string(),
        source: e,
    })?;

    let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let filename = format!("{ts}.json");
    let path = inbox_dir.join(&filename);
    let tmp = inbox_dir.join(format!("{filename}.tmp"));

    let data = serde_json::to_string(msg).map_err(|e| RexError::JsonSerialize {
        context: "inbox message".into(),
        source: e,
    })?;

    let mut file = std::fs::File::create(&tmp).map_err(|e| RexError::FileWrite {
        path: tmp.display().to_string(),
        source: e,
    })?;
    file.write_all(data.as_bytes())
        .map_err(|e| RexError::FileWrite {
            path: tmp.display().to_string(),
            source: e,
        })?;
    file.sync_all().map_err(|e| RexError::FileWrite {
        path: tmp.display().to_string(),
        source: e,
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| RexError::FileWrite {
        path: path.display().to_string(),
        source: e,
    })?;

    debug!(file = %filename, "wrote inbox message");
    Ok(())
}

/// Read the oldest inbox message and delete it. Returns `None` if empty.
pub fn read_inbox(project_dir: &Path) -> Option<InboxMessage> {
    let inbox_dir = project_dir.join(".rex-autorun-inbox");
    let entries = std::fs::read_dir(&inbox_dir).ok()?;

    // Collect .json files, sorted by name (timestamp-based = chronological)
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "json")
        })
        .collect();

    files.sort_by_key(|e| e.file_name());

    let entry = files.first()?;
    let path = entry.path();

    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => {
            warn!("failed to read inbox file {}: {e}", path.display());
            let _ = std::fs::remove_file(&path);
            return None;
        }
    };

    let msg: InboxMessage = match serde_json::from_str(&data) {
        Ok(m) => m,
        Err(e) => {
            warn!("failed to parse inbox file {}: {e}", path.display());
            let _ = std::fs::remove_file(&path);
            return None;
        }
    };

    let _ = std::fs::remove_file(&path);
    debug!(file = %path.display(), "read and deleted inbox message");
    Some(msg)
}

/// Remove the inbox directory and all its contents.
pub fn cleanup_inbox(project_dir: &Path) {
    let inbox_dir = project_dir.join(".rex-autorun-inbox");
    let _ = std::fs::remove_dir_all(&inbox_dir);
}

// ── Rex-chat state (presence + offset handoff) ───────────────────────────

/// State written by rex-chat so autoruns know it's alive and can hand off offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatState {
    pub pid: u32,
    pub telegram_update_offset: i64,
}

const CHAT_STATE_FILE: &str = ".rex-chat.state";

/// Write rex-chat state atomically.
pub fn write_chat_state(project_dir: &Path, state: &ChatState) -> RexResult<()> {
    let path = project_dir.join(CHAT_STATE_FILE);
    let tmp = path.with_extension("state.tmp");

    let data = serde_json::to_string_pretty(state).map_err(|e| RexError::JsonSerialize {
        context: "rex-chat state".into(),
        source: e,
    })?;

    let mut file = std::fs::File::create(&tmp).map_err(|e| RexError::FileWrite {
        path: tmp.display().to_string(),
        source: e,
    })?;
    file.write_all(data.as_bytes())
        .map_err(|e| RexError::FileWrite {
            path: tmp.display().to_string(),
            source: e,
        })?;
    file.sync_all().map_err(|e| RexError::FileWrite {
        path: tmp.display().to_string(),
        source: e,
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| RexError::FileWrite {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

/// Read rex-chat state. Returns `None` if file missing or corrupt.
pub fn read_chat_state(project_dir: &Path) -> Option<ChatState> {
    let path = project_dir.join(CHAT_STATE_FILE);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Delete rex-chat state file.
pub fn delete_chat_state(project_dir: &Path) {
    let path = project_dir.join(CHAT_STATE_FILE);
    let _ = std::fs::remove_file(&path);
    let tmp = path.with_extension("state.tmp");
    let _ = std::fs::remove_file(&tmp);
}

/// Check if the rex-chat daemon is alive by reading state and checking PID.
pub fn rex_chat_is_running(project_dir: &Path) -> bool {
    let Some(state) = read_chat_state(project_dir) else {
        return false;
    };
    is_process_alive(state.pid)
}

/// Check if a process is alive via `kill(pid, 0)`.
fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
