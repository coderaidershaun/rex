use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use rex_cli::autorun::runner::Args;

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize tracing: human-readable to stderr, controlled by RUST_LOG env
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    match rex_cli::autorun::runner::run(args).await {
        Ok(code) => code,
        Err(e) => {
            tracing::error!("fatal: {e}");
            ExitCode::from(1)
        }
    }
}
