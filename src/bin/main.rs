use clap::{Parser, Subcommand};
use console::style;

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
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Project { action } => match action {
            ProjectAction::Create => rex::commands::project::create(),
            ProjectAction::GetActive => rex::commands::project::get_active(),
        },
    };

    if let Err(e) = result {
        eprintln!("\n  {} {e}", style("error:").red().bold());
        std::process::exit(1);
    }
}
