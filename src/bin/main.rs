use clap::{Args, Parser, Subcommand};
use console::style;
use rex_cli::models::checklist::{ChecklistCategory, Phase};
use rex_cli::models::planning::{ListMods, PlanningStatus};
use rex_cli::models::project::Category;
use rex_cli::models::project::Complexity;
use rex_cli::models::project_status::Status;

#[derive(Parser)]
#[command(name = "rex", about = "Rex project management CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the rex harness in the current directory
    Init {
        /// Use Claude Code (skip interactive prompt)
        #[arg(long)]
        claude: bool,
        /// Use Cursor (skip interactive prompt)
        #[arg(long)]
        cursor: bool,
    },
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
    /// Manage milestones (major checkpoints in the project journey)
    Milestone {
        #[command(subcommand)]
        action: MilestoneAction,
    },
    /// Manage objectives (strategic outcomes beneath milestones)
    Objective {
        #[command(subcommand)]
        action: ObjectiveAction,
    },
    /// Manage tasks (atomic units of work beneath objectives)
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    /// Manage session history (recent and archived work)
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },
}

// ---------------------------------------------------------------------------
// Shared args for list-field modifications (used by milestone/objective/task)
// ---------------------------------------------------------------------------

#[derive(Args)]
struct ListModArgs {
    /// Add a reference (file path, URL, or entity ID). Repeatable.
    #[arg(long = "add-reference")]
    add_references: Vec<String>,

    /// Remove a reference. Repeatable.
    #[arg(long = "remove-reference")]
    remove_references: Vec<String>,

    /// Add an output path. Repeatable.
    #[arg(long = "add-output")]
    add_outputs: Vec<String>,

    /// Remove an output path. Repeatable.
    #[arg(long = "remove-output")]
    remove_outputs: Vec<String>,

    /// Add an upstream dependency (same-type entity ID). Repeatable.
    #[arg(long = "add-upstream")]
    add_upstream: Vec<String>,

    /// Remove an upstream dependency. Repeatable.
    #[arg(long = "remove-upstream")]
    remove_upstream: Vec<String>,

    /// Add a downstream dependency (same-type entity ID). Repeatable.
    #[arg(long = "add-downstream")]
    add_downstream: Vec<String>,

    /// Remove a downstream dependency. Repeatable.
    #[arg(long = "remove-downstream")]
    remove_downstream: Vec<String>,

    /// Add a checklist item in "id:text" format. Repeatable.
    #[arg(long = "add-checklist", value_name = "ID:TEXT")]
    add_checklist: Vec<String>,

    /// Remove a checklist item by ID. Repeatable.
    #[arg(long = "remove-checklist")]
    remove_checklist: Vec<String>,

    /// Mark a checklist item as done by ID. Repeatable.
    #[arg(long)]
    check: Vec<String>,

    /// Mark a checklist item as not done by ID. Repeatable.
    #[arg(long)]
    uncheck: Vec<String>,
}

impl From<ListModArgs> for ListMods {
    fn from(a: ListModArgs) -> Self {
        Self {
            add_references: a.add_references,
            remove_references: a.remove_references,
            add_outputs: a.add_outputs,
            remove_outputs: a.remove_outputs,
            add_upstream: a.add_upstream,
            remove_upstream: a.remove_upstream,
            add_downstream: a.add_downstream,
            remove_downstream: a.remove_downstream,
            add_checklist: a.add_checklist,
            remove_checklist: a.remove_checklist,
            check: a.check,
            uncheck: a.uncheck,
        }
    }
}

// ---------------------------------------------------------------------------
// Project subcommands (unchanged)
// ---------------------------------------------------------------------------

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
    /// Update the active project's category
    UpdateCategory {
        /// New category
        #[arg(value_enum)]
        category: Category,
    },
    /// Update the active project's complexity
    UpdateComplexity {
        /// New complexity
        #[arg(value_enum)]
        complexity: Complexity,
    },
    /// Get the next actionable item from the project status
    NextItem,
}

// ---------------------------------------------------------------------------
// Checklist subcommands (unchanged)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Milestone subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum MilestoneAction {
    /// Create or update a milestone. On create, --title and --description are required.
    Upsert {
        /// Unique kebab-case identifier
        #[arg(long)]
        id: String,
        /// Milestone title
        #[arg(long)]
        title: Option<String>,
        /// Milestone description
        #[arg(long)]
        description: Option<String>,
        /// Status
        #[arg(long, value_enum)]
        status: Option<PlanningStatus>,
        #[command(flatten)]
        mods: ListModArgs,
    },
    /// Get a milestone by ID (JSON output)
    Get {
        /// Milestone ID
        id: String,
    },
    /// List milestones with optional status filter (JSON output)
    List {
        /// Filter by status
        #[arg(long, value_enum)]
        status: Option<PlanningStatus>,
    },
    /// Remove a milestone by ID
    Remove {
        /// Milestone ID
        id: String,
    },
}

// ---------------------------------------------------------------------------
// Objective subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum ObjectiveAction {
    /// Create or update an objective. On create, --milestone, --title, and --description are required.
    Upsert {
        /// Unique kebab-case identifier
        #[arg(long)]
        id: String,
        /// Parent milestone ID (required on create, optional on update to re-parent)
        #[arg(long)]
        milestone: Option<String>,
        /// Objective title
        #[arg(long)]
        title: Option<String>,
        /// Objective description
        #[arg(long)]
        description: Option<String>,
        /// Status
        #[arg(long, value_enum)]
        status: Option<PlanningStatus>,
        #[command(flatten)]
        mods: ListModArgs,
    },
    /// Get an objective by ID (JSON output)
    Get {
        /// Objective ID
        id: String,
    },
    /// List objectives with optional filters (JSON output)
    List {
        /// Filter by parent milestone ID
        #[arg(long)]
        milestone: Option<String>,
        /// Filter by status
        #[arg(long, value_enum)]
        status: Option<PlanningStatus>,
    },
    /// Remove an objective by ID
    Remove {
        /// Objective ID
        id: String,
    },
}

// ---------------------------------------------------------------------------
// Task subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum TaskAction {
    /// Create or update a task. On create, --objective, --title, and --description are required.
    Upsert {
        /// Unique kebab-case identifier
        #[arg(long)]
        id: String,
        /// Parent objective ID (required on create, optional on update to re-parent)
        #[arg(long)]
        objective: Option<String>,
        /// Task title
        #[arg(long)]
        title: Option<String>,
        /// Task description
        #[arg(long)]
        description: Option<String>,
        /// Status
        #[arg(long, value_enum)]
        status: Option<PlanningStatus>,
        #[command(flatten)]
        mods: ListModArgs,
    },
    /// Get a task by ID (JSON output)
    Get {
        /// Task ID
        id: String,
    },
    /// List tasks with optional filters (JSON output)
    List {
        /// Filter by parent objective ID
        #[arg(long)]
        objective: Option<String>,
        /// Filter by status
        #[arg(long, value_enum)]
        status: Option<PlanningStatus>,
    },
    /// Remove a task by ID
    Remove {
        /// Task ID
        id: String,
    },
    /// Get the next task to work on based on dependency ordering and priority
    Next,
}

// ---------------------------------------------------------------------------
// History subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum HistoryAction {
    /// Insert a new entry into the recent section
    InsertRecent {
        /// Unique kebab-case identifier for this entry
        #[arg(long)]
        id: String,
        /// ISO-8601 timestamp (e.g. 2026-04-01T14:30:00Z)
        #[arg(long)]
        timestamp: String,
        /// Brief summary of what was done
        #[arg(long)]
        summary: String,
        /// Entity IDs (milestones/objectives/tasks) affected. Repeatable.
        #[arg(long = "entity")]
        entities: Vec<String>,
        /// Files created or modified. Repeatable.
        #[arg(long = "file")]
        files: Vec<String>,
        /// Agent session identifier
        #[arg(long)]
        session: Option<String>,
    },
    /// Remove an entry from the recent section
    RemoveFromRecent {
        /// Entry ID to remove
        id: String,
    },
    /// Insert a compacted entry into the archived section
    InsertCompacted {
        /// Unique kebab-case identifier for this entry
        #[arg(long)]
        id: String,
        /// ISO-8601 timestamp (e.g. 2026-04-01T14:30:00Z)
        #[arg(long)]
        timestamp: String,
        /// Compacted summary of the work
        #[arg(long)]
        summary: String,
        /// Entity IDs (milestones/objectives/tasks) affected. Repeatable.
        #[arg(long = "entity")]
        entities: Vec<String>,
        /// Files created or modified. Repeatable.
        #[arg(long = "file")]
        files: Vec<String>,
        /// Agent session identifier
        #[arg(long)]
        session: Option<String>,
    },
    /// Remove an entry from the archived section
    RemoveFromCompacted {
        /// Entry ID to remove
        id: String,
    },
    /// Get just the recent entries as JSON
    GetRecent,
    /// List all history (recent and archived) as JSON
    List,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        // -- Init -----------------------------------------------------------
        Commands::Init { claude, cursor } => {
            let agent_os = if claude {
                Some(rex_cli::commands::init::AgentOs::Claude)
            } else if cursor {
                Some(rex_cli::commands::init::AgentOs::Cursor)
            } else {
                None
            };
            rex_cli::commands::init::init(agent_os)
        }

        // -- Project --------------------------------------------------------
        Commands::Project { action } => match action {
            ProjectAction::Create => rex_cli::commands::project::create(),
            ProjectAction::GetActive => rex_cli::commands::project::get_active(),
            ProjectAction::Remove { id } => rex_cli::commands::project::remove(&id),
            ProjectAction::Activate { id } => rex_cli::commands::project::activate(&id),
            ProjectAction::UpdateDirectory { directory } => {
                rex_cli::commands::project::update_directory(&directory)
            }
            ProjectAction::UpdateStatus { item, status } => {
                rex_cli::commands::project::update_status(&item, status)
            }
            ProjectAction::UpdateTitle { title } => rex_cli::commands::project::update_title(&title),
            ProjectAction::UpdateSubtitle { subtitle } => {
                rex_cli::commands::project::update_subtitle(&subtitle)
            }
            ProjectAction::UpdateDescription { description } => {
                rex_cli::commands::project::update_description(&description)
            }
            ProjectAction::UpdateCategory { category } => {
                rex_cli::commands::project::update_category(category)
            }
            ProjectAction::UpdateComplexity { complexity } => {
                rex_cli::commands::project::update_complexity(complexity)
            }
            ProjectAction::NextItem => rex_cli::commands::project::next_item(),
        },

        // -- Checklist ------------------------------------------------------
        Commands::Checklist { action } => match action {
            ChecklistAction::Init { date } => rex_cli::commands::checklist::init(date),
            ChecklistAction::Add {
                category,
                id,
                title,
                description,
                phase,
            } => rex_cli::commands::checklist::add(category, &id, &title, &description, phase),
            ChecklistAction::List {
                category,
                phase,
                complete,
                incomplete,
            } => rex_cli::commands::checklist::list(category, phase, complete, incomplete),
            ChecklistAction::Get { id } => rex_cli::commands::checklist::get(&id),
            ChecklistAction::Update {
                id,
                title,
                description,
                phase,
            } => rex_cli::commands::checklist::update(&id, title, description, phase),
            ChecklistAction::Complete { id } => rex_cli::commands::checklist::complete(&id),
            ChecklistAction::Uncomplete { id } => rex_cli::commands::checklist::uncomplete(&id),
            ChecklistAction::Remove { id } => rex_cli::commands::checklist::remove(&id),
            ChecklistAction::SetContext { context } => {
                rex_cli::commands::checklist::set_context(&context)
            }
        },

        // -- Milestone ------------------------------------------------------
        Commands::Milestone { action } => match action {
            MilestoneAction::Upsert {
                id,
                title,
                description,
                status,
                mods,
            } => rex_cli::commands::milestone::upsert(&id, title, description, status, mods.into()),
            MilestoneAction::Get { id } => rex_cli::commands::milestone::get(&id),
            MilestoneAction::List { status } => rex_cli::commands::milestone::list(status),
            MilestoneAction::Remove { id } => rex_cli::commands::milestone::remove(&id),
        },

        // -- Objective ------------------------------------------------------
        Commands::Objective { action } => match action {
            ObjectiveAction::Upsert {
                id,
                milestone,
                title,
                description,
                status,
                mods,
            } => rex_cli::commands::objective::upsert(
                &id, milestone, title, description, status, mods.into(),
            ),
            ObjectiveAction::Get { id } => rex_cli::commands::objective::get(&id),
            ObjectiveAction::List { milestone, status } => {
                rex_cli::commands::objective::list(milestone, status)
            }
            ObjectiveAction::Remove { id } => rex_cli::commands::objective::remove(&id),
        },

        // -- Task -----------------------------------------------------------
        Commands::Task { action } => match action {
            TaskAction::Upsert {
                id,
                objective,
                title,
                description,
                status,
                mods,
            } => {
                rex_cli::commands::task::upsert(&id, objective, title, description, status, mods.into())
            }
            TaskAction::Get { id } => rex_cli::commands::task::get(&id),
            TaskAction::List { objective, status } => {
                rex_cli::commands::task::list(objective, status)
            }
            TaskAction::Remove { id } => rex_cli::commands::task::remove(&id),
            TaskAction::Next => rex_cli::commands::task::next(),
        },

        // -- History --------------------------------------------------------
        Commands::History { action } => match action {
            HistoryAction::InsertRecent {
                id,
                timestamp,
                summary,
                entities,
                files,
                session,
            } => rex_cli::commands::history::insert_recent(
                &id, &timestamp, &summary, entities, files, session,
            ),
            HistoryAction::RemoveFromRecent { id } => {
                rex_cli::commands::history::remove_from_recent(&id)
            }
            HistoryAction::InsertCompacted {
                id,
                timestamp,
                summary,
                entities,
                files,
                session,
            } => rex_cli::commands::history::insert_compacted(
                &id, &timestamp, &summary, entities, files, session,
            ),
            HistoryAction::RemoveFromCompacted { id } => {
                rex_cli::commands::history::remove_from_compacted(&id)
            }
            HistoryAction::GetRecent => rex_cli::commands::history::get_recent(),
            HistoryAction::List => rex_cli::commands::history::list(),
        },
    };

    if let Err(e) = result {
        eprintln!("\n  {} {e}", style("error:").red().bold());
        std::process::exit(1);
    }
}
