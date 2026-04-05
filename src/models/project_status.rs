use crate::errors::{RexError, RexResult};
use crate::models::project::Category;
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
    #[serde(default = "default_count")]
    pub count: u32,
    pub effort: String,
    pub model: String,
    pub skills: Vec<String>,
}

fn default_count() -> u32 {
    1
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
    #[serde(rename = "stop-on-finish", default)]
    pub stop_on_finish: bool,
    pub agent: Agent,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
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
    "integration-testing",
    "success-measures",
    "known-risks",
    "uat",
    "environment-variables",
    "idea-generation",
    "skill-building",
    "checklist",
];

pub const DESIGN_ITEMS: &[&str] = &[
    "existing-code-exploration",
    "library-review",
    "module-design",
    "architecture-design",
    "integration-testing",
    "foreign-critique",
    "error-handling",
    "architecture-proposal",
    "user-acceptance",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatus {
    pub user_support: Vec<TaskStep>,
    pub onboarding: Vec<TaskStep>,
    pub design: Vec<TaskStep>,
    pub planning: Vec<TaskStep>,
    pub execution: Vec<TaskStep>,
}

impl ProjectStatus {
    pub fn load(project_dir: &Path) -> RexResult<Self> {
        let path = project_dir.join("project-status.json");
        let contents = fs::read_to_string(&path)
            .map_err(|e| RexError::FileRead { path: path.display().to_string(), source: e })?;
        serde_json::from_str(&contents)
            .map_err(|e| RexError::JsonParse { context: "project-status.json".into(), source: e })
    }

    pub fn new(
        project_id: &str,
        selected_onboarding: &[String],
        selected_design: &[String],
        category: &Category,
    ) -> Self {
        let onboarding = ONBOARDING_ITEMS
            .iter()
            .enumerate()
            .map(|(i, &item)| {
                let is_selected = selected_onboarding.iter().any(|s| s == item);
                let status = if is_required_always(item) || is_selected {
                    Status::NotStarted
                } else {
                    Status::NotRequired
                };

                let inputs = ONBOARDING_ITEMS[..i]
                    .iter()
                    .map(|prev| format!("rex/{project_id}/onboarding/{prev}.md"))
                    .collect();

                let (effort, model) = agent_defaults(item);
                let ext = if item == "checklist" { "json" } else { "md" };

                TaskStep {
                    item: item.to_string(),
                    stop_on_finish: item == "checklist",
                    agent: Agent {
                        count: 1,
                        effort: effort.into(),
                        model: model.into(),
                        skills: vec![format!("rex-onboarding-{item}")],
                    },
                    inputs,
                    outputs: vec![format!("rex/{project_id}/onboarding/{item}.{ext}")],
                    status,
                }
            })
            .collect();

        let design = build_design_steps(project_id, selected_design, category);
        let planning = build_planning_steps(project_id);
        let execution = build_execution_steps();

        Self {
            user_support: vec![TaskStep {
                item: "user-input".into(),
                stop_on_finish: false,
                agent: Agent {
                    count: 1,
                    effort: "high".into(),
                    model: "sonnet".into(),
                    skills: vec!["rex-user-input".into()],
                },
                inputs: vec![format!("rex/{project_id}/user-support/requested.md")],
                outputs: vec![format!("rex/{project_id}/user-support/provided.md")],
                status: Status::Completed,
            }],
            onboarding,
            design,
            planning,
            execution,
        }
    }

    pub fn save(&self, project_dir: &Path) -> RexResult<()> {
        let path = project_dir.join("project-status.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| RexError::JsonSerialize { context: "project-status.json".into(), source: e })?;
        fs::write(&path, format!("{json}\n"))
            .map_err(|e| RexError::FileWrite { path: path.display().to_string(), source: e })?;
        Ok(())
    }
}

/// Items that are required regardless of category.
fn is_required_always(item: &str) -> bool {
    matches!(item, "goal" | "scope" | "uat" | "checklist")
}

/// Returns the default (effort, model) for each onboarding item.
fn agent_defaults(_item: &str) -> (&'static str, &'static str) {
    ("high", "sonnet")
}

/// Design items that are always required.
pub fn is_design_required(item: &str, category: &Category) -> bool {
    matches!(
        item,
        "module-design" | "architecture-design" | "error-handling" | "architecture-proposal"
    ) || (item == "existing-code-exploration" && matches!(category, Category::Refactor))
}

fn build_design_steps(
    project_id: &str,
    selected_items: &[String],
    category: &Category,
) -> Vec<TaskStep> {
    DESIGN_ITEMS
        .iter()
        .map(|&item| {
            let is_selected = selected_items.iter().any(|s| s == item);
            let status = if is_design_required(item, category) || is_selected {
                Status::NotStarted
            } else {
                Status::NotRequired
            };

            let (count, effort, model, skills) = design_agent_config(item);

            TaskStep {
                item: item.to_string(),
                stop_on_finish: true,
                agent: Agent {
                    count,
                    effort: effort.into(),
                    model: model.into(),
                    skills: skills.iter().map(|s| s.to_string()).collect(),
                },
                inputs: design_inputs(project_id, item),
                outputs: design_outputs(project_id, item),
                status,
            }
        })
        .collect()
}

fn design_agent_config(item: &str) -> (u32, &'static str, &'static str, Vec<&'static str>) {
    match item {
        "existing-code-exploration" => (
            3,
            "high",
            "sonnet",
            vec!["rex-design-rust-existing-code-exploration"],
        ),
        "library-review" => (1, "high", "sonnet", vec!["rex-design-rust-library-review"]),
        "module-design" => (1, "max", "opus", vec!["rex-design-rust-modules"]),
        "architecture-design" => (1, "max", "opus", vec!["rex-design-rust-architecture"]),
        "integration-testing" => (
            1,
            "high",
            "sonnet",
            vec!["rex-design-rust-integration-tests"],
        ),
        "foreign-critique" => (3, "max", "opus", vec!["rex-design-foreign-critique"]),
        "error-handling" => (1, "high", "sonnet", vec!["rex-design-rust-errors"]),
        "architecture-proposal" => (
            1,
            "max",
            "opus",
            vec!["rex-design-rust-architecture-proposal"],
        ),
        "user-acceptance" => (1, "high", "sonnet", vec!["rex-design-user-acceptance"]),
        _ => (1, "high", "sonnet", vec![]),
    }
}

fn design_inputs(id: &str, item: &str) -> Vec<String> {
    let o = |name: &str| format!("rex/{id}/onboarding/{name}");
    let d = |name: &str| format!("rex/{id}/design/{name}");

    match item {
        "existing-code-exploration" => vec![o("goal.md"), o("scope.md"), o("existing-code.md")],
        "library-review" => vec![
            o("goal.md"),
            o("scope.md"),
            o("user-expertise.md"),
            o("checklist.json"),
            o("libraries-and-sdks.md"),
        ],
        "module-design" => vec![
            o("research.md"),
            o("resources.md"),
            o("user-expertise.md"),
            o("success-measures.md"),
            o("known-risks.md"),
            o("uat.md"),
            o("environment-variables.md"),
            o("idea-generation.md"),
            o("goal.md"),
            o("user-expertise.md"),
            o("checklist.json"),
            o("libraries-and-sdks.md"),
            d("existing-code-exploration.md"),
            d("library-review.md"),
        ],
        "architecture-design" => vec![
            o("research.md"),
            o("resources.md"),
            o("user-expertise.md"),
            o("success-measures.md"),
            o("known-risks.md"),
            o("uat.md"),
            o("environment-variables.md"),
            o("idea-generation.md"),
            o("goal.md"),
            o("user-expertise.md"),
            o("checklist.json"),
            o("libraries-and-sdks.md"),
            d("existing-code-exploration.md"),
            d("library-review.md"),
            d("module-design.md"),
        ],
        "integration-testing" => vec![
            o("research.md"),
            o("resources.md"),
            o("user-expertise.md"),
            o("success-measures.md"),
            o("known-risks.md"),
            o("uat.md"),
            o("environment-variables.md"),
            o("idea-generation.md"),
            o("goal.md"),
            o("user-expertise.md"),
            o("checklist.json"),
            o("libraries-and-sdks.md"),
            d("existing-code-exploration.md"),
            d("library-review.md"),
            d("module-design.md"),
            d("architecture-design.md"),
        ],
        "foreign-critique" => vec![
            o("goal.md"),
            o("scope.md"),
            o("existing-code.md"),
            o("libraries-and-sdks.md"),
            o("research.md"),
            o("resources.md"),
            o("user-expertise.md"),
            o("success-measures.md"),
            o("known-risks.md"),
            o("uat.md"),
            o("environment-variables.md"),
            o("idea-generation.md"),
            o("skill-building.md"),
            d("existing-code-exploration.md"),
            d("library-review.md"),
            d("module-design.md"),
            d("architecture-design.md"),
        ],
        "error-handling" => vec![
            d("existing-code-exploration.md"),
            d("library-review.md"),
            d("module-design.md"),
            d("architecture-design.md"),
        ],
        "architecture-proposal" => vec![
            o("goal.md"),
            o("scope.md"),
            d("error-handling.md"),
            d("existing-code-exploration.md"),
            d("library-review.md"),
            d("module-design.md"),
            d("architecture-design.md"),
            d("foreign-critique.md"),
        ],
        "user-acceptance" => vec![d("architecture-proposal.html")],
        _ => vec![],
    }
}

fn design_outputs(id: &str, item: &str) -> Vec<String> {
    let d = |name: &str| format!("rex/{id}/design/{name}");

    match item {
        "architecture-proposal" => {
            vec![d("architecture-proposal.md"), d("architecture-proposal.html")]
        }
        "integration-testing" => vec![d("integration-tests.md")],
        _ => vec![d(&format!("{item}.md"))],
    }
}

fn build_planning_steps(id: &str) -> Vec<TaskStep> {
    let o = |name: &str| format!("rex/{id}/onboarding/{name}");
    let d = |name: &str| format!("rex/{id}/design/{name}");
    let p = |name: &str| format!("rex/{id}/planning/{name}");

    let full_inputs = vec![
        o("goal.md"),
        o("scope.md"),
        o("research.md"),
        o("resources.md"),
        o("user-expertise.md"),
        o("success-measures.md"),
        o("known-risks.md"),
        o("uat.md"),
        o("environment-variables.md"),
        o("idea-generation.md"),
        o("checklist.json"),
        d("error-handling.md"),
        d("existing-code-exploration.md"),
        d("library-review.md"),
        d("module-design.md"),
        d("architecture-design.md"),
        d("foreign-critique.md"),
        d("architecture-proposal.md"),
        p("planning.json"),
    ];

    let tasks_inputs = vec![
        d("error-handling.md"),
        d("existing-code-exploration.md"),
        d("library-review.md"),
        d("module-design.md"),
        d("architecture-design.md"),
        d("foreign-critique.md"),
        d("architecture-proposal.md"),
        p("planning.json"),
    ];

    vec![
        TaskStep {
            item: "milestones".into(),
            stop_on_finish: true,
            agent: Agent {
                count: 1,
                effort: "max".into(),
                model: "opus".into(),
                skills: vec!["rex-planning-milestones".into()],
            },
            inputs: full_inputs.clone(),
            outputs: vec![],
            status: Status::NotStarted,
        },
        TaskStep {
            item: "objectives".into(),
            stop_on_finish: true,
            agent: Agent {
                count: 1,
                effort: "max".into(),
                model: "opus".into(),
                skills: vec!["rex-planning-objectives".into()],
            },
            inputs: full_inputs,
            outputs: vec![],
            status: Status::NotStarted,
        },
        TaskStep {
            item: "tasks".into(),
            stop_on_finish: true,
            agent: Agent {
                count: 1,
                effort: "max".into(),
                model: "opus".into(),
                skills: vec!["rex-planning-tasks".into()],
            },
            inputs: tasks_inputs,
            outputs: vec![],
            status: Status::NotStarted,
        },
        TaskStep {
            item: "review".into(),
            stop_on_finish: true,
            agent: Agent {
                count: 1,
                effort: "max".into(),
                model: "opus".into(),
                skills: vec!["rex-planning-review".into()],
            },
            inputs: vec![
                d("error-handling.md"),
                d("existing-code-exploration.md"),
                d("library-review.md"),
                d("module-design.md"),
                d("architecture-design.md"),
                d("existing-code-exploration.md"),
                d("library-review.md"),
                d("module-design.md"),
                d("architecture-design.md"),
                d("architecture-proposal.md"),
                p("milestones.json"),
                p("objectives.json"),
                p("tasks.json"),
                p("planning.json"),
            ],
            outputs: vec![],
            status: Status::NotStarted,
        },
    ]
}

fn build_execution_steps() -> Vec<TaskStep> {
    vec![TaskStep {
        item: "run".into(),
        stop_on_finish: false,
        agent: Agent {
            count: 1,
            effort: "high".into(),
            model: "sonnet".into(),
            skills: vec![
                "STEP 1: run 'rex task next'".into(),
                "STEP 2: Assign agents with model, effort level and skills as defined in the task".into(),
            ],
        },
        inputs: vec![],
        outputs: vec![],
        status: Status::NotStarted,
    }]
}
