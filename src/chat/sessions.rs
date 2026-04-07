//! Chat session management: per-project Claude sessions with process tracking.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tracing::{error, info, warn};

use crate::errors::{RexError, RexResult};

/// A chat session for a specific project.
pub struct ChatSession {
    pub project_id: String,
    pub project_dir: PathBuf,
    pub claude_session_id: Option<String>,
    pub session_name: String,
    pub last_activity: DateTime<Utc>,
    /// Process group ID of an active Claude process (if running).
    pub active_pgid: Option<i32>,
}

/// Manages chat sessions and message-to-project routing.
pub struct SessionManager {
    sessions: HashMap<String, ChatSession>,
    /// message_id -> project_id for reply routing.
    message_to_project: HashMap<i64, String>,
    /// Last project the user chatted with (for smart routing).
    pub last_active: Option<String>,
}

const CHAT_SYSTEM_PROMPT: &str = r#"You are a project assistant responding to a user query via Telegram.

RULES:
- You MUST invoke the /rex-chat skill to handle the user's request.
- Keep your response under 3500 characters.
- Use plain text only -- no HTML tags, no markdown.
- Do NOT output any JSON status objects.
- Be concise and actionable."#;

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            message_to_project: HashMap::new(),
            last_active: None,
        }
    }

    /// Get or create a session for the given project.
    pub fn get_or_create(
        &mut self,
        project_id: &str,
        project_dir: &str,
    ) -> &mut ChatSession {
        self.sessions
            .entry(project_id.to_string())
            .or_insert_with(|| ChatSession {
                project_id: project_id.to_string(),
                project_dir: PathBuf::from(project_dir),
                claude_session_id: None,
                session_name: format!("rex-chat-{project_id}"),
                last_activity: Utc::now(),
                active_pgid: None,
            })
    }

    /// Reset the Claude session for a project (or all projects if no ID given).
    /// Kills any active process and clears the session ID so the next message starts fresh.
    pub fn reset(&mut self, project_id: Option<&str>) {
        match project_id {
            Some(pid) => {
                if let Some(session) = self.sessions.get_mut(pid) {
                    kill_session_process(session);
                    session.claude_session_id = None;
                    session.active_pgid = None;
                    info!(project_id = %pid, "reset chat session");
                }
            }
            None => {
                for (pid, session) in &mut self.sessions {
                    kill_session_process(session);
                    session.claude_session_id = None;
                    session.active_pgid = None;
                    info!(project_id = %pid, "reset chat session");
                }
            }
        }
    }

    /// Register a Telegram message as belonging to a rex-chat session.
    pub fn register_message(&mut self, msg_id: i64, project_id: &str) {
        self.message_to_project
            .insert(msg_id, project_id.to_string());
    }

    /// Look up which project a reply-to-message belongs to.
    pub fn lookup_reply(&self, reply_to_msg_id: i64) -> Option<&str> {
        self.message_to_project.get(&reply_to_msg_id).map(|s| s.as_str())
    }

    /// Clean up sessions that have been idle longer than the timeout.
    /// Kills any active Claude processes for stale sessions.
    pub fn cleanup_stale(&mut self, timeout: Duration) {
        let cutoff = Utc::now() - chrono::Duration::from_std(timeout).unwrap_or_default();
        let stale: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.last_activity < cutoff)
            .map(|(k, _)| k.clone())
            .collect();

        for id in &stale {
            if let Some(session) = self.sessions.remove(id) {
                kill_session_process(&session);
                info!(project_id = %id, "cleaned up stale chat session");
            }
        }

        if !stale.is_empty() {
            // Clean up message mappings for stale sessions
            self.message_to_project
                .retain(|_, pid| !stale.contains(pid));
        }
    }

    /// Kill all active Claude processes across all sessions.
    /// Called on daemon shutdown to prevent orphaned processes.
    pub fn kill_all(&mut self) {
        for (pid, session) in &mut self.sessions {
            if let Some(pgid) = session.active_pgid.take() {
                info!(project = %pid, pgid, "killing chat claude process group on shutdown");
                unsafe {
                    libc::killpg(pgid, libc::SIGTERM);
                }
                // Give it a moment, then force kill
                std::thread::sleep(Duration::from_millis(500));
                unsafe {
                    libc::killpg(pgid, libc::SIGKILL);
                }
            }
        }
    }

    /// Invoke Claude for a chat session and return the response text.
    pub async fn invoke_claude(
        &mut self,
        project_id: &str,
        prompt: &str,
        max_turns: u32,
        max_budget_usd: f64,
        chat_timeout: Duration,
    ) -> RexResult<String> {
        let session = self
            .sessions
            .get_mut(project_id)
            .ok_or_else(|| RexError::NotFound(format!("no chat session for {project_id}")))?;

        session.last_activity = Utc::now();

        let (response, new_session_id) = spawn_and_await(
            &session.project_dir,
            prompt,
            session.claude_session_id.as_deref(),
            &session.session_name,
            max_turns,
            max_budget_usd,
            &mut session.active_pgid,
            chat_timeout,
        )
        .await?;

        session.claude_session_id = Some(new_session_id);
        self.last_active = Some(project_id.to_string());
        Ok(response)
    }
}

/// Kill the process group for a session if one is active.
fn kill_session_process(session: &ChatSession) {
    if let Some(pgid) = session.active_pgid {
        warn!(
            project = %session.project_id,
            pgid,
            "killing orphaned chat claude process group"
        );
        unsafe {
            libc::killpg(pgid, libc::SIGTERM);
        }
        std::thread::sleep(Duration::from_millis(500));
        unsafe {
            libc::killpg(pgid, libc::SIGKILL);
        }
    }
}

/// Spawn Claude and wait for the response.
/// Tracks the process group ID via `active_pgid` so it can be killed on cleanup.
async fn spawn_and_await(
    project_dir: &Path,
    prompt: &str,
    session_id: Option<&str>,
    session_name: &str,
    max_turns: u32,
    max_budget_usd: f64,
    active_pgid: &mut Option<i32>,
    chat_timeout: Duration,
) -> RexResult<(String, String)> {
    let mut cmd = tokio::process::Command::new("claude");
    cmd.current_dir(project_dir);

    if let Some(sid) = session_id {
        cmd.arg("--resume").arg(sid);
    }

    cmd.arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("json")
        .arg("--model")
        .arg("sonnet[1m]")
        .arg("--effort")
        .arg("high")
        .arg("--dangerously-skip-permissions")
        .arg("--max-turns")
        .arg(max_turns.to_string())
        .arg("--max-budget-usd")
        .arg(format!("{max_budget_usd:.2}"))
        .arg("--append-system-prompt")
        .arg(CHAT_SYSTEM_PROMPT);

    if session_id.is_none() {
        cmd.arg("--name").arg(session_name);
    }

    cmd.env("REX_CHAT", "1");

    // Process group isolation: child becomes its own group leader.
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    info!(session_name, "spawning chat claude");
    let child = cmd.spawn().map_err(|e| {
        RexError::ClaudeProcess(format!("failed to spawn chat claude: {e}"))
    })?;

    // Track the process group so it can be killed on cleanup/shutdown.
    // PGID = PID because of setpgid(0, 0).
    let pid = child.id().unwrap_or(0);
    let pgid = pid as i32;
    *active_pgid = Some(pgid);
    info!(pid, pgid, "chat claude process started");

    let result = await_chat_claude(child, chat_timeout).await;

    // Clear the tracked PGID -- process is done.
    *active_pgid = None;

    result
}

async fn await_chat_claude(
    mut child: tokio::process::Child,
    timeout: Duration,
) -> RexResult<(String, String)> {
    let result = tokio::time::timeout(timeout, async {
        use tokio::io::AsyncReadExt;

        let mut stdout_handle = child.stdout.take();
        let mut stderr_handle = child.stderr.take();

        let (stdout_buf, stderr_buf) = tokio::join!(
            async {
                let mut buf = Vec::new();
                if let Some(ref mut s) = stdout_handle {
                    s.read_to_end(&mut buf).await.ok();
                }
                buf
            },
            async {
                let mut buf = Vec::new();
                if let Some(ref mut s) = stderr_handle {
                    s.read_to_end(&mut buf).await.ok();
                }
                buf
            },
        );

        let _status = child.wait().await?;
        Ok::<_, std::io::Error>((stdout_buf, stderr_buf))
    })
    .await;

    match result {
        Ok(Ok((stdout_buf, stderr_buf))) => {
            let stdout_str = String::from_utf8_lossy(&stdout_buf);
            let output: crate::autorun::types::ClaudeOutput =
                serde_json::from_str(&stdout_str).map_err(|e| {
                    let stderr_str = String::from_utf8_lossy(&stderr_buf);
                    error!(stderr = %stderr_str, "chat claude stderr");
                    RexError::JsonParse {
                        context: format!(
                            "chat output: {}",
                            stdout_str.chars().take(200).collect::<String>()
                        ),
                        source: e,
                    }
                })?;
            Ok((output.result, output.session_id))
        }
        Ok(Err(e)) => Err(RexError::ClaudeProcess(format!(
            "chat process error: {e}"
        ))),
        Err(_) => {
            // Timeout -- kill the process group
            if let Some(id) = child.id() {
                let pgid = id as i32;
                warn!(pgid, "chat claude timed out, killing process group");
                unsafe {
                    libc::killpg(pgid, libc::SIGTERM);
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                unsafe {
                    libc::killpg(pgid, libc::SIGKILL);
                }
            }
            Err(RexError::ClaudeProcess(format!(
                "chat claude timed out ({}m)",
                timeout.as_secs() / 60
            )))
        }
    }
}
