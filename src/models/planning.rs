use crate::models::project_status::Agent;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanningStatus {
    NotStarted,
    InProgress,
    Completed,
    Blocked,
}

impl fmt::Display for PlanningStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => f.write_str("not-started"),
            Self::InProgress => f.write_str("in-progress"),
            Self::Completed => f.write_str("completed"),
            Self::Blocked => f.write_str("blocked"),
        }
    }
}

// ---------------------------------------------------------------------------
// Checklist item (definition-of-done entry within an entity)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningChecklistItem {
    pub id: String,
    pub item: String,
    #[serde(default)]
    pub done: bool,
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default = "default_not_started")]
    pub status: PlanningStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checklist: Vec<PlanningChecklistItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upstream: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub downstream: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub id: String,
    pub milestone_id: String,
    pub title: String,
    pub description: String,
    #[serde(default = "default_not_started")]
    pub status: PlanningStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checklist: Vec<PlanningChecklistItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upstream: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub downstream: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub objective_id: String,
    pub title: String,
    pub description: String,
    #[serde(default = "default_not_started")]
    pub status: PlanningStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<Agent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checklist: Vec<PlanningChecklistItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upstream: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub downstream: Vec<String>,
}

fn default_not_started() -> PlanningStatus {
    PlanningStatus::NotStarted
}

// ---------------------------------------------------------------------------
// Unified store – single planning.json per project
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanningStore {
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    #[serde(default)]
    pub objectives: Vec<Objective>,
    #[serde(default)]
    pub tasks: Vec<Task>,
}

impl PlanningStore {
    pub fn load(project_dir: &Path) -> Result<Self, String> {
        let path = project_dir.join("planning/planning.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read planning.json: {e}"))?;
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse planning.json: {e}"))
    }

    pub fn save(&self, project_dir: &Path) -> Result<(), String> {
        let dir = project_dir.join("planning");
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create planning dir: {e}"))?;
        let path = dir.join("planning.json");
        let json =
            serde_json::to_string_pretty(self).map_err(|e| format!("Failed to serialize: {e}"))?;
        fs::write(&path, format!("{json}\n"))
            .map_err(|e| format!("Failed to write planning.json: {e}"))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ListMods – bag of add/remove operations for all list fields
// ---------------------------------------------------------------------------

pub struct ListMods {
    pub add_references: Vec<String>,
    pub remove_references: Vec<String>,
    pub add_outputs: Vec<String>,
    pub remove_outputs: Vec<String>,
    pub add_upstream: Vec<String>,
    pub remove_upstream: Vec<String>,
    pub add_downstream: Vec<String>,
    pub remove_downstream: Vec<String>,
    pub add_checklist: Vec<String>,
    pub remove_checklist: Vec<String>,
    pub check: Vec<String>,
    pub uncheck: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Append items from `add` (deduplicated) and remove items in `remove`.
pub fn apply_list_mods(list: &mut Vec<String>, add: &[String], remove: &[String]) {
    for item in remove {
        list.retain(|x| x != item);
    }
    for item in add {
        if !list.contains(item) {
            list.push(item.clone());
        }
    }
}

/// Apply add / remove / check / uncheck operations to a checklist.
///
/// `add` entries use the format `"id:text"`.
pub fn apply_checklist_mods(
    checklist: &mut Vec<PlanningChecklistItem>,
    add: &[String],
    remove: &[String],
    check: &[String],
    uncheck: &[String],
) -> Result<(), String> {
    for id in remove {
        checklist.retain(|item| item.id != *id);
    }
    for entry in add {
        let (id, text) = entry
            .split_once(':')
            .ok_or_else(|| format!("Invalid checklist format: \"{entry}\". Expected \"id:text\"."))?;
        if checklist.iter().any(|i| i.id == id) {
            return Err(format!("Checklist item \"{id}\" already exists."));
        }
        checklist.push(PlanningChecklistItem {
            id: id.to_string(),
            item: text.to_string(),
            done: false,
        });
    }
    for id in check {
        let item = checklist
            .iter_mut()
            .find(|i| i.id == *id)
            .ok_or_else(|| format!("Checklist item \"{id}\" not found."))?;
        item.done = true;
    }
    for id in uncheck {
        let item = checklist
            .iter_mut()
            .find(|i| i.id == *id)
            .ok_or_else(|| format!("Checklist item \"{id}\" not found."))?;
        item.done = false;
    }
    Ok(())
}

/// Apply all list modifications to the standard entity fields.
pub fn apply_all_list_mods(
    references: &mut Vec<String>,
    outputs: &mut Vec<String>,
    upstream: &mut Vec<String>,
    downstream: &mut Vec<String>,
    checklist: &mut Vec<PlanningChecklistItem>,
    mods: &ListMods,
) -> Result<(), String> {
    apply_list_mods(references, &mods.add_references, &mods.remove_references);
    apply_list_mods(outputs, &mods.add_outputs, &mods.remove_outputs);
    apply_list_mods(upstream, &mods.add_upstream, &mods.remove_upstream);
    apply_list_mods(downstream, &mods.add_downstream, &mods.remove_downstream);
    apply_checklist_mods(
        checklist,
        &mods.add_checklist,
        &mods.remove_checklist,
        &mods.check,
        &mods.uncheck,
    )
}
