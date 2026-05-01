pub mod store;

use std::{fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RexError;

pub use store::ProjectStore;

/// Slug identifier for a project, e.g. `my-cool-feature`.
///
/// Wrapping the raw slug as a newtype prevents it from being mixed with
/// other strings (titles, categories, file paths) at API boundaries.
/// Construction always goes through [`ProjectId::parse`] or `TryFrom`, so
/// the invariant (non-empty, non-whitespace-only) is always upheld.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct ProjectId(String);

impl ProjectId {
    /// Parse and validate a project-id slug.
    ///
    /// # Errors
    /// [`RexError::InvalidProjectId`] when `slug` is empty or whitespace-only.
    pub fn parse(slug: impl AsRef<str>) -> Result<Self, RexError> {
        let s = slug.as_ref();
        if s.trim().is_empty() {
            return Err(RexError::InvalidProjectId {
                reason: "must not be empty or whitespace-only".to_owned(),
            });
        }
        Ok(Self(s.to_owned()))
    }

    /// The slug as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ProjectId {
    type Error = RexError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for ProjectId {
    type Error = RexError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl From<ProjectId> for String {
    fn from(id: ProjectId) -> String {
        id.0
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

/// Project envelope consumed by rex pipeline agents.
///
/// Fields mirror [`ProjectYaml`] minus `steps`. Build via [`From<&ProjectYaml>`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectMeta {
    pub project_id: ProjectId,
    pub category: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub complexity: String,
    pub chunks_required: u32,
    pub chunks_completed: u32,
    pub tasks_required: u32,
    pub tasks_completed: u32,
    pub completed: bool,
}

impl From<&ProjectYaml> for ProjectMeta {
    fn from(p: &ProjectYaml) -> Self {
        Self {
            project_id: p.project_id.clone(),
            category: p.category.clone(),
            title: p.title.clone(),
            subtitle: p.subtitle.clone(),
            description: p.description.clone(),
            complexity: p.complexity.clone(),
            chunks_required: p.chunks_required,
            chunks_completed: p.chunks_completed,
            tasks_required: p.tasks_required,
            tasks_completed: p.tasks_completed,
            completed: p.completed,
        }
    }
}

/// Return the first step in `active` where `completed` is `false`, or `None`
/// when every step is complete (or the steps list is empty).
pub fn current_incomplete_step(active: &ProjectYaml) -> Option<&PipelineStep> {
    active.steps.iter().find(|s| !s.completed)
}

/// Parse the pipeline template YAML.
///
/// # Errors
/// [`RexError::Yaml`] if the YAML is malformed.
pub fn parse_pipeline(yaml: &str) -> Result<PipelineTemplate, RexError> {
    serde_yml::from_str(yaml).map_err(|source| RexError::Yaml {
        path: PathBuf::from("rex/pipeline.yaml"),
        source,
    })
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
            project_id: ProjectId::parse("template").unwrap(),
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

    #[test]
    fn project_id_parse_accepts_valid_slug() {
        let id = ProjectId::parse("my-project").unwrap();
        assert_eq!(id.as_str(), "my-project");
    }

    #[test]
    fn project_id_parse_rejects_empty() {
        let err = ProjectId::parse("").unwrap_err();
        assert!(
            matches!(err, crate::error::RexError::InvalidProjectId { .. }),
            "expected InvalidProjectId, got: {err}"
        );
    }

    #[test]
    fn project_id_parse_rejects_whitespace_only() {
        let err = ProjectId::parse("   ").unwrap_err();
        assert!(
            matches!(err, crate::error::RexError::InvalidProjectId { .. }),
            "expected InvalidProjectId, got: {err}"
        );
    }
}
