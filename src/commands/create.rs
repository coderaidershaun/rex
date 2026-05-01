use std::path::Path;

use inquire::{Confirm, Select, Text};
use slug::slugify;

use crate::{
    bundle::Bundle,
    error::RexError,
    project::{
        ProjectId, ProjectYaml, archive_active, has_active_project, list_inactive, parse_pipeline,
        prune_steps, write_active_project,
    },
};

/// Options collected from the interactive prompts, ready for non-interactive use in tests.
pub struct CreateOpts {
    /// Human-readable project title.
    pub title: String,
    /// Optional subtitle.
    pub subtitle: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Project category, e.g. `feature`, `refactor`.
    pub category: String,
    /// Project complexity, e.g. `low`, `medium`, `high`.
    pub complexity: String,
    /// Slug identifier for the project.
    pub project_id: ProjectId,
    /// Names of optional pipeline steps the user opted into.
    pub selected_optional_steps: Vec<String>,
}

/// Run `rex create` against `cwd` interactively.
///
/// # Errors
/// - [`RexError::PromptCancelled`] if user exits prompts
/// - [`RexError::SlugCollision`] if the chosen project-id already exists in inactive
/// - [`RexError::Io`] for filesystem failures
pub fn run(cwd: &Path, bundle: &Bundle) -> Result<(), RexError> {
    let pipeline_bytes = bundle.read_file(Path::new("rex/pipeline.yaml"))?;
    let pipeline_yaml = String::from_utf8_lossy(&pipeline_bytes);
    let template = parse_pipeline(&pipeline_yaml)?;

    let required_steps: Vec<_> = template
        .steps
        .iter()
        .filter(|s| s.required)
        .map(|s| s.step.as_str())
        .collect();

    let optional_steps: Vec<_> = template
        .steps
        .iter()
        .filter(|s| !s.required)
        .map(|s| s.step.clone())
        .collect();

    println!("Always included: {}", required_steps.join(", "));

    let title = Text::new("Project title:")
        .prompt()
        .map_err(|_| RexError::PromptCancelled)?;

    let default_id = slugify(&title);

    let subtitle_raw = Text::new("Subtitle (optional, enter to skip):")
        .prompt()
        .map_err(|_| RexError::PromptCancelled)?;
    let subtitle = if subtitle_raw.trim().is_empty() {
        None
    } else {
        Some(subtitle_raw.trim().to_owned())
    };

    let description_raw = Text::new("Description (optional, enter to skip):")
        .prompt()
        .map_err(|_| RexError::PromptCancelled)?;
    let description = if description_raw.trim().is_empty() {
        None
    } else {
        Some(description_raw.trim().to_owned())
    };

    let category = Select::new(
        "Category:",
        vec!["feature", "refactor", "spike", "research"],
    )
    .with_starting_cursor(0)
    .prompt()
    .map_err(|_| RexError::PromptCancelled)?
    .to_owned();

    let complexity = Select::new("Complexity:", vec!["low", "medium", "high"])
        .with_starting_cursor(1)
        .prompt()
        .map_err(|_| RexError::PromptCancelled)?
        .to_owned();

    let project_id = ProjectId::new(
        Text::new("Project ID:")
            .with_default(&default_id)
            .prompt()
            .map_err(|_| RexError::PromptCancelled)?,
    );

    let optional_step_refs: Vec<&str> = optional_steps.iter().map(String::as_str).collect();
    let selected: Vec<String> = if optional_steps.is_empty() {
        Vec::new()
    } else {
        let chosen = inquire::MultiSelect::new("Select optional steps:", optional_step_refs)
            .prompt()
            .map_err(|_| RexError::PromptCancelled)?;
        chosen.into_iter().map(str::to_owned).collect()
    };

    let opts = CreateOpts {
        title,
        subtitle,
        description,
        category,
        complexity,
        project_id,
        selected_optional_steps: selected,
    };

    apply_create(cwd, &template, opts)
}

/// Apply a create operation with the given opts. Separated from prompts for testability.
///
/// # Errors
/// - [`RexError::SlugCollision`] if project-id already exists in inactive
/// - [`RexError::Io`] for filesystem failures
pub fn apply_create(
    cwd: &Path,
    template: &crate::project::PipelineTemplate,
    opts: CreateOpts,
) -> Result<(), RexError> {
    let inactive_target = cwd.join("rex/inactive").join(opts.project_id.as_str());
    if inactive_target.exists() {
        return Err(RexError::SlugCollision {
            path: inactive_target,
        });
    }

    let inactive_ids = list_inactive(cwd)?;
    if inactive_ids.contains(&opts.project_id) {
        return Err(RexError::SlugCollision {
            path: cwd.join("rex/inactive").join(opts.project_id.as_str()),
        });
    }

    if has_active_project(cwd) {
        let confirm = Confirm::new("Archive existing active project?")
            .with_default(true)
            .prompt()
            .map_err(|_| RexError::PromptCancelled)?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
        let archived_id = archive_active(cwd)?;
        println!("Archived active project to rex/inactive/{archived_id}");
    }

    let selected_refs: Vec<&str> = opts
        .selected_optional_steps
        .iter()
        .map(String::as_str)
        .collect();
    let steps = prune_steps(template, &selected_refs);

    let project = ProjectYaml {
        project_id: opts.project_id.clone(),
        category: opts.category,
        title: Some(opts.title),
        subtitle: opts.subtitle,
        description: opts.description,
        complexity: opts.complexity,
        chunks_required: 0,
        chunks_completed: 0,
        tasks_required: 0,
        tasks_completed: 0,
        completed: false,
        steps,
    };

    write_active_project(cwd, &project)?;

    println!(
        "Created project '{}' at rex/active/project.yaml",
        opts.project_id
    );
    Ok(())
}
