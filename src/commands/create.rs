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

/// One API entry collected when the user selects the `research-api` optional step.
pub struct ResearchApiRow {
    /// Short display name, e.g. `"Binance Spot"`.
    pub name: String,
    /// Docs URL the research skill will explore.
    pub url: String,
}

/// One external resource collected when the user selects the `resources` optional step.
pub struct ResourceRow {
    /// Human label, e.g. `"Design spec"`.
    pub label: String,
    /// URL or file path to the resource.
    pub url: String,
}

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
    /// APIs to research, collected when the user selects `research-api`.
    pub research_apis: Vec<ResearchApiRow>,
    /// External resources, collected when the user selects `resources`.
    pub resources: Vec<ResourceRow>,
    /// `true` runs the pipeline straight through; `false` pauses between steps
    /// and chunks so the user can review intermediate output.
    pub is_autopilot: bool,
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

    // Prompt order matches the pipeline execution order: resources runs before research-api.
    let resources = if selected.iter().any(|s| s == "resources") {
        prompt_resource_rows()?
    } else {
        Vec::new()
    };

    let research_apis = if selected.iter().any(|s| s == "research-api") {
        prompt_research_api_rows()?
    } else {
        Vec::new()
    };

    let is_autopilot = Confirm::new(
        "Run pipeline on autopilot? (no = pause for review between steps and chunks)",
    )
    .with_default(false)
    .prompt()
    .map_err(|_| RexError::PromptCancelled)?;

    let opts = CreateOpts {
        title,
        subtitle,
        description,
        category,
        complexity,
        project_id,
        selected_optional_steps: selected,
        research_apis,
        resources,
        is_autopilot,
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
        is_autopilot: opts.is_autopilot,
        steps,
    };

    store.write_active(&project)?;

    if !opts.research_apis.is_empty() {
        let body = render_research_api_md(&opts.research_apis);
        store.write_active_subfile("research", "apis.md", body.as_bytes())?;
    }
    if !opts.resources.is_empty() {
        let body = render_resources_md(&opts.resources);
        store.write_active_subfile("resources", "urls.md", body.as_bytes())?;
    }

    println!(
        "Created project '{}' at rex/active/project.yaml",
        opts.project_id
    );
    Ok(())
}

// Best-effort markdown rendering: row fields land in a bullet line as-is. Newlines are
// stripped (they would break bullet structure); other markdown metacharacters in user
// names/URLs are not escaped — downstream skills consume this as plain prose.
fn sanitize_field(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

fn render_research_api_md(rows: &[ResearchApiRow]) -> String {
    let mut out = String::from("# APIs to research\n");
    for row in rows {
        out.push_str(&format!(
            "\n- **{}** — {}",
            sanitize_field(&row.name),
            sanitize_field(&row.url)
        ));
    }
    out.push('\n');
    out
}

fn render_resources_md(rows: &[ResourceRow]) -> String {
    let mut out = String::from("# Resources\n");
    for row in rows {
        out.push_str(&format!(
            "\n- **{}** — {}",
            sanitize_field(&row.label),
            sanitize_field(&row.url)
        ));
    }
    out.push('\n');
    out
}

fn prompt_research_api_rows() -> Result<Vec<ResearchApiRow>, RexError> {
    println!("Add APIs to research. Leave name empty to finish.");
    let mut rows = Vec::new();
    loop {
        let name = Text::new("API name:")
            .prompt()
            .map_err(|_| RexError::PromptCancelled)?;
        if name.trim().is_empty() {
            break;
        }
        let url = Text::new("API docs URL:")
            .prompt()
            .map_err(|_| RexError::PromptCancelled)?;
        // Drop rows with a blank URL rather than persisting half-empty bullets;
        // re-prompting would balloon the loop into a state machine for thin payoff.
        if url.trim().is_empty() {
            println!("(skipped — blank URL)");
            continue;
        }
        rows.push(ResearchApiRow {
            name: name.trim().to_owned(),
            url: url.trim().to_owned(),
        });
    }
    Ok(rows)
}

fn prompt_resource_rows() -> Result<Vec<ResourceRow>, RexError> {
    println!("Add resources. Leave label empty to finish.");
    let mut rows = Vec::new();
    loop {
        let label = Text::new("Resource label:")
            .prompt()
            .map_err(|_| RexError::PromptCancelled)?;
        if label.trim().is_empty() {
            break;
        }
        let url = Text::new("Resource URL:")
            .prompt()
            .map_err(|_| RexError::PromptCancelled)?;
        if url.trim().is_empty() {
            println!("(skipped — blank URL)");
            continue;
        }
        rows.push(ResourceRow {
            label: label.trim().to_owned(),
            url: url.trim().to_owned(),
        });
    }
    Ok(rows)
}
