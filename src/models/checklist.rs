use crate::errors::{RexError, RexResult};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Design,
    Planning,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design => f.write_str("design"),
            Self::Planning => f.write_str("planning"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ChecklistCategory {
    DesignMustHaves,
    ArchitectureConstraints,
    PlanningMilestones,
    Objectives,
    TasksToPlanFor,
    ResearchAndPrototyping,
    RiskMitigations,
    OutOfScope,
}

impl ChecklistCategory {
    pub const ALL: &'static [Self] = &[
        Self::DesignMustHaves,
        Self::ArchitectureConstraints,
        Self::PlanningMilestones,
        Self::Objectives,
        Self::TasksToPlanFor,
        Self::ResearchAndPrototyping,
        Self::RiskMitigations,
        Self::OutOfScope,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::DesignMustHaves => "Design Must-Haves",
            Self::ArchitectureConstraints => "Architecture Constraints",
            Self::PlanningMilestones => "Planning Milestones",
            Self::Objectives => "Objectives",
            Self::TasksToPlanFor => "Tasks to Plan For",
            Self::ResearchAndPrototyping => "Research & Prototyping",
            Self::RiskMitigations => "Risk Mitigations",
            Self::OutOfScope => "Out of Scope",
        }
    }
}

impl fmt::Display for ChecklistCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DesignMustHaves => f.write_str("design-must-haves"),
            Self::ArchitectureConstraints => f.write_str("architecture-constraints"),
            Self::PlanningMilestones => f.write_str("planning-milestones"),
            Self::Objectives => f.write_str("objectives"),
            Self::TasksToPlanFor => f.write_str("tasks-to-plan-for"),
            Self::ResearchAndPrototyping => f.write_str("research-and-prototyping"),
            Self::RiskMitigations => f.write_str("risk-mitigations"),
            Self::OutOfScope => f.write_str("out-of-scope"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<Phase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectChecklist {
    pub date: String,
    #[serde(default)]
    pub design_must_haves: Vec<ChecklistItem>,
    #[serde(default)]
    pub architecture_constraints: Vec<ChecklistItem>,
    #[serde(default)]
    pub planning_milestones: Vec<ChecklistItem>,
    #[serde(default)]
    pub objectives: Vec<ChecklistItem>,
    #[serde(default)]
    pub tasks_to_plan_for: Vec<ChecklistItem>,
    #[serde(default)]
    pub research_and_prototyping: Vec<ChecklistItem>,
    #[serde(default)]
    pub risk_mitigations: Vec<ChecklistItem>,
    #[serde(default)]
    pub out_of_scope: Vec<ChecklistItem>,
    #[serde(default)]
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checklist {
    pub project_checklist: ProjectChecklist,
}

impl Checklist {
    pub fn new(date: &str) -> Self {
        Self {
            project_checklist: ProjectChecklist {
                date: date.to_string(),
                design_must_haves: Vec::new(),
                architecture_constraints: Vec::new(),
                planning_milestones: Vec::new(),
                objectives: Vec::new(),
                tasks_to_plan_for: Vec::new(),
                research_and_prototyping: Vec::new(),
                risk_mitigations: Vec::new(),
                out_of_scope: Vec::new(),
                context: String::new(),
            },
        }
    }

    pub fn load(path: &Path) -> RexResult<Self> {
        let contents = fs::read_to_string(path)
            .map_err(|e| RexError::FileRead { path: path.display().to_string(), source: e })?;
        serde_json::from_str(&contents)
            .map_err(|e| RexError::JsonParse { context: "checklist.json".into(), source: e })
    }

    pub fn save(&self, path: &Path) -> RexResult<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| RexError::JsonSerialize { context: "checklist.json".into(), source: e })?;
        fs::write(path, format!("{json}\n"))
            .map_err(|e| RexError::FileWrite { path: path.display().to_string(), source: e })?;
        Ok(())
    }

    pub fn items(&self, category: ChecklistCategory) -> &[ChecklistItem] {
        let c = &self.project_checklist;
        match category {
            ChecklistCategory::DesignMustHaves => &c.design_must_haves,
            ChecklistCategory::ArchitectureConstraints => &c.architecture_constraints,
            ChecklistCategory::PlanningMilestones => &c.planning_milestones,
            ChecklistCategory::Objectives => &c.objectives,
            ChecklistCategory::TasksToPlanFor => &c.tasks_to_plan_for,
            ChecklistCategory::ResearchAndPrototyping => &c.research_and_prototyping,
            ChecklistCategory::RiskMitigations => &c.risk_mitigations,
            ChecklistCategory::OutOfScope => &c.out_of_scope,
        }
    }

    pub fn items_mut(&mut self, category: ChecklistCategory) -> &mut Vec<ChecklistItem> {
        let c = &mut self.project_checklist;
        match category {
            ChecklistCategory::DesignMustHaves => &mut c.design_must_haves,
            ChecklistCategory::ArchitectureConstraints => &mut c.architecture_constraints,
            ChecklistCategory::PlanningMilestones => &mut c.planning_milestones,
            ChecklistCategory::Objectives => &mut c.objectives,
            ChecklistCategory::TasksToPlanFor => &mut c.tasks_to_plan_for,
            ChecklistCategory::ResearchAndPrototyping => &mut c.research_and_prototyping,
            ChecklistCategory::RiskMitigations => &mut c.risk_mitigations,
            ChecklistCategory::OutOfScope => &mut c.out_of_scope,
        }
    }

    pub fn find_category(&self, id: &str) -> Option<ChecklistCategory> {
        for &cat in ChecklistCategory::ALL {
            if self.items(cat).iter().any(|item| item.id == id) {
                return Some(cat);
            }
        }
        None
    }

    pub fn has_id(&self, id: &str) -> bool {
        self.find_category(id).is_some()
    }
}
