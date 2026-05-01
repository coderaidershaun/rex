use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::RexError;

/// Slug identifier for a project, e.g. `my-cool-feature`.
///
/// Wrapping the raw slug as a newtype prevents it from being mixed with
/// other strings (titles, categories, file paths) at API boundaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// Wrap an existing slug. Caller is responsible for slug validity.
    pub fn new(slug: impl Into<String>) -> Self {
        Self(slug.into())
    }

    /// The slug as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `true` when the slug is the empty string.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for ProjectId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ProjectId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// One step in a project pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct PipelineStep {
    /// Step identifier, e.g. `discovery`, `prd`.
    pub step: String,
    /// `true` when the step must run for every project.
    pub required: bool,
    /// Skill the step invokes, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    /// Agent the step invokes, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Free-form per-step instructions for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Vec<String>>,
    /// Path glob describing what the step reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<String>,
    /// Path glob describing what the step writes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<String>,
    /// Marks the step as already executed for this project.
    pub completed: bool,
}

/// Active or inactive project file (`project.yaml`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectYaml {
    /// Slug identifier.
    pub project_id: ProjectId,
    /// Project category, e.g. `feature`, `refactor`.
    pub category: String,
    /// Human-readable title.
    pub title: Option<String>,
    /// Optional one-liner under the title.
    pub subtitle: Option<String>,
    /// Free-form long description.
    pub description: Option<String>,
    /// Project complexity, e.g. `low`, `medium`, `high`.
    pub complexity: String,
    /// Number of chunks declared for the project.
    pub chunks_required: u32,
    /// Number of chunks marked completed.
    pub chunks_completed: u32,
    /// Number of tasks declared for the project.
    pub tasks_required: u32,
    /// Number of tasks marked completed.
    pub tasks_completed: u32,
    /// `true` once every required step has been completed.
    pub completed: bool,
    /// Ordered list of steps for this project.
    pub steps: Vec<PipelineStep>,
}

/// Pipeline template read from YAML. Identical structure to ProjectYaml but
/// with a placeholder project-id; used as the source for new project creation.
pub type PipelineTemplate = ProjectYaml;

const ACTIVE_DIR: &str = "rex/active";
const INACTIVE_DIR: &str = "rex/inactive";
const PROJECT_FILE: &str = "project.yaml";

/// Parse the pipeline template YAML.
///
/// # Errors
/// [`RexError::YamlParse`] if the YAML is malformed.
pub fn parse_pipeline(yaml: &str) -> Result<PipelineTemplate, RexError> {
    serde_yml::from_str(yaml).map_err(|source| RexError::YamlParse {
        path: PathBuf::from("rex/pipeline.yaml"),
        source,
    })
}

/// Write a customized `project.yaml` to `rex/active/project.yaml` in `cwd`.
///
/// # Errors
/// - [`RexError::Io`] for filesystem failures
/// - [`RexError::YamlParse`] if the project cannot be serialized to YAML
pub fn write_active_project(cwd: &Path, project: &ProjectYaml) -> Result<(), RexError> {
    let active_dir = cwd.join(ACTIVE_DIR);
    fs::create_dir_all(&active_dir).map_err(|source| RexError::Io {
        path: active_dir.clone(),
        source,
    })?;

    let yaml = serde_yml::to_string(project).map_err(|source| RexError::YamlParse {
        path: active_dir.join(PROJECT_FILE),
        source,
    })?;

    let project_path = active_dir.join(PROJECT_FILE);
    fs::write(&project_path, yaml.as_bytes()).map_err(|source| RexError::Io {
        path: project_path,
        source,
    })
}

/// Read the active project from `rex/active/project.yaml`.
///
/// # Errors
/// - [`RexError::NoActiveProject`] when the file is absent
/// - [`RexError::Io`] reading the file
/// - [`RexError::YamlParse`] if the YAML is malformed
pub fn read_active_project(cwd: &Path) -> Result<ProjectYaml, RexError> {
    let project_path = cwd.join(ACTIVE_DIR).join(PROJECT_FILE);
    if !project_path.exists() {
        return Err(RexError::NoActiveProject { path: project_path });
    }
    let raw = fs::read_to_string(&project_path).map_err(|source| RexError::Io {
        path: project_path.clone(),
        source,
    })?;
    serde_yml::from_str(&raw).map_err(|source| RexError::YamlParse {
        path: project_path,
        source,
    })
}

/// Check whether an active project exists (non-empty `rex/active/`).
pub fn has_active_project(cwd: &Path) -> bool {
    cwd.join(ACTIVE_DIR).join(PROJECT_FILE).exists()
}

/// Archive current active to `rex/inactive/<project_id>/`.
///
/// Returns the project-id that was archived.
///
/// # Errors
/// - [`RexError::NoActiveProject`] / [`RexError::MissingProjectId`] when no project to archive
/// - [`RexError::SlugCollision`] when `rex/inactive/<id>/` already exists
/// - [`RexError::Io`] for filesystem failures
pub fn archive_active(cwd: &Path) -> Result<ProjectId, RexError> {
    let active = read_active_project(cwd)?;
    let project_id = active.project_id.clone();
    if project_id.is_empty() {
        return Err(RexError::MissingProjectId);
    }

    let inactive_target = cwd.join(INACTIVE_DIR).join(project_id.as_str());
    if inactive_target.exists() {
        return Err(RexError::SlugCollision {
            path: inactive_target,
        });
    }

    let inactive_parent = cwd.join(INACTIVE_DIR);
    fs::create_dir_all(&inactive_parent).map_err(|source| RexError::Io {
        path: inactive_parent,
        source,
    })?;

    let active_dir = cwd.join(ACTIVE_DIR);
    fs::rename(&active_dir, &inactive_target).map_err(|source| RexError::Io {
        path: active_dir,
        source,
    })?;

    Ok(project_id)
}

/// Activate an inactive project by ID, archiving the current active first if any.
///
/// # Errors
/// - [`RexError::ProjectNotFound`] if `rex/inactive/<project_id>/` is absent
/// - [`RexError::SlugCollision`] when archiving the current active fails (id reuse)
/// - [`RexError::Io`] for filesystem failures
pub fn swap_active(cwd: &Path, project_id: &str) -> Result<(), RexError> {
    let source = cwd.join(INACTIVE_DIR).join(project_id);
    if !source.exists() {
        return Err(RexError::ProjectNotFound {
            id: project_id.to_owned(),
        });
    }

    let active_dir = cwd.join(ACTIVE_DIR);

    if has_active_project(cwd) {
        archive_active(cwd)?;
    } else if active_dir.exists() {
        fs::remove_dir_all(&active_dir).map_err(|source| RexError::Io {
            path: active_dir.clone(),
            source,
        })?;
    }

    fs::rename(&source, &active_dir).map_err(|io_err| RexError::Io {
        path: source.clone(),
        source: io_err,
    })
}

/// List project IDs available in `rex/inactive/`, sorted.
///
/// # Errors
/// [`RexError::Io`] reading the inactive directory.
pub fn list_inactive(cwd: &Path) -> Result<Vec<ProjectId>, RexError> {
    let inactive_dir = cwd.join(INACTIVE_DIR);
    if !inactive_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&inactive_dir).map_err(|source| RexError::Io {
        path: inactive_dir.clone(),
        source,
    })?;

    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| RexError::Io {
            path: inactive_dir.clone(),
            source,
        })?;
        if entry.path().is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            ids.push(ProjectId::new(name));
        }
    }
    ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    Ok(ids)
}

/// Retain only steps that are required, or whose `step` name is in `keep_optional`.
pub fn prune_steps(template: &PipelineTemplate, keep_optional: &[&str]) -> Vec<PipelineStep> {
    template
        .steps
        .iter()
        .filter(|s| s.required || keep_optional.contains(&s.step.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_template() -> PipelineTemplate {
        PipelineTemplate {
            project_id: ProjectId::new("template"),
            category: "feature".to_owned(),
            title: None,
            subtitle: None,
            description: None,
            complexity: "medium".to_owned(),
            chunks_required: 0,
            chunks_completed: 0,
            tasks_required: 0,
            tasks_completed: 0,
            completed: false,
            steps: vec![
                PipelineStep {
                    step: "discovery".to_owned(),
                    required: true,
                    skill: None,
                    agent: None,
                    instructions: None,
                    inputs: None,
                    outputs: None,
                    completed: false,
                },
                PipelineStep {
                    step: "resources".to_owned(),
                    required: false,
                    skill: None,
                    agent: None,
                    instructions: None,
                    inputs: None,
                    outputs: None,
                    completed: false,
                },
                PipelineStep {
                    step: "prd".to_owned(),
                    required: true,
                    skill: None,
                    agent: None,
                    instructions: None,
                    inputs: None,
                    outputs: None,
                    completed: false,
                },
            ],
        }
    }

    #[test]
    fn prune_keeps_required_only() {
        let tmpl = make_template();
        let pruned = prune_steps(&tmpl, &[]);
        assert_eq!(pruned.len(), 2);
        assert!(pruned.iter().all(|s| s.required));
    }

    #[test]
    fn prune_includes_selected_optional() {
        let tmpl = make_template();
        let pruned = prune_steps(&tmpl, &["resources"]);
        assert_eq!(pruned.len(), 3);
        let names: Vec<_> = pruned.iter().map(|s| s.step.as_str()).collect();
        assert!(names.contains(&"resources"));
    }

    #[test]
    fn prune_skips_unselected_optional() {
        let tmpl = make_template();
        let pruned = prune_steps(&tmpl, &[]);
        let names: Vec<_> = pruned.iter().map(|s| s.step.as_str()).collect();
        assert!(!names.contains(&"resources"));
    }
}
