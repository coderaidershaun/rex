use std::path::Path;

use inquire::{Confirm, Select, Text};
use slug::slugify;

use crate::{
    bundle::Bundle,
    error::RexError,
    project::{
        PipelineTemplate, ProjectId, ProjectStore, ProjectYaml, parse_pipeline, prune_steps,
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
/// - [`RexError::InvalidProjectId`] if the entered project-id is empty/whitespace-only
/// - [`RexError::SlugCollision`] if the chosen project-id already exists in inactive
/// - [`RexError::Io`] for filesystem failures
/// - [`RexError::Yaml`] if the embedded pipeline YAML is malformed
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

    let project_id = ProjectId::parse(
        Text::new("Project ID:")
            .with_default(&default_id)
            .prompt()
            .map_err(|_| RexError::PromptCancelled)?,
    )?;

    let optional_step_refs: Vec<&str> = optional_steps.iter().map(String::as_str).collect();
    let selected: Vec<String> = if optional_steps.is_empty() {
        Vec::new()
    } else {
        let all_indices: Vec<usize> = (0..optional_step_refs.len()).collect();
        let chosen = inquire::MultiSelect::new("Select optional steps:", optional_step_refs)
            .with_default(&all_indices)
            .with_page_size(optional_steps.len().max(1))
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
    template: &PipelineTemplate,
    opts: CreateOpts,
) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);

    let inactive_ids = store.list_inactive()?;
    if inactive_ids.contains(&opts.project_id) {
        return Err(RexError::SlugCollision {
            path: cwd.join("rex/inactive").join(opts.project_id.as_str()),
        });
    }

    if store.has_active() {
        let confirm = Confirm::new("Archive existing active project?")
            .with_default(true)
            .prompt()
            .map_err(|_| RexError::PromptCancelled)?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
        let archived_id = store.archive_active()?;
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

    store.write_active(&project)?;

    println!(
        "Created project '{}' at rex/active/project.yaml",
        opts.project_id
    );
    Ok(())
}
