use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Binary,
    Library,
    Refactor,
}

impl Category {
    pub const ALL: [&str; 3] = ["binary", "library", "refactor"];

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Binary,
            1 => Self::Library,
            2 => Self::Refactor,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binary => write!(f, "binary"),
            Self::Library => write!(f, "library"),
            Self::Refactor => write!(f, "refactor"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Complexity {
    Low,
    Medium,
    High,
}

impl Complexity {
    pub const ALL: [&str; 3] = ["low", "medium", "high"];

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Low,
            1 => Self::Medium,
            2 => Self::High,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for Complexity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub category: Category,
    pub complexity: Complexity,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectRegistry {
    pub active: Option<Project>,
    pub inactive: Vec<Project>,
}

impl ProjectRegistry {
    fn registry_path() -> PathBuf {
        PathBuf::from("rex/projects.json")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::registry_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read projects.json: {e}"))?;
        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse projects.json: {e}"))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::registry_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {e}"))?;
        fs::write(&path, format!("{json}\n"))
            .map_err(|e| format!("Failed to write projects.json: {e}"))?;
        Ok(())
    }

    pub fn set_active(&mut self, project: Project) {
        if let Some(current) = self.active.take() {
            self.inactive.push(current);
        }
        self.active = Some(project);
    }

    pub fn has_project(&self, id: &str) -> bool {
        self.active.as_ref().is_some_and(|a| a.id == id)
            || self.inactive.iter().any(|p| p.id == id)
    }

    pub fn remove_project(&mut self, id: &str) -> Option<Project> {
        if self.active.as_ref().is_some_and(|a| a.id == id) {
            return self.active.take();
        }
        let pos = self.inactive.iter().position(|p| p.id == id)?;
        Some(self.inactive.remove(pos))
    }

    pub fn activate_project(&mut self, id: &str) -> Result<(), String> {
        if self.active.as_ref().is_some_and(|a| a.id == id) {
            return Err(format!("Project \"{id}\" is already the active project."));
        }
        let pos = self
            .inactive
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| format!("Project \"{id}\" not found."))?;
        let project = self.inactive.remove(pos);
        self.set_active(project);
        Ok(())
    }
}
