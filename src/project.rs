use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::RexError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct PipelineStep {
    pub step: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectYaml {
    pub project_id: String,
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
    pub steps: Vec<PipelineStep>,
}

/// Pipeline template read from YAML. Identical structure to ProjectYaml but
/// with a placeholder project-id; used as the source for new project creation.
pub type PipelineTemplate = ProjectYaml;

const ACTIVE_DIR: &str = "rex/active";
const INACTIVE_DIR: &str = "rex/inactive";
const PROJECT_FILE: &str = "project.yaml";

/// Read the pipeline template YAML.
pub fn parse_pipeline(yaml: &str) -> Result<PipelineTemplate, RexError> {
    serde_yml::from_str(yaml).map_err(|source| RexError::YamlParse {
        path: PathBuf::from("rex/pipeline.yaml"),
        source,
    })
}

/// Write a customized project.yaml to `rex/active/project.yaml` in `cwd`.
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
pub fn archive_active(cwd: &Path) -> Result<String, RexError> {
    let active = read_active_project(cwd)?;
    let project_id = active.project_id.clone();
    if project_id.is_empty() {
        return Err(RexError::MissingProjectId);
    }

    let inactive_target = cwd.join(INACTIVE_DIR).join(&project_id);
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
        // Active dir exists but no project.yaml — clean it up so rename succeeds.
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

/// List project IDs available in `rex/inactive/`.
pub fn list_inactive(cwd: &Path) -> Result<Vec<String>, RexError> {
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
            ids.push(name.to_owned());
        }
    }
    ids.sort();
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
            project_id: "template".to_owned(),
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
