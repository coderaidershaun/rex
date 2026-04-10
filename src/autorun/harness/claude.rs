//! Claude Code CLI: process spawning and output parsing.

use std::path::Path;
use std::time::Duration;

use crate::autorun::types::AgentOutput;
use crate::errors::{RexError, RexResult};
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info, warn};

use super::shared::{SpawnedAgent, is_auth_error, is_retryable, kill_process_group};

/// Spawn `claude -p` with process group isolation.
pub fn spawn_agent(
    project_dir: &Path,
    prompt: &str,
    system_prompt: &str,
    model: &str,
    session_id_to_resume: Option<&str>,
    session_name: &str,
    max_turns: u32,
    max_budget_usd: f64,
) -> RexResult<SpawnedAgent> {
    let mut cmd = tokio::process::Command::new("claude");
    cmd.current_dir(project_dir);

    if let Some(sid) = session_id_to_resume {
        cmd.arg("--resume").arg(sid);
    }

    cmd.arg("-p").arg(prompt);
    cmd.arg("--output-format").arg("json");
    cmd.arg("--model").arg(model);
    cmd.arg("--effort").arg("high");
    cmd.arg("--dangerously-skip-permissions");
    cmd.arg("--max-turns").arg(max_turns.to_string());
    cmd.arg("--max-budget-usd").arg(format!("{max_budget_usd:.2}"));
    cmd.arg("--append-system-prompt").arg(system_prompt);

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

    info!("spawning agent process");
    let child = cmd
        .spawn()
        .map_err(|e| RexError::AgentProcess(format!("failed to spawn agent process: {e}")))?;

    let pid = child.id().unwrap_or(0);
    let pgid = pid as i32;
    info!(pid, pgid, "agent process started");

    Ok(SpawnedAgent { child, pid, pgid })
}

/// Await a spawned agent process with timeout, returning parsed output.
pub async fn await_agent(
    spawned: SpawnedAgent,
    timeout: Duration,
) -> RexResult<(AgentOutput, i32)> {
    let mut child = spawned.child;
    let pgid = spawned.pgid;

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
                debug!("agent stderr: {}", stderr_str.chars().take(500).collect::<String>());
            }

            if !status.success() {
                let code = status.code().unwrap_or(-1);
                if is_auth_error(&stderr_str) {
                    return Err(RexError::AuthExpired(
                        stderr_str.chars().take(500).collect::<String>()
                    ));
                }
                if is_retryable(&stderr_str) {
                    return Err(RexError::AgentProcess(format!(
                        "agent exited with code {code} (retryable): {}",
                        stderr_str.chars().take(200).collect::<String>()
                    )));
                }
                return Err(RexError::AgentProcess(format!(
                    "agent exited with code {code}: {}",
                    stderr_str.chars().take(200).collect::<String>()
                )));
            }

            let output: AgentOutput = serde_json::from_str(&stdout_str)
                .map_err(|e| RexError::JsonParse {
                    context: format!(
                        "agent JSON output (first 300 chars): {}",
                        stdout_str.chars().take(300).collect::<String>()
                    ),
                    source: e,
                })?;

            Ok((output, pgid))
        }
        Ok(Err(e)) => {
            error!("agent process error: {e}");
            kill_process_group(pgid).await;
            Err(RexError::AgentProcess(format!("process error: {e}")))
        }
        Err(_) => {
            warn!(pgid, "agent process timed out, killing process group");
            kill_process_group(pgid).await;
            Err(RexError::AgentProcess(format!(
                "agent process timed out after {} minutes",
                timeout.as_secs() / 60
            )))
        }
    }
}
