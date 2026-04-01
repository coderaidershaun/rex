use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A single entry in the recent or archived history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unique identifier for this entry (kebab-case).
    pub id: String,
    /// ISO-8601 timestamp of when this entry was recorded.
    pub timestamp: String,
    /// Brief summary of what was done.
    pub summary: String,
    /// Entities (milestone/objective/task IDs) that were affected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<String>,
    /// Files that were created or modified.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// The agent session identifier, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

/// Top-level history store with two sections.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
    /// Work from the last three agent sessions (detailed, recent).
    #[serde(default)]
    pub recent: Vec<HistoryEntry>,
    /// Compacted summaries of older work.
    #[serde(default)]
    pub archived: Vec<HistoryEntry>,
}

impl History {
    pub fn load(project_dir: &Path) -> Result<Self, String> {
        let path = project_dir.join("history.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read history.json: {e}"))?;
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse history.json: {e}"))
    }

    pub fn save(&self, project_dir: &Path) -> Result<(), String> {
        let path = project_dir.join("history.json");
        let json =
            serde_json::to_string_pretty(self).map_err(|e| format!("Failed to serialize: {e}"))?;
        fs::write(&path, format!("{json}\n"))
            .map_err(|e| format!("Failed to write history.json: {e}"))?;
        Ok(())
    }
}
