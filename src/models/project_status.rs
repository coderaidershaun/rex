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
pub struct OnboardingStep {
    pub item: String,
    pub agent: Agent,
    pub inputs: Vec<String>,
    pub output: String,
    pub status: Status,
}

pub const ONBOARDING_ITEMS: &[&str] = &[
    "goal",
    "scope",
    "existing-code",
    "libraries-and-sdks",
    "research",
    "resources",
    "user-expertise",
    "uat",
    "known-risks",
    "success-measures",
    "environment-variables",
    "idea-generation",
    "team-building",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatus {
    pub user_support: Step,
    pub onboarding: Vec<OnboardingStep>,
}

impl ProjectStatus {
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

                OnboardingStep {
                    item: item.to_string(),
                    agent: Agent {
                        effort: "medium".into(),
                        model: "sonnet".into(),
                        skills: vec![],
                    },
                    inputs: vec![],
                    output: format!("onboarding/{item}.md"),
                    status,
                }
            })
            .collect();

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
    matches!(item, "goal" | "scope" | "uat")
}
