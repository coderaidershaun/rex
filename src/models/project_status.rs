use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    NotStarted,
    InProgress,
    NotRequired,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub effort: String,
    pub model: String,
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub agent: Agent,
    pub inputs: Vec<String>,
    pub output: String,
    pub status: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatus {
    pub user_support: Step,
    pub intake_goal: Step,
    pub intake_scope: Step,
    pub intake_existing_code_refs: Step,
    pub intake_user_knowledge: Step,
}

impl ProjectStatus {
    pub fn new(include_existing_code_refs: bool, include_user_knowledge: bool) -> Self {
        Self {
            user_support: Step {
                agent: Agent {
                    effort: "high".into(),
                    model: "opus".into(),
                    skills: vec!["rex-user-input".into()],
                },
                inputs: vec!["user-support/requested.md".into()],
                output: "user-support/provided.md".into(),
                status: Status::Completed,
            },
            intake_goal: Step {
                agent: Agent {
                    effort: "medium".into(),
                    model: "opus".into(),
                    skills: vec!["rex-intake-goal".into()],
                },
                inputs: vec![],
                output: "intake/goal.md".into(),
                status: Status::NotStarted,
            },
            intake_scope: Step {
                agent: Agent {
                    effort: "medium".into(),
                    model: "opus".into(),
                    skills: vec!["rex-intake-scope".into()],
                },
                inputs: vec![],
                output: "intake/scope.md".into(),
                status: Status::NotStarted,
            },
            intake_existing_code_refs: Step {
                agent: Agent {
                    effort: "medium".into(),
                    model: "sonnet".into(),
                    skills: vec!["rex-intake-existing-code-refs".into()],
                },
                inputs: vec![],
                output: "intake/existing-code-refs.md".into(),
                status: if include_existing_code_refs {
                    Status::NotStarted
                } else {
                    Status::NotRequired
                },
            },
            intake_user_knowledge: Step {
                agent: Agent {
                    effort: "medium".into(),
                    model: "sonnet".into(),
                    skills: vec!["rex-intake-user-knowledge".into()],
                },
                inputs: vec![],
                output: "intake/user-knowledge.md".into(),
                status: if include_user_knowledge {
                    Status::NotStarted
                } else {
                    Status::NotRequired
                },
            },
        }
    }

    pub fn save(&self, project_dir: &Path) -> Result<(), String> {
        let path = project_dir.join("project-status.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize project status: {e}"))?;
        fs::write(&path, format!("{json}\n"))
            .map_err(|e| format!("Failed to write project-status.json: {e}"))?;
        Ok(())
    }
}
