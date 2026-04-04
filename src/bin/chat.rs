//! Binary entry point for the `rex-chat` Telegram daemon.

use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use rex_cli::chat::daemon::Args;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    match rex_cli::chat::daemon::run(args).await {
        Ok(code) => code,
        Err(e) => {
            tracing::error!("fatal: {e}");
            ExitCode::from(1)
        }
    }
}
