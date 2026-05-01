use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use rex_cli::bundle::Bundle;
use rex_cli::commands::{activate, create, init};

#[derive(Parser)]
#[command(name = "rex", version, about = "Rex project harness manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract or update the .claude/ bundle in the current directory.
    Init {
        /// Overwrite all files regardless of user modifications.
        #[arg(long)]
        force: bool,
    },

    /// Create a new project interactively.
    Create,

    /// Activate an inactive project by ID.
    Activate {
        /// The project ID to activate.
        project_id: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = PathBuf::from(".");
    let bundle = Bundle::from_env();

    match cli.command {
        Command::Init { force } => {
            init::run(&cwd, &bundle, force).context("rex init failed")?;
        }
        Command::Create => {
            create::run(&cwd, &bundle).context("rex create failed")?;
        }
        Command::Activate { project_id } => {
            activate::run(&cwd, &project_id).context("rex activate failed")?;
        }
    }

    Ok(())
}
