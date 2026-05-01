use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum RexError {
    #[error("io error at {}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("yaml parse failed at {}", path.display())]
    YamlParse {
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
    JsonSerialize(#[source] serde_json::Error),

    #[error("inactive project already exists at {}", path.display())]
    SlugCollision { path: PathBuf },

    #[error("inactive project not found: {id}")]
    ProjectNotFound { id: String },

    #[error("no active project found at {}", path.display())]
    NoActiveProject { path: PathBuf },

    #[error("active project has no project-id field")]
    MissingProjectId,

    #[error("pipeline yaml contains no steps")]
    EmptyPipeline,

    #[error("prompt cancelled by user")]
    PromptCancelled,

    #[error("bundle file not found: {}", path.display())]
    BundleFileNotFound { path: PathBuf },
}
