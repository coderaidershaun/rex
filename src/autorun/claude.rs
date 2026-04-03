use std::path::Path;
use std::time::Duration;

use crate::errors::{RexError, RexResult};
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info, warn};

use super::types::{ClaudeOutput, OperatorResult};

/// System prompt appended to every `claude -p` invocation.
pub const AUTORUN_SYSTEM_PROMPT: &str = r#"You are running inside the rex-autorun headless harness. The environment variable REX_AUTORUN=1 is set.

CRITICAL: When your work for this invocation is complete, you MUST output exactly one JSON object
as the VERY LAST LINE of your response. Nothing may follow it.

Use one of these four statuses:

Completed work (more items may remain in project-status.json):
{"status": "completed", "message": "<1-sentence summary of what was done>"}

Project fully complete (no more work items exist):
{"status": "project_done", "message": "All items completed."}

Blocked on human input (interactive item — onboarding, user-acceptance, etc.):
{"status": "needs_input", "message": "<the exact question the user must answer>"}

Error (unrecoverable problem):
{"status": "error", "message": "<what went wrong>"}"#;

/// A spawned claude process, ready to be awaited.
///
/// Split from `await_claude` so the caller can write the PID to the
/// state file between spawn and await — enabling orphan cleanup on crash.
pub struct SpawnedClaude {
    pub child: tokio::process::Child,
    pub pid: u32,
    pub pgid: i32,
}

/// Spawn `claude -p` with process group isolation.
///
/// Returns immediately with the child handle and PID/PGID. Call
/// `await_claude()` to wait for completion and parse output.
pub fn spawn_claude(
    project_dir: &Path,
    prompt: &str,
    session_id_to_resume: Option<&str>,
    session_name: &str,
    max_turns: u32,
    max_budget_usd: f64,
) -> RexResult<SpawnedClaude> {
    let mut cmd = tokio::process::Command::new("claude");
    cmd.current_dir(project_dir);

    if let Some(sid) = session_id_to_resume {
        cmd.arg("--resume").arg(sid);
    }

    cmd.arg("-p").arg(prompt);
    cmd.arg("--output-format").arg("json");
    cmd.arg("--dangerously-skip-permissions");
    cmd.arg("--max-turns").arg(max_turns.to_string());
    cmd.arg("--max-budget-usd").arg(format!("{max_budget_usd:.2}"));
    cmd.arg("--append-system-prompt").arg(AUTORUN_SYSTEM_PROMPT);

    if session_id_to_resume.is_none() {
        cmd.arg("--name").arg(session_name);
    }

    // Set REX_AUTORUN=1 so the operator skill detects headless mode.
    cmd.env("REX_AUTORUN", "1");

    // Process group isolation: child becomes its own group leader.
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    info!("spawning claude process");
    let child = cmd
        .spawn()
        .map_err(|e| RexError::ClaudeProcess(format!("failed to spawn claude process: {e}")))?;

    let pid = child.id().unwrap_or(0);
    let pgid = pid as i32; // group leader = same as PID due to setpgid(0,0)
    info!(pid, pgid, "claude process started");

    Ok(SpawnedClaude { child, pid, pgid })
}

/// Await a spawned claude process with timeout, returning parsed output.
pub async fn await_claude(
    spawned: SpawnedClaude,
    timeout: Duration,
) -> RexResult<(ClaudeOutput, i32)> {
    let mut child = spawned.child;
    let pgid = spawned.pgid;

    // Read stdout and stderr concurrently to avoid pipe deadlock.
    // If we read sequentially (stdout then stderr), a child that fills the
    // stderr pipe buffer (~64KB) before finishing stdout will block, while
    // we block reading stdout — classic deadlock.
    let result = tokio::time::timeout(timeout, async {
        let mut stdout_handle = child.stdout.take();
        let mut stderr_handle = child.stderr.take();

        let (stdout_buf, stderr_buf) = tokio::join!(
            async {
                let mut buf = Vec::new();
                if let Some(ref mut stdout) = stdout_handle {
                    stdout.read_to_end(&mut buf).await.ok();
                }
                buf
            },
            async {
                let mut buf = Vec::new();
                if let Some(ref mut stderr) = stderr_handle {
                    stderr.read_to_end(&mut buf).await.ok();
                }
                buf
            },
        );

        let status = child.wait().await?;
        Ok::<_, RexError>((status, stdout_buf, stderr_buf))
    })
    .await;

    match result {
        Ok(Ok((status, stdout_buf, stderr_buf))) => {
            let stdout_str = String::from_utf8_lossy(&stdout_buf);
            let stderr_str = String::from_utf8_lossy(&stderr_buf);

            if !stderr_str.is_empty() {
                debug!("claude stderr: {}", stderr_str.chars().take(500).collect::<String>());
            }

            if !status.success() {
                let code = status.code().unwrap_or(-1);
                if is_auth_error(&stderr_str) {
                    return Err(RexError::AuthExpired(
                        stderr_str.chars().take(500).collect::<String>()
                    ));
                }
                if is_retryable(&stderr_str) {
                    return Err(RexError::ClaudeProcess(format!(
                        "claude exited with code {code} (retryable): {}",
                        stderr_str.chars().take(200).collect::<String>()
                    )));
                }
                return Err(RexError::ClaudeProcess(format!(
                    "claude exited with code {code}: {}",
                    stderr_str.chars().take(200).collect::<String>()
                )));
            }

            let output: ClaudeOutput = serde_json::from_str(&stdout_str)
                .map_err(|e| RexError::JsonParse {
                    context: format!(
                        "claude JSON output (first 300 chars): {}",
                        stdout_str.chars().take(300).collect::<String>()
                    ),
                    source: e,
                })?;

            Ok((output, pgid))
        }
        Ok(Err(e)) => {
            // Process error (not timeout)
            error!("claude process error: {e}");
            kill_process_group(pgid).await;
            Err(RexError::ClaudeProcess(format!("process error: {e}")))
        }
        Err(_) => {
            // Timeout
            warn!(pgid, "claude process timed out, killing process group");
            kill_process_group(pgid).await;
            Err(RexError::ClaudeProcess(format!(
                "claude process timed out after {} minutes",
                timeout.as_secs() / 60
            )))
        }
    }
}

/// Parse the `OperatorResult` from the result text.
///
/// Searches backward for `{"status":` and parses the JSON object from that position.
pub fn parse_operator_result(result_text: &str) -> RexResult<OperatorResult> {
    let marker = r#"{"status":"#;
    let pos = result_text
        .rfind(marker)
        .ok_or_else(|| {
            RexError::ClaudeProcess(
                "no operator result JSON found in claude output (missing {\"status\":)".into(),
            )
        })?;

    let from_marker = &result_text[pos..];

    // Find the closing brace. We need to handle nested braces in the message field.
    // Simple approach: try parsing increasing substrings ending at each `}`.
    let mut depth = 0i32;
    let mut end = None;
    for (i, ch) in from_marker.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    let end = end.ok_or_else(|| {
        RexError::ClaudeProcess("no matching closing brace for operator result JSON".into())
    })?;
    let json_str = &from_marker[..end];

    serde_json::from_str::<OperatorResult>(json_str).map_err(|e| RexError::JsonParse {
        context: format!("operator result JSON: {json_str}"),
        source: e,
    })
}

/// Kill a process group: SIGTERM, wait 5s, then SIGKILL.
pub async fn kill_process_group(pgid: i32) {
    if pgid <= 0 {
        return;
    }
    info!(pgid, "sending SIGTERM to process group");
    #[cfg(unix)]
    unsafe {
        libc::killpg(pgid, libc::SIGTERM);
    }
    tokio::time::sleep(Duration::from_secs(5)).await;
    #[cfg(unix)]
    unsafe {
        libc::killpg(pgid, libc::SIGKILL);
    }
    info!(pgid, "sent SIGKILL to process group");
}

/// Check if a stderr message indicates an expired OAuth token.
pub fn is_auth_error(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("authentication_error")
        || lower.contains("oauth token has expired")
        || (lower.contains("401") && (lower.contains("token") || lower.contains("auth")))
}

/// Check if a stderr message indicates a retryable error.
pub fn is_retryable(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("rate limit")
        || lower.contains("overloaded")
        || lower.contains("connection")
        || lower.contains("timeout")
}
