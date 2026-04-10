//! Shared functions used by both Claude and Cursor harness implementations.

use std::time::Duration;

use crate::errors::{RexError, RexResult};
use tracing::info;

use crate::autorun::types::OperatorResult;

/// System prompt appended to every agent invocation.
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

/// A spawned agent process, ready to be awaited.
///
/// Split from `await_agent` so the caller can write the PID to the
/// state file between spawn and await — enabling orphan cleanup on crash.
pub struct SpawnedAgent {
    pub child: tokio::process::Child,
    pub pid: u32,
    pub pgid: i32,
}

/// Parse the `OperatorResult` from the result text.
///
/// Searches backward for `{"status":` and parses the JSON object from that position.
pub fn parse_operator_result(result_text: &str) -> RexResult<OperatorResult> {
    let marker = r#"{"status":"#;
    let pos = result_text
        .rfind(marker)
        .ok_or_else(|| {
            RexError::AgentProcess(
                "no operator result JSON found in agent output (missing {\"status\":)".into(),
            )
        })?;

    let from_marker = &result_text[pos..];

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
        RexError::AgentProcess("no matching closing brace for operator result JSON".into())
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
