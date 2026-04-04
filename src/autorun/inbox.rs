//! Inter-autorun message routing and cooperative coordination.
//!
//! When multiple autoruns share the same bot token, messages are routed between
//! them via per-project inbox directories and a shared registry file.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::errors::{RexError, RexResult};

// ── Inbox messages (autorun → autorun) ──────────────────────────────────

/// A message written into an autorun's inbox directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboxMessage {
    /// User's reply to a pending question.
    Reply { text: String },
    /// Kill command received.
    Kill,
    /// Query/stats request received.
    Query,
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

// ── Autorun registry (cooperative triage) ───────────────────────────────

const REGISTRY_FILE: &str = ".rex-autorun-registry.json";

/// Shared registry of active autorun instances.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutorunRegistry {
    pub autoruns: HashMap<String, AutorunEntry>,
}

/// An entry in the autorun registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutorunEntry {
    pub pid: u32,
    pub project_dir: String,
    /// The Telegram message_id this autorun is waiting for a reply to.
    pub expected_message_id: Option<i64>,
}

/// Register an autorun in the shared registry.
pub fn register_autorun(
    root_dir: &Path,
    project_id: &str,
    entry: AutorunEntry,
) -> RexResult<()> {
    let mut registry = load_registry(root_dir);
    registry.autoruns.insert(project_id.to_string(), entry);
    write_registry(root_dir, &registry)
}

/// Deregister an autorun from the shared registry.
pub fn deregister_autorun(root_dir: &Path, project_id: &str) {
    if let Ok(mut registry) = load_registry_raw(root_dir) {
        registry.autoruns.remove(project_id);
        let _ = write_registry(root_dir, &registry);
    }
}

/// Update the expected_message_id for an autorun in the registry.
pub fn update_expected_message_id(
    root_dir: &Path,
    project_id: &str,
    msg_id: Option<i64>,
) {
    if let Ok(mut registry) = load_registry_raw(root_dir)
        && let Some(entry) = registry.autoruns.get_mut(project_id)
    {
        entry.expected_message_id = msg_id;
        let _ = write_registry(root_dir, &registry);
    }
}

/// Load the registry, pruning dead PIDs.
pub fn load_registry(root_dir: &Path) -> AutorunRegistry {
    let mut registry = load_registry_raw(root_dir).unwrap_or_default();

    // Prune dead processes
    let dead: Vec<String> = registry
        .autoruns
        .iter()
        .filter(|(_, entry)| !is_process_alive(entry.pid))
        .map(|(k, _)| k.clone())
        .collect();

    if !dead.is_empty() {
        for id in &dead {
            registry.autoruns.remove(id);
        }
        let _ = write_registry(root_dir, &registry);
    }

    registry
}

fn load_registry_raw(root_dir: &Path) -> RexResult<AutorunRegistry> {
    let path = root_dir.join(REGISTRY_FILE);
    let data = std::fs::read_to_string(&path).map_err(|e| RexError::FileRead {
        path: path.display().to_string(),
        source: e,
    })?;
    serde_json::from_str(&data).map_err(|e| RexError::JsonParse {
        context: "autorun registry".into(),
        source: e,
    })
}

fn write_registry(root_dir: &Path, registry: &AutorunRegistry) -> RexResult<()> {
    let path = root_dir.join(REGISTRY_FILE);
    let tmp = path.with_extension("json.tmp");

    let data = serde_json::to_string_pretty(registry).map_err(|e| RexError::JsonSerialize {
        context: "autorun registry".into(),
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

/// Check if a process is alive via `kill(pid, 0)`.
fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

// ── Poll lock (cooperative triage) ──────────────────────────────────────

const POLL_LOCK_FILE: &str = ".rex-autorun-poll.lock";

/// A guard that releases the file lock on drop.
pub struct PollLockGuard {
    fd: i32,
}

impl Drop for PollLockGuard {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.fd, libc::LOCK_UN);
            libc::close(self.fd);
        }
    }
}

/// Try to acquire the poll lock (non-blocking).
/// Returns `Some(guard)` if acquired, `None` if another autorun holds it.
pub fn try_acquire_poll_lock(root_dir: &Path) -> Option<PollLockGuard> {
    let path = root_dir.join(POLL_LOCK_FILE);

    // Create the file if it doesn't exist
    let c_path = std::ffi::CString::new(path.to_string_lossy().as_bytes()).ok()?;
    let fd = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_CREAT | libc::O_RDWR,
            0o644,
        )
    };
    if fd < 0 {
        return None;
    }

    // Try non-blocking exclusive lock
    let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if result != 0 {
        unsafe { libc::close(fd) };
        return None;
    }

    Some(PollLockGuard { fd })
}
