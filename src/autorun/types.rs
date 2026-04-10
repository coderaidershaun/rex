//! Data types, serialization structures, and utilities shared across the autorun module.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Parsed from the last JSON object in Claude's `result` text.
#[derive(Debug, Deserialize)]
pub struct OperatorResult {
    pub status: OperatorStatus,
    #[serde(default)]
    pub message: String,
    /// Current work item name (e.g. "goal", "architecture", "t-token-endpoint").
    #[serde(default)]
    pub item: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorStatus {
    Completed,
    ProjectDone,
    NeedsInput,
    Error,
}

/// Top-level JSON returned by the agent CLI with `--output-format json`.
#[derive(Debug, Deserialize)]
pub struct AgentOutput {
    pub result: String,
    pub session_id: String,
    #[serde(default)]
    pub cost: AgentCost,
    #[serde(default)]
    pub total_cost_usd: f64,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub usage: AgentUsage,
    #[serde(default, rename = "modelUsage")]
    pub model_usage: HashMap<String, ModelUsageEntry>,
    #[serde(default)]
    pub fast_mode_state: String,
}

impl AgentOutput {
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

    /// Approximate context window usage as a percentage.
    ///
    /// `modelUsage` token counts are cumulative across all API calls in the
    /// session, so we can't get an exact snapshot. We approximate with:
    ///   `cache_creation_input_tokens + output_tokens`
    /// which represents the cached prefix (system prompt, project context)
    /// plus all generated assistant text that accumulates in the conversation.
    /// `cache_read_input_tokens` is excluded because it re-counts the same
    /// cached prefix on every turn.
    pub fn context_percent(&self) -> f64 {
        let Some(entry) = self.model_usage.values().next() else {
            return 0.0;
        };
        if entry.context_window == 0 {
            return 0.0;
        }
        let approx_context = entry.cache_creation_input_tokens + entry.output_tokens;
        (approx_context as f64 / entry.context_window as f64) * 100.0
    }

    /// Formatted stats line for Telegram messages (HTML).
    pub fn telegram_stats(&self) -> String {
        format!(
            "⚡ <code>{speed}</code>  ·  📊 <code>{ctx:.1}%</code> context",
            speed = self.speed(),
            ctx = self.context_percent(),
        )
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct AgentCost {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub total_cost: f64,
}

#[derive(Debug, Default, Deserialize)]
pub struct AgentUsage {
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

/// Divider line for Telegram messages.
pub const DIV: &str = "⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯";

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
    pub agent_pid: Option<u32>,
    pub agent_pgid: Option<i32>,
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

/// Maximum entries kept in the rolling stat windows.
const MAX_ROLLING_ENTRIES: usize = 50;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RunStats {
    pub invocations_completed: u32,
    pub items_completed: u32,
    pub total_cost_usd: f64,
    pub started_at: String,
    /// Rolling window of the last N context-usage percentages (one per invocation).
    #[serde(default)]
    pub context_percents: Vec<f64>,
    /// Rolling window of the last N session durations in milliseconds.
    #[serde(default)]
    pub session_durations_ms: Vec<u64>,
}

impl RunStats {
    pub fn push_context_percent(&mut self, pct: f64) {
        if self.context_percents.len() >= MAX_ROLLING_ENTRIES {
            self.context_percents.remove(0);
        }
        self.context_percents.push(pct);
    }

    pub fn push_session_duration_ms(&mut self, ms: u64) {
        if self.session_durations_ms.len() >= MAX_ROLLING_ENTRIES {
            self.session_durations_ms.remove(0);
        }
        self.session_durations_ms.push(ms);
    }

    pub fn last_context_percent(&self) -> Option<f64> {
        self.context_percents.last().copied()
    }

    pub fn avg_context_percent(&self) -> Option<f64> {
        if self.context_percents.is_empty() {
            return None;
        }
        Some(self.context_percents.iter().sum::<f64>() / self.context_percents.len() as f64)
    }

    pub fn last_session_duration_ms(&self) -> Option<u64> {
        self.session_durations_ms.last().copied()
    }

    pub fn avg_session_duration_ms(&self) -> Option<u64> {
        if self.session_durations_ms.is_empty() {
            return None;
        }
        Some(self.session_durations_ms.iter().sum::<u64>() / self.session_durations_ms.len() as u64)
    }
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
    KilledByUser {
        project_id: String,
        timestamp: String,
    },
    AuthRefresh {
        project_id: String,
        timestamp: String,
    },
}

/// Acknowledgment responses sent when a Telegram reply is received.
/// Provides instant feedback before Claude starts processing.
pub const ACK_RESPONSES: [&str; 50] = [
    "Copy that! Processing now...",
    "Got it! On it...",
    "Message received! Working on it...",
    "Roger that! Processing...",
    "Received! Putting the hamsters to work...",
    "Got your reply! Spinning up the gears...",
    "Acknowledged! Processing your input...",
    "Loud and clear! Getting to work...",
    "Reply captured! Crunching away...",
    "On it like a bonnet! Processing...",
    "Received loud and clear! Working...",
    "Copy copy! Processing now...",
    "Message in a bottle received! Working...",
    "Gotcha! Firing up the engines...",
    "Reply secured! Processing...",
    "Heard you! Computing away...",
    "Affirmative! Getting to work...",
    "Response captured! Processing...",
    "10-4 good buddy! On it...",
    "Read you 5 by 5! Processing...",
    "Message intercepted! Working on it...",
    "Input absorbed! Processing...",
    "Reply digested! Working...",
    "Signal received! Processing...",
    "Got your signal! Engines firing...",
    "Transmission received! Processing...",
    "Read and understood! Working...",
    "Input locked in! Processing...",
    "Reply banked! Getting to work...",
    "Confirmed! Processing your response...",
    "Logged and loaded! Working...",
    "Challenge accepted! Processing...",
    "Message decoded! On it...",
    "Consider it received! Processing...",
    "In the pipeline! Working...",
    "Captured! Processing your input...",
    "Response logged! Working on it...",
    "Game on! Processing...",
    "Received and caffeinated! Working...",
    "Your wish is my command! Processing...",
    "Locked in! Getting to work...",
    "Reply received! Neurons firing...",
    "Got the memo! Processing...",
    "Incoming processed! Working...",
    "Message secured! On it...",
    "Input acknowledged! Processing...",
    "Received! The wheels are turning...",
    "Copy that, processing at light speed...",
    "Reply in hand! Working on it...",
    "Noted and processing! Hang tight...",
];
