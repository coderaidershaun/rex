use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum RexError {
    #[error("io error at {}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("walk failed at {}", path.display())]
    Walk {
        path: PathBuf,
        #[source]
        source: ignore::Error,
    },

    #[error("yaml error at {}", path.display())]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yml::Error,
    },

    #[error("json parse failed at {}", path.display())]
    JsonParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("json serialize failed")]
    JsonSerialize(#[from] serde_json::Error),

    #[error("inactive project already exists at {}", path.display())]
    SlugCollision { path: PathBuf },

    #[error("inactive project not found: {id}")]
    ProjectNotFound { id: String },

    #[error("no active project found at {}", path.display())]
    NoActiveProject { path: PathBuf },

    #[error("invalid project-id: {reason}")]
    InvalidProjectId { reason: String },

    #[error("pipeline yaml contains no steps")]
    EmptyPipeline,

    #[error("prompt cancelled by user")]
    PromptCancelled,

    #[error("bundle file not found: {}", path.display())]
    BundleFileNotFound { path: PathBuf },

    #[error("schedule file not found: {}", path.display())]
    ScheduleNotFound { path: PathBuf },

    #[error("schedule address not found: {addr}")]
    ScheduleAddrNotFound { addr: String },

    #[error("blocked-by cycle detected involving {addr}")]
    BlockedByCycle { addr: String },

    #[error("duplicate slug {addr}")]
    DuplicateSlug { addr: String },

    #[error("ambiguous address '{addr}' — matches: {candidates}")]
    AmbiguousAddr { addr: String, candidates: String },

    #[error("replace would regress state for: {offenders}")]
    ReplaceWouldRegressState { offenders: String },

    #[error("invalid slug: {reason}")]
    InvalidSlug { reason: String },
}
