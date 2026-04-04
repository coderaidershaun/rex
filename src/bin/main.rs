use clap::{Args, Parser, Subcommand};
use console::style;
use rex_cli::models::checklist::{ChecklistCategory, Phase};
use rex_cli::models::planning::{ListMods, PlanningStatus};
use rex_cli::models::project::{Category, Complexity, RepoVisibility};
use rex_cli::models::project_status::Status;

#[derive(Parser)]
#[command(name = "rex", about = "Rex project management CLI", version, after_long_help = COMMANDS_HELP)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// List all commands and subcommands
    #[arg(long)]
    commands: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the rex harness in the current directory
    Init,
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
    /// Create a Cargo workspace monorepo with git
    Mono(MonoArgs),
    /// Manage session history (recent and archived work)
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },
    /// Autorun management
    Autorun {
        #[command(subcommand)]
        action: AutorunAction,
    },
}

#[derive(Subcommand)]
enum AutorunAction {
    /// List running autoruns across all registered projects
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    Create {
        /// Create a GitHub repository (public or private) using the gh CLI
        #[arg(long, value_enum)]
        with_git_repo: Option<RepoVisibility>,
    },
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
    /// Update the active project's fields
    Update {
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New subtitle
        #[arg(long)]
        subtitle: Option<String>,
        /// New description
        #[arg(long)]
        description: Option<String>,
        /// New directory path
        #[arg(long)]
        directory: Option<String>,
        /// New category
        #[arg(long, value_enum)]
        category: Option<Category>,
        /// New complexity
        #[arg(long, value_enum)]
        complexity: Option<Complexity>,
    },
    /// Update the status of a project item
    UpdateStatus {
        /// Item name (e.g., "goal", "scope", "uat")
        item: String,
        /// New status
        #[arg(value_enum)]
        status: Status,
    },
    /// Get the next actionable item from the project status
    NextItem,
    /// Lock the active project
    Lock,
    /// Unlock the active project
    Unlock,
    /// Get project completion percentage (JSON output)
    GetCompletionPercent,
    /// Read and consume user-provided input for the active project
    GetUserInput,
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
        /// Agent model (e.g. opus, sonnet, haiku)
        #[arg(long)]
        agent_model: Option<String>,
        /// Agent effort level (e.g. medium, high, max, ultrathink)
        #[arg(long)]
        agent_effort: Option<String>,
        /// Agent skill(s) to invoke. Repeatable.
        #[arg(long = "agent-skill")]
        agent_skills: Vec<String>,
        /// Agent count (number of agents to spawn)
        #[arg(long)]
        agent_count: Option<u32>,
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
// Mono args
// ---------------------------------------------------------------------------

#[derive(Args)]
struct MonoArgs {
    /// Name of the monorepo directory to create
    #[arg(long)]
    name: String,
    /// Create an empty workspace (no rex harness or claude folders)
    #[arg(long)]
    no_harness: bool,
    /// Create a GitHub repository (public or private) using the gh CLI
    #[arg(long, value_enum)]
    with_git_repo: Option<RepoVisibility>,
}

// ---------------------------------------------------------------------------
// History subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum HistoryAction {
    /// Insert a new history entry (recent by default, archived with --archived)
    Insert {
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
        /// Insert into the archived section instead of recent
        #[arg(long)]
        archived: bool,
    },
    /// Remove a history entry (recent by default, archived with --archived)
    Remove {
        /// Entry ID to remove
        id: String,
        /// Remove from the archived section instead of recent
        #[arg(long)]
        archived: bool,
    },
    /// Get just the recent entries as JSON
    GetRecent,
    /// List all history (recent and archived) as JSON
    List,
}

// ---------------------------------------------------------------------------
// Full command reference (shown by --commands and --help)
// NOTE: Update COMMANDS_HELP whenever commands are added, removed, or renamed.
// ---------------------------------------------------------------------------

const COMMANDS_HELP: &str = "\
All commands:

  rex init                                  Initialize the rex harness in the current directory

  rex project create [--with-git-repo <public|private>]
                                            Create a new project interactively
  rex project get-active                    Display the current active project
  rex project remove <id>                   Remove a project
  rex project activate <id>                 Activate an inactive project
  rex project next-item                     Get the next actionable item from project status
  rex project lock                          Lock the active project
  rex project unlock                        Unlock the active project
  rex project update [--title <t>] [--subtitle <s>] [--description <d>] [--directory <p>]
      [--category <c>] [--complexity <x>]   Update the active project's fields
  rex project update-status <item> <status> Update the status of a project item
  rex project get-completion-percent        Get project completion percentage (JSON)
  rex project get-user-input                Read and consume user-provided input

  rex checklist init [--date <YYYY-MM-DD>]  Initialize an empty checklist for the active project
  rex checklist add --category <cat> --id <id> --title <t> --description <d> [--phase <p>]
                                            Add a new checklist item
  rex checklist list [--category <cat>] [--phase <p>] [--complete] [--incomplete]
                                            List checklist items with optional filters
  rex checklist get <id>                    Get a specific checklist item
  rex checklist update <id> [--title <t>] [--description <d>] [--phase <p>]
                                            Update a checklist item
  rex checklist complete <id>               Mark a checklist item as complete
  rex checklist uncomplete <id>             Mark a checklist item as incomplete
  rex checklist remove <id>                 Remove a checklist item
  rex checklist set-context <text>          Set the checklist context text

  rex milestone upsert --id <id> [--title <t>] [--description <d>] [--status <s>]
                                            Create or update a milestone
  rex milestone get <id>                    Get a milestone by ID
  rex milestone list [--status <s>]         List milestones
  rex milestone remove <id>                 Remove a milestone

  rex objective upsert --id <id> [--milestone <m>] [--title <t>] [--description <d>] [--status <s>]
                                            Create or update an objective
  rex objective get <id>                    Get an objective by ID
  rex objective list [--milestone <m>] [--status <s>]
                                            List objectives
  rex objective remove <id>                 Remove an objective

  rex task upsert --id <id> [--objective <o>] [--title <t>] [--description <d>] [--status <s>]
                                            Create or update a task
  rex task get <id>                         Get a task by ID
  rex task list [--objective <o>] [--status <s>]
                                            List tasks
  rex task remove <id>                      Remove a task
  rex task next                             Get the next task based on dependency order and priority

  rex mono --name <name> [--no-harness] [--with-git-repo <public|private>]
                                            Create a Cargo workspace monorepo with git

  rex history insert --id <id> --timestamp <ts> --summary <s> [--entity <e>...] [--file <f>...] [--archived]
                                            Insert a history entry (recent by default, archived with --archived)
  rex history remove <id> [--archived]      Remove a history entry (recent by default, archived with --archived)
  rex history get-recent                    Get recent history entries as JSON
  rex history list                          List all history (recent and archived) as JSON

  rex autorun list [--json]                 List running autoruns across all registered projects

Autorun (separate binary — headless autopilot, uses REX_AUTORUN_TELEGRAM_BOT_TOKEN):

  rex-autorun [OPTIONS]                     Run the active project autonomously
      --project-dir <PATH>                  Rex project root (default: current directory)
      --max-budget-usd <AMT>               Max USD per invocation (default: 50)
      --max-total-budget-usd <AMT>         Hard stop for total spend (default: 500)
      --max-turns <N>                       Max agentic turns per invocation (default: 200)
      --process-timeout-mins <N>            Max minutes per Claude process (default: 60)
      --max-retries <N>                     Max retries for transient failures (default: 5)
      --human-timeout-days <N>              Max days to wait for Telegram reply (default: 1)
      --log-file <PATH>                     JSONL log file (default: .rex-autorun.log)

  Telegram commands (send to bot while autorun is running):
      /kill <project-id>                    Terminate the autorun session
      /query <project-id>                   Show live stats and other running autoruns

Rex Chat (separate binary — Telegram chat interface, uses REX_AUTOCHAT_TELEGRAM_BOT_TOKEN):

  rex-chat [OPTIONS]                        Telegram chat daemon for all rex projects
      --scan-dir <PATH>                     Root directory to scan for projects (default: $HOME)
      --max-budget-usd <AMT>               Budget per chat invocation (default: 10)
      --max-turns <N>                       Max turns per chat invocation (default: 50)
      --session-timeout-mins <N>            Idle session timeout (default: 30)

  Telegram commands (send to chat bot):
      <message>                              Chat with active project (or only project)
      <id>: <message>                        Chat with a specific project
      /chat <id>                             Switch active project
      /start <id>                            Start autorun for a project
      /stop <id>                             Stop autorun for a project
      /status [id]                           Show autorun status (all if no id)
      /projects                              List all discovered projects
      /menu                                  Show project dashboard with buttons
      /commands                              Show command help
      /clear                                 Clear chat history

  Background usage (recommended):
      nohup rex-autorun --project-dir /absolute/path/to/project > /dev/null 2>&1 &";

fn print_commands() {
    println!();
    println!("  {}  v{}", style("Rex CLI").bold().cyan(), env!("CARGO_PKG_VERSION"));
    println!("  {}", style("\u{2500}".repeat(40)).dim());
    println!();
    for line in COMMANDS_HELP.lines().skip(1) {
        if line.is_empty() {
            println!();
        } else {
            println!("  {line}");
        }
    }
    println!();
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    if cli.commands {
        print_commands();
        return;
    }

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            // No subcommand given — show help
            Cli::parse_from(["rex", "--help"]);
            return;
        }
    };

    let result = match command {
        // -- Init -----------------------------------------------------------
        Commands::Init => rex_cli::commands::init::init(),

        // -- Project --------------------------------------------------------
        Commands::Project { action } => match action {
            ProjectAction::Create { with_git_repo } => rex_cli::commands::project::create(with_git_repo),
            ProjectAction::GetActive => rex_cli::commands::project::get_active(),
            ProjectAction::Remove { id } => rex_cli::commands::project::remove(&id),
            ProjectAction::Activate { id } => rex_cli::commands::project::activate(&id),
            ProjectAction::Update {
                title,
                subtitle,
                description,
                directory,
                category,
                complexity,
            } => rex_cli::commands::project::update(
                title, subtitle, description, directory, category, complexity,
            ),
            ProjectAction::UpdateStatus { item, status } => {
                rex_cli::commands::project::update_status(&item, status)
            }
            ProjectAction::NextItem => rex_cli::commands::project::next_item(),
            ProjectAction::Lock => rex_cli::commands::project::lock(),
            ProjectAction::Unlock => rex_cli::commands::project::unlock(),
            ProjectAction::GetCompletionPercent => {
                rex_cli::commands::project::get_completion_percent()
            }
            ProjectAction::GetUserInput => rex_cli::commands::project::get_user_input(),
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
                agent_model,
                agent_effort,
                agent_skills,
                agent_count,
                mods,
            } => {
                rex_cli::commands::task::upsert(
                    &id, objective, title, description, status,
                    agent_model, agent_effort, agent_skills, agent_count,
                    mods.into(),
                )
            }
            TaskAction::Get { id } => rex_cli::commands::task::get(&id),
            TaskAction::List { objective, status } => {
                rex_cli::commands::task::list(objective, status)
            }
            TaskAction::Remove { id } => rex_cli::commands::task::remove(&id),
            TaskAction::Next => rex_cli::commands::task::next(),
        },

        // -- Mono -----------------------------------------------------------
        Commands::Mono(args) => rex_cli::commands::mono::create(&args.name, args.no_harness, args.with_git_repo),

        // -- History --------------------------------------------------------
        Commands::History { action } => match action {
            HistoryAction::Insert {
                id,
                timestamp,
                summary,
                entities,
                files,
                session,
                archived,
            } => rex_cli::commands::history::insert(
                &id, &timestamp, &summary, entities, files, session, archived,
            ),
            HistoryAction::Remove { id, archived } => {
                rex_cli::commands::history::remove(&id, archived)
            }
            HistoryAction::GetRecent => rex_cli::commands::history::get_recent(),
            HistoryAction::List => rex_cli::commands::history::list(),
        },
        Commands::Autorun { action } => match action {
            AutorunAction::List { json } => rex_cli::commands::autorun::list(json),
        },
    };

    if let Err(e) = result {
        eprintln!("\n  {} {e}", style("error:").red().bold());
        std::process::exit(1);
    }
}
