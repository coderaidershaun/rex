use serde::{Deserialize, Serialize};

/// Parsed from the last JSON object in Claude's `result` text.
#[derive(Debug, Deserialize)]
pub struct OperatorResult {
    pub status: OperatorStatus,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorStatus {
    Completed,
    ProjectDone,
    NeedsInput,
    Error,
}

/// Top-level JSON returned by `claude -p --output-format json`.
#[derive(Debug, Deserialize)]
pub struct ClaudeOutput {
    pub result: String,
    pub session_id: String,
    #[serde(default)]
    pub cost: ClaudeCost,
    #[serde(default)]
    pub duration_ms: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct ClaudeCost {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub total_cost: f64,
}

/// Persisted to `.rex-autorun.json` for crash recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutorunState {
    pub phase: AutorunPhase,
    pub session_id: Option<String>,
    pub claude_pid: Option<u32>,
    pub claude_pgid: Option<i32>,
    pub pending_question: Option<String>,
    pub telegram_message_id: Option<i64>,
    /// Telegram `getUpdates` offset — persisted to avoid replaying stale messages.
    pub telegram_update_offset: Option<i64>,
    pub invocation_count: u32,
    pub updated_at: String,
    pub stats: RunStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutorunPhase {
    Running,
    PendingInput,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RunStats {
    pub invocations_completed: u32,
    pub items_completed: u32,
    pub total_cost_usd: f64,
    pub started_at: String,
}

/// JSONL log events written to `.rex-autorun.log`.
#[derive(Serialize)]
#[serde(tag = "event")]
pub enum LogEvent {
    Started {
        project_id: String,
        timestamp: String,
    },
    InvocationStarted {
        n: u32,
        timestamp: String,
    },
    InvocationCompleted {
        n: u32,
        status: String,
        message: String,
        session_id: String,
        cost_usd: f64,
        duration_ms: u64,
        timestamp: String,
    },
    NeedsInput {
        question: String,
        session_id: String,
        timestamp: String,
    },
    InputReceived {
        reply_length: usize,
        timestamp: String,
    },
    Error {
        message: String,
        retryable: bool,
        attempt: u32,
        timestamp: String,
    },
    ProjectDone {
        total_cost_usd: f64,
        total_invocations: u32,
        total_duration: String,
        timestamp: String,
    },
}
