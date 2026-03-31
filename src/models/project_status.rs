use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize)]
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
pub struct TaskStep {
    pub item: String,
    pub agent: Agent,
    pub inputs: Vec<String>,
    pub output: String,
    pub status: Status,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => f.write_str("not-started"),
            Self::InProgress => f.write_str("in-progress"),
            Self::NotRequired => f.write_str("not-required"),
            Self::Completed => f.write_str("completed"),
        }
    }
}

pub const ONBOARDING_ITEMS: &[&str] = &[
    "goal",
    "scope",
    "existing-code",
    "libraries-and-sdks",
    "research",
    "resources",
    "user-expertise",
    "success-measures",
    "known-risks",
    "uat",
    "environment-variables",
    "idea-generation",
    "skill-building",
    "checklist",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatus {
    pub user_support: Vec<TaskStep>,
    pub onboarding: Vec<TaskStep>,
}

impl ProjectStatus {
    pub fn load(project_dir: &Path) -> Result<Self, String> {
        let path = project_dir.join("project-status.json");
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read project-status.json: {e}"))?;
        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse project-status.json: {e}"))
    }

    pub fn new(selected_items: &[String]) -> Self {
        let onboarding = ONBOARDING_ITEMS
            .iter()
            .map(|&item| {
                let is_selected = selected_items.iter().any(|s| s == item);
                let status = if is_required_always(item) || is_selected {
                    Status::NotStarted
                } else {
                    Status::NotRequired
                };

                TaskStep {
                    item: item.to_string(),
                    agent: Agent {
                        effort: "medium".into(),
                        model: "sonnet".into(),
                        skills: vec![format!("rex-onboarding-{item}")],
                    },
                    inputs: vec![],
                    output: format!("onboarding/{item}.md"),
                    status,
                }
            })
            .collect();

        Self {
            user_support: vec![TaskStep {
                item: "user-input".into(),
                agent: Agent {
                    effort: "high".into(),
                    model: "opus".into(),
                    skills: vec!["rex-user-input".into()],
                },
                inputs: vec!["user-support/requested.md".into()],
                output: "user-support/provided.md".into(),
                status: Status::Completed,
            }],
            onboarding,
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

/// Items that are required regardless of category.
fn is_required_always(item: &str) -> bool {
    matches!(item, "goal" | "scope" | "uat" | "checklist")
}
