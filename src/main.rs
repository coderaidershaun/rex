use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{ArgGroup, Args, Parser, Subcommand};

use rex_cli::bundle::{Bundle, BundleMode};
use rex_cli::commands::schedule::{ChunkUpdateInput, TaskUpdateInput};
use rex_cli::commands::{activate, codebase, create, init, project, schedule};
use rex_cli::schedule::ScheduleState;

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

    /// Write a tree outline of the working directory to CODEBASE.md.
    Codebase,

    /// Create a new project interactively.
    Create,

    /// Activate an inactive project by ID.
    Activate {
        /// The project ID to activate.
        project_id: String,
    },

    /// Print active project information as JSON (agent-facing).
    Project {
        #[command(subcommand)]
        sub: Option<ProjectCommand>,
    },
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Print project metadata only (no steps).
    Meta,

    /// Update title / subtitle / description on the active project.
    Update(UpdateArgs),

    /// Print the first incomplete step, or mark it done with `step complete`.
    Step {
        #[command(subcommand)]
        sub: Option<StepCommand>,
    },

    /// Print the next pending chunk from schedule.json.
    ChunkNext,

    /// Print the most recently completed chunk from schedule.json.
    ChunkPrior,

    /// Operate on individual tasks.
    Task {
        #[command(subcommand)]
        sub: TaskCommand,
    },

    /// CRUD operations on schedule phases, chunks, and tasks.
    Schedule {
        #[command(subcommand)]
        sub: ScheduleCommand,
    },
}

#[derive(Args)]
#[command(group(
    ArgGroup::new("meta_fields")
        .required(true)
        .multiple(true)
        .args(["title", "subtitle", "description"])
))]
struct UpdateArgs {
    /// New title. Pass an empty string to clear.
    #[arg(long)]
    title: Option<String>,
    /// New subtitle. Pass an empty string to clear.
    #[arg(long)]
    subtitle: Option<String>,
    /// New description. Pass an empty string to clear.
    #[arg(long)]
    description: Option<String>,
}

#[derive(Subcommand)]
enum StepCommand {
    /// Mark the first incomplete step as done.
    Complete,
}

#[derive(Subcommand)]
enum TaskCommand {
    /// Mark the current task done in schedule.json and increment counters.
    Complete,
}

#[derive(Subcommand)]
enum ScheduleCommand {
    /// Print the full schedule as JSON.
    Show,

    /// Atomically replace the schedule from a JSON file.
    Replace {
        /// Path to the new schedule JSON file.
        #[arg(long)]
        file: PathBuf,
    },

    /// Operate on schedule phases.
    Phase {
        #[command(subcommand)]
        sub: PhaseCommand,
    },

    /// Operate on schedule chunks.
    Chunk {
        #[command(subcommand)]
        sub: ChunkCommand,
    },

    /// Operate on schedule tasks.
    Task {
        #[command(subcommand)]
        sub: ScheduleTaskCommand,
    },
}

#[derive(Subcommand)]
enum PhaseCommand {
    /// Append a new phase.
    Add {
        #[arg(long)]
        description: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        blocked_by: Vec<String>,
    },
    /// Update a phase by slug or dotted position.
    Update {
        addr: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        blocked_by: Vec<String>,
    },
    /// Remove a phase and all its chunks/tasks.
    Remove { addr: String },
    /// Move a phase to a 1-indexed position.
    Move {
        addr: String,
        #[arg(long)]
        to: usize,
    },
}

#[derive(Subcommand)]
enum ChunkCommand {
    /// Append a chunk to a phase.
    Add {
        #[arg(long)]
        phase: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        scenario: Vec<String>,
        #[arg(long)]
        spec_ref: Vec<String>,
        #[arg(long)]
        blocked_by: Vec<String>,
    },
    /// Update a chunk by slug or dotted position.
    Update {
        addr: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        scenario: Vec<String>,
        #[arg(long)]
        spec_ref: Vec<String>,
        #[arg(long)]
        blocked_by: Vec<String>,
    },
    /// Remove a chunk and all its tasks.
    Remove { addr: String },
    /// Move a chunk to a new position and optionally a new parent phase.
    Move {
        addr: String,
        #[arg(long)]
        to_phase: Option<String>,
        #[arg(long)]
        to: Option<usize>,
    },
}

#[derive(Subcommand)]
enum ScheduleTaskCommand {
    /// Append a task to a chunk.
    Add {
        #[arg(long)]
        chunk: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        skill: Option<String>,
        #[arg(long)]
        inputs: Option<String>,
        #[arg(long)]
        outputs: Option<String>,
    },
    /// Update a task by slug or dotted position.
    Update {
        addr: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        skill: Option<String>,
        #[arg(long)]
        inputs: Option<String>,
        #[arg(long)]
        outputs: Option<String>,
    },
    /// Remove a task.
    Remove { addr: String },
    /// Move a task to a new position and optionally a new parent chunk.
    Move {
        addr: String,
        #[arg(long)]
        to_chunk: Option<String>,
        #[arg(long)]
        to: Option<usize>,
    },
}

fn parse_state(s: &str) -> Result<ScheduleState, anyhow::Error> {
    s.parse::<ScheduleState>()
        .map_err(|e| anyhow::anyhow!("{e}"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = PathBuf::from(".");
    let bundle = Bundle::from_env();

    match cli.command {
        Command::Init { force } => {
            let mode = if force {
                BundleMode::Force
            } else {
                BundleMode::Merge
            };
            init::run(&cwd, &bundle, mode).context("rex init failed")?;
        }
        Command::Codebase => {
            codebase::run(&cwd).context("rex codebase failed")?;
        }
        Command::Create => {
            create::run(&cwd, &bundle).context("rex create failed")?;
        }
        Command::Activate { project_id } => {
            activate::run(&cwd, &project_id).context("rex activate failed")?;
        }
        Command::Project { sub } => match sub {
            None => project::run_show(&cwd).context("rex project failed")?,
            Some(ProjectCommand::Meta) => {
                project::run_meta(&cwd).context("rex project meta failed")?
            }
            Some(ProjectCommand::Update(UpdateArgs {
                title,
                subtitle,
                description,
            })) => project::run_update(&cwd, title, subtitle, description)
                .context("rex project update failed")?,
            Some(ProjectCommand::Step { sub: None }) => {
                project::run_step(&cwd).context("rex project step failed")?
            }
            Some(ProjectCommand::Step {
                sub: Some(StepCommand::Complete),
            }) => project::run_step_complete(&cwd).context("rex project step complete failed")?,
            Some(ProjectCommand::ChunkNext) => {
                project::run_chunk_next(&cwd).context("rex project chunk-next failed")?
            }
            Some(ProjectCommand::ChunkPrior) => {
                project::run_chunk_prior(&cwd).context("rex project chunk-prior failed")?
            }
            Some(ProjectCommand::Task {
                sub: TaskCommand::Complete,
            }) => project::run_task_complete(&cwd).context("rex project task complete failed")?,
            Some(ProjectCommand::Schedule { sub }) => {
                dispatch_schedule(&cwd, sub).context("rex project schedule failed")?
            }
        },
    }

    Ok(())
}

fn dispatch_schedule(cwd: &Path, sub: ScheduleCommand) -> Result<()> {
    match sub {
        ScheduleCommand::Show => {
            schedule::run_schedule_show(cwd).context("schedule show failed")?;
        }
        ScheduleCommand::Replace { file } => {
            schedule::run_schedule_replace(cwd, &file).context("schedule replace failed")?;
        }
        ScheduleCommand::Phase { sub } => match sub {
            PhaseCommand::Add {
                description,
                id,
                blocked_by,
            } => {
                schedule::run_phase_add(cwd, &description, id.as_deref(), &blocked_by)
                    .context("phase add failed")?;
            }
            PhaseCommand::Update {
                addr,
                description,
                id,
                state,
                blocked_by,
            } => {
                let state = state.as_deref().map(parse_state).transpose()?;
                let blocked_by_opt = if blocked_by.is_empty() {
                    None
                } else {
                    Some(blocked_by.as_slice())
                };
                schedule::run_phase_update(
                    cwd,
                    &addr,
                    description.as_deref(),
                    id.as_deref(),
                    state,
                    blocked_by_opt,
                )
                .context("phase update failed")?;
            }
            PhaseCommand::Remove { addr } => {
                schedule::run_phase_remove(cwd, &addr).context("phase remove failed")?;
            }
            PhaseCommand::Move { addr, to } => {
                schedule::run_phase_move(cwd, &addr, to).context("phase move failed")?;
            }
        },
        ScheduleCommand::Chunk { sub } => match sub {
            ChunkCommand::Add {
                phase,
                description,
                id,
                scenario,
                spec_ref,
                blocked_by,
            } => {
                schedule::run_chunk_add(
                    cwd,
                    &phase,
                    &description,
                    id.as_deref(),
                    &scenario,
                    &spec_ref,
                    &blocked_by,
                )
                .context("chunk add failed")?;
            }
            ChunkCommand::Update {
                addr,
                description,
                id,
                state,
                scenario,
                spec_ref,
                blocked_by,
            } => {
                let state = state.as_deref().map(parse_state).transpose()?;
                let scenario_opt = if scenario.is_empty() {
                    None
                } else {
                    Some(scenario.as_slice())
                };
                let spec_ref_opt = if spec_ref.is_empty() {
                    None
                } else {
                    Some(spec_ref.as_slice())
                };
                let blocked_by_opt = if blocked_by.is_empty() {
                    None
                } else {
                    Some(blocked_by.as_slice())
                };
                schedule::run_chunk_update(
                    cwd,
                    ChunkUpdateInput {
                        addr: &addr,
                        description: description.as_deref(),
                        new_id: id.as_deref(),
                        state,
                        scenarios: scenario_opt,
                        spec_refs: spec_ref_opt,
                        blocked_by: blocked_by_opt,
                    },
                )
                .context("chunk update failed")?;
            }
            ChunkCommand::Remove { addr } => {
                schedule::run_chunk_remove(cwd, &addr).context("chunk remove failed")?;
            }
            ChunkCommand::Move { addr, to_phase, to } => {
                schedule::run_chunk_move(cwd, &addr, to_phase.as_deref(), to)
                    .context("chunk move failed")?;
            }
        },
        ScheduleCommand::Task { sub } => match sub {
            ScheduleTaskCommand::Add {
                chunk,
                description,
                id,
                skill,
                inputs,
                outputs,
            } => {
                schedule::run_task_add(
                    cwd,
                    &chunk,
                    &description,
                    id.as_deref(),
                    skill.as_deref(),
                    inputs.as_deref(),
                    outputs.as_deref(),
                )
                .context("task add failed")?;
            }
            ScheduleTaskCommand::Update {
                addr,
                description,
                id,
                state,
                skill,
                inputs,
                outputs,
            } => {
                let state = state.as_deref().map(parse_state).transpose()?;
                schedule::run_task_update(
                    cwd,
                    TaskUpdateInput {
                        addr: &addr,
                        description: description.as_deref(),
                        new_id: id.as_deref(),
                        state,
                        skill: skill.map(Some),
                        inputs: inputs.map(Some),
                        outputs: outputs.map(Some),
                    },
                )
                .context("task update failed")?;
            }
            ScheduleTaskCommand::Remove { addr } => {
                schedule::run_task_remove(cwd, &addr).context("task remove failed")?;
            }
            ScheduleTaskCommand::Move { addr, to_chunk, to } => {
                schedule::run_task_move(cwd, &addr, to_chunk.as_deref(), to)
                    .context("task move failed")?;
            }
        },
    }
    Ok(())
}
