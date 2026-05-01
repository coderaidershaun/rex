use std::{fs, path::PathBuf};

use crate::{
    error::RexError,
    schedule::{Schedule, counters_for},
};

use super::{ProjectId, ProjectYaml};

const ACTIVE_DIR: &str = "rex/active";
const INACTIVE_DIR: &str = "rex/inactive";
const PROJECT_FILE: &str = "project.yaml";
const SCHEDULE_FILE: &str = "schedule.json";

/// On-disk lifecycle manager for the active and inactive project directories.
///
/// All methods operate relative to the `root` path supplied at construction.
#[derive(Debug, Clone)]
pub struct ProjectStore {
    root: PathBuf,
}

impl ProjectStore {
    /// Create a store rooted at `root` (typically the current working directory).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Write a project to `rex/active/project.yaml`.
    ///
    /// # Errors
    /// - [`RexError::Io`] for filesystem failures
    /// - [`RexError::Yaml`] if the project cannot be serialized to YAML
    pub fn write_active(&self, project: &ProjectYaml) -> Result<(), RexError> {
        let active_dir = self.root.join(ACTIVE_DIR);
        fs::create_dir_all(&active_dir).map_err(|source| RexError::Io {
            path: active_dir.clone(),
            source,
        })?;

        let project_path = active_dir.join(PROJECT_FILE);
        let yaml = serde_yml::to_string(project).map_err(|source| RexError::Yaml {
            path: project_path.clone(),
            source,
        })?;

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
    /// - [`RexError::Yaml`] if the YAML is malformed
    pub fn read_active(&self) -> Result<ProjectYaml, RexError> {
        let project_path = self.root.join(ACTIVE_DIR).join(PROJECT_FILE);
        if !project_path.exists() {
            return Err(RexError::NoActiveProject { path: project_path });
        }
        let raw = fs::read_to_string(&project_path).map_err(|source| RexError::Io {
            path: project_path.clone(),
            source,
        })?;
        serde_yml::from_str(&raw).map_err(|source| RexError::Yaml {
            path: project_path,
            source,
        })
    }

    /// Returns `true` when `rex/active/project.yaml` exists.
    pub fn has_active(&self) -> bool {
        self.root.join(ACTIVE_DIR).join(PROJECT_FILE).exists()
    }

    /// Archive the current active project to `rex/inactive/<project_id>/`.
    ///
    /// Returns the project-id that was archived.
    ///
    /// # Errors
    /// - [`RexError::NoActiveProject`] when no project is active
    /// - [`RexError::SlugCollision`] when `rex/inactive/<id>/` already exists
    /// - [`RexError::Io`] for filesystem failures
    pub fn archive_active(&self) -> Result<ProjectId, RexError> {
        let active = self.read_active()?;
        let project_id = active.project_id.clone();

        let inactive_target = self.root.join(INACTIVE_DIR).join(project_id.as_str());
        if inactive_target.exists() {
            return Err(RexError::SlugCollision {
                path: inactive_target,
            });
        }

        let inactive_parent = self.root.join(INACTIVE_DIR);
        fs::create_dir_all(&inactive_parent).map_err(|source| RexError::Io {
            path: inactive_parent,
            source,
        })?;

        let active_dir = self.root.join(ACTIVE_DIR);
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
    pub fn swap_active(&self, project_id: &str) -> Result<(), RexError> {
        let source = self.root.join(INACTIVE_DIR).join(project_id);
        if !source.exists() {
            return Err(RexError::ProjectNotFound {
                id: project_id.to_owned(),
            });
        }

        let active_dir = self.root.join(ACTIVE_DIR);

        if self.has_active() {
            self.archive_active()?;
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

    /// Absolute path to the active project's `schedule.json`.
    ///
    /// The active project lives flat under `rex/active/`; the slug only enters
    /// paths at archive boundaries.
    pub fn schedule_path(&self) -> PathBuf {
        self.root.join(ACTIVE_DIR).join(SCHEDULE_FILE)
    }

    /// Read and deserialize the active project's `schedule.json`.
    ///
    /// # Errors
    /// - [`RexError::ScheduleNotFound`] when the file is absent
    /// - [`RexError::Io`] for other filesystem failures
    /// - [`RexError::JsonParse`] if the JSON is malformed
    pub fn read_schedule(&self) -> Result<Schedule, RexError> {
        let path = self.schedule_path();
        if !path.exists() {
            return Err(RexError::ScheduleNotFound { path });
        }
        let raw = fs::read_to_string(&path).map_err(|source| RexError::Io {
            path: path.clone(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|source| RexError::JsonParse { path, source })
    }

    /// Serialize and write the active project's `schedule.json`.
    ///
    /// Creates the `rex/active/` directory if it does not exist.
    ///
    /// # Errors
    /// - [`RexError::Io`] for filesystem failures
    /// - [`RexError::JsonSerialize`] if serialization fails
    pub fn write_schedule(&self, schedule: &Schedule) -> Result<(), RexError> {
        let path = self.schedule_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| RexError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let json = serde_json::to_string_pretty(schedule)?;
        fs::write(&path, json.as_bytes()).map_err(|source| RexError::Io { path, source })
    }

    /// Write `schedule.json` and update the four counter fields in `project.yaml`
    /// in one call.
    ///
    /// This is the single seam that keeps `project.yaml` counters in sync with
    /// `schedule.json`. Every CRUD mutation on phases/chunks/tasks routes through
    /// here; open-coded counter arithmetic belongs nowhere else.
    ///
    /// # Errors
    /// - [`RexError::NoActiveProject`] when `project.yaml` is absent.
    /// - [`RexError::Io`] / [`RexError::Yaml`] / [`RexError::JsonSerialize`] for I/O failures.
    pub fn write_schedule_with_counters(&self, schedule: &Schedule) -> Result<(), RexError> {
        let counters = counters_for(schedule);
        let mut project = self.read_active()?;
        project.chunks_required = counters.chunks_required;
        project.tasks_required = counters.tasks_required;
        project.chunks_completed = counters.chunks_completed;
        project.tasks_completed = counters.tasks_completed;
        self.write_schedule(schedule)?;
        self.write_active(&project)
    }

    /// List project IDs available in `rex/inactive/`, sorted.
    ///
    /// # Errors
    /// [`RexError::Io`] reading the inactive directory.
    pub fn list_inactive(&self) -> Result<Vec<ProjectId>, RexError> {
        let inactive_dir = self.root.join(INACTIVE_DIR);
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
                && let Ok(id) = ProjectId::parse(name)
            {
                ids.push(id);
            }
        }
        ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(ids)
    }
}
