// Harness feature flag validation: exactly one of "claude" or "cursor" must be enabled.
#[cfg(all(feature = "claude", feature = "cursor"))]
compile_error!(
    "Features 'claude' and 'cursor' are mutually exclusive. \
     Use --no-default-features --features cursor to select the Cursor harness."
);

#[cfg(not(any(feature = "claude", feature = "cursor")))]
compile_error!(
    "Exactly one harness feature must be enabled: 'claude' or 'cursor'."
);

pub mod autorun;
pub mod chat;
pub mod commands;
pub mod errors;
pub mod models;
pub mod ui;
