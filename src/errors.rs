use std::io;

#[derive(Debug, thiserror::Error)]
pub enum RexError {
    // --- File I/O with context ---
    #[error("failed to read {path}: {source}")]
    FileRead { path: String, source: io::Error },

    #[error("failed to write {path}: {source}")]
    FileWrite { path: String, source: io::Error },

    #[error("failed to create directory {path}: {source}")]
    DirCreate { path: String, source: io::Error },

    // --- Bare I/O (terminal, crossterm, env, etc.) ---
    #[error(transparent)]
    Io(#[from] io::Error),

    // --- JSON with context ---
    #[error("failed to parse {context}: {source}")]
    JsonParse {
        context: String,
        source: serde_json::Error,
    },

    #[error("failed to serialize {context}: {source}")]
    JsonSerialize {
        context: String,
        source: serde_json::Error,
    },

    // --- Bare JSON (stdout serialization, etc.) ---
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    // --- Domain ---
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    AlreadyExists(String),

    #[error("{0}")]
    Validation(String),

    // --- Subprocess ---
    #[error("{command} failed: {detail}")]
    Subprocess { command: String, detail: String },

    // --- UI ---
    #[error(transparent)]
    Dialoguer(#[from] dialoguer::Error),

    // --- Autorun: Environment ---
    #[error("{name} not set: {detail}")]
    EnvVar { name: String, detail: String },

    // --- Autorun: HTTP ---
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    // --- Autorun: Telegram ---
    #[error("telegram: {0}")]
    Telegram(String),

    // --- Autorun: Claude process ---
    #[error("claude process: {0}")]
    ClaudeProcess(String),

    // --- Autorun: Killed via /kill command ---
    #[error("killed via /kill command")]
    Killed,

    // --- Cancelled (user Ctrl+C in UI) ---
    #[error("Cancelled")]
    Cancelled,
}

pub type RexResult<T> = Result<T, RexError>;
