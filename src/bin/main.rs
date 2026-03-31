use clap::{Parser, Subcommand};
use console::style;
use rex::models::project_status::Status;

#[derive(Parser)]
#[command(name = "rex", about = "Rex project management CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage projects
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// Create a new project interactively
    Create,
    /// Display the current active project
    GetActive,
    /// Remove a project
    Remove {
        /// Project ID to remove
        id: String,
    },
    /// Activate an inactive project
    Activate {
        /// Project ID to activate
        id: String,
    },
    /// Update the active project's directory
    UpdateDirectory {
        /// New directory path
        directory: String,
    },
    /// Update the status of a project item
    UpdateStatus {
        /// Item name (e.g., "goal", "scope", "uat")
        item: String,
        /// New status
        #[arg(value_enum)]
        status: Status,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Project { action } => match action {
            ProjectAction::Create => rex::commands::project::create(),
            ProjectAction::GetActive => rex::commands::project::get_active(),
            ProjectAction::Remove { id } => rex::commands::project::remove(&id),
            ProjectAction::Activate { id } => rex::commands::project::activate(&id),
            ProjectAction::UpdateDirectory { directory } => {
                rex::commands::project::update_directory(&directory)
            }
            ProjectAction::UpdateStatus { item, status } => {
                rex::commands::project::update_status(&item, status)
            }
        },
    };

    if let Err(e) = result {
        eprintln!("\n  {} {e}", style("error:").red().bold());
        std::process::exit(1);
    }
}
