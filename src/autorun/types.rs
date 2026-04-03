use std::collections::HashMap;

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
    pub total_cost_usd: f64,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub usage: ClaudeUsage,
    #[serde(default, rename = "modelUsage")]
    pub model_usage: HashMap<String, ModelUsageEntry>,
    #[serde(default)]
    pub fast_mode_state: String,
}

impl ClaudeOutput {
    /// Effective cost — prefers top-level `total_cost_usd` over nested `cost.total_cost`.
    pub fn effective_cost(&self) -> f64 {
        if self.total_cost_usd > 0.0 {
            self.total_cost_usd
        } else {
            self.cost.total_cost
        }
    }

    /// Model name extracted from `modelUsage` keys.
    pub fn model_name(&self) -> &str {
        self.model_usage
            .keys()
            .next()
            .map(|s| s.as_str())
            .unwrap_or("unknown")
    }

    /// Speed / thinking mode (e.g. "standard", "fast").
    pub fn speed(&self) -> &str {
        if !self.usage.speed.is_empty() {
            &self.usage.speed
        } else if self.fast_mode_state == "on" {
            "fast"
        } else {
            "standard"
        }
    }

    /// Context window usage as a percentage.
    pub fn context_percent(&self) -> f64 {
        if let Some(entry) = self.model_usage.values().next() {
            if entry.context_window > 0 {
                let total = entry.input_tokens
                    + entry.output_tokens
                    + entry.cache_read_input_tokens
                    + entry.cache_creation_input_tokens;
                (total as f64 / entry.context_window as f64) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Formatted header line for Telegram messages (HTML).
    pub fn telegram_header(&self) -> String {
        format!(
            "<b>{}</b>  |  {}  |  {:.1}% context",
            escape_html(self.model_name()),
            self.speed(),
            self.context_percent(),
        )
    }
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

#[derive(Debug, Default, Deserialize)]
pub struct ClaudeUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub speed: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ModelUsageEntry {
    #[serde(default, rename = "inputTokens")]
    pub input_tokens: u64,
    #[serde(default, rename = "outputTokens")]
    pub output_tokens: u64,
    #[serde(default, rename = "cacheReadInputTokens")]
    pub cache_read_input_tokens: u64,
    #[serde(default, rename = "cacheCreationInputTokens")]
    pub cache_creation_input_tokens: u64,
    #[serde(default, rename = "contextWindow")]
    pub context_window: u64,
    #[serde(default, rename = "maxOutputTokens")]
    pub max_output_tokens: u64,
    #[serde(default, rename = "costUSD")]
    pub cost_usd: f64,
}

/// Escape HTML special characters for Telegram HTML parse mode.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
