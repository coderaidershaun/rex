use clap::{Parser, Subcommand};
use console::style;
use rex::models::checklist::{ChecklistCategory, Phase};
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
    /// Manage checklist items
    Checklist {
        #[command(subcommand)]
        action: ChecklistAction,
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
    /// Update the active project's title
    UpdateTitle {
        /// New title
        title: String,
    },
    /// Update the active project's subtitle
    UpdateSubtitle {
        /// New subtitle
        subtitle: String,
    },
    /// Update the active project's description
    UpdateDescription {
        /// New description
        description: String,
    },
}

#[derive(Subcommand)]
enum ChecklistAction {
    /// Initialize an empty checklist.json for the active project
    Init {
        /// Date for the checklist (defaults to today, YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
    },
    /// Add a new item to the checklist
    Add {
        /// Category to add the item to
        #[arg(long, value_enum)]
        category: ChecklistCategory,
        /// Unique kebab-case identifier
        #[arg(long)]
        id: String,
        /// Short actionable title
        #[arg(long)]
        title: String,
        /// Description with source references
        #[arg(long)]
        description: String,
        /// Phase: design or planning (required except for out-of-scope)
        #[arg(long, value_enum)]
        phase: Option<Phase>,
    },
    /// List checklist items with optional filters
    List {
        /// Filter by category
        #[arg(long, value_enum)]
        category: Option<ChecklistCategory>,
        /// Filter by phase
        #[arg(long, value_enum)]
        phase: Option<Phase>,
        /// Show only complete items
        #[arg(long)]
        complete: bool,
        /// Show only incomplete items
        #[arg(long)]
        incomplete: bool,
    },
    /// Get a specific checklist item by ID
    Get {
        /// Item ID
        id: String,
    },
    /// Update a checklist item's fields
    Update {
        /// Item ID
        id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New description
        #[arg(long)]
        description: Option<String>,
        /// New phase
        #[arg(long, value_enum)]
        phase: Option<Phase>,
    },
    /// Mark a checklist item as complete
    Complete {
        /// Item ID
        id: String,
    },
    /// Mark a checklist item as incomplete
    Uncomplete {
        /// Item ID
        id: String,
    },
    /// Remove a checklist item
    Remove {
        /// Item ID
        id: String,
    },
    /// Set the checklist context text
    SetContext {
        /// Context text describing how the checklist was derived
        context: String,
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
            ProjectAction::UpdateTitle { title } => {
                rex::commands::project::update_title(&title)
            }
            ProjectAction::UpdateSubtitle { subtitle } => {
                rex::commands::project::update_subtitle(&subtitle)
            }
            ProjectAction::UpdateDescription { description } => {
                rex::commands::project::update_description(&description)
            }
        },
        Commands::Checklist { action } => match action {
            ChecklistAction::Init { date } => rex::commands::checklist::init(date),
            ChecklistAction::Add {
                category,
                id,
                title,
                description,
                phase,
            } => rex::commands::checklist::add(category, &id, &title, &description, phase),
            ChecklistAction::List {
                category,
                phase,
                complete,
                incomplete,
            } => rex::commands::checklist::list(category, phase, complete, incomplete),
            ChecklistAction::Get { id } => rex::commands::checklist::get(&id),
            ChecklistAction::Update {
                id,
                title,
                description,
                phase,
            } => rex::commands::checklist::update(&id, title, description, phase),
            ChecklistAction::Complete { id } => rex::commands::checklist::complete(&id),
            ChecklistAction::Uncomplete { id } => rex::commands::checklist::uncomplete(&id),
            ChecklistAction::Remove { id } => rex::commands::checklist::remove(&id),
            ChecklistAction::SetContext { context } => {
                rex::commands::checklist::set_context(&context)
            }
        },
    };

    if let Err(e) = result {
        eprintln!("\n  {} {e}", style("error:").red().bold());
        std::process::exit(1);
    }
}
