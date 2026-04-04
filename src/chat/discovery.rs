//! Project discovery: find registered projects and their autorun status.

use std::path::Path;

use crate::autorun::state::read_state;
use crate::autorun::types::AutorunState;
use crate::errors::RexResult;
use crate::models::project::ProjectRegistry;

/// A discovered project with its autorun status.
pub struct DiscoveredProject {
    pub id: String,
    pub title: String,
    pub directory: String,
    pub running: bool,
    pub autorun_state: Option<AutorunState>,
    pub has_project_status: bool,
}

/// Scan the ProjectRegistry for all projects and check their autorun status.
///
/// `project_dir` is where `rex/projects.json` lives (CWD is set to this).
pub fn discover_projects(_project_dir: &Path) -> RexResult<Vec<DiscoveredProject>> {
    let registry = ProjectRegistry::load().unwrap_or_default();

    let all_projects = registry.active.iter().chain(registry.inactive.iter());

    let mut results = Vec::new();

    for proj in all_projects {
        let dir = Path::new(&proj.directory);
        let state_path = dir.join(".rex-autorun.json");
        let status_path = dir.join("rex").join("project-status.json");

        let autorun_state = read_state(&state_path);
        let running = autorun_state.is_some();
        let has_project_status = status_path.exists();

        results.push(DiscoveredProject {
            id: proj.id.clone(),
            title: proj.title.clone(),
            directory: proj.directory.clone(),
            running,
            autorun_state,
            has_project_status,
        });
    }

    Ok(results)
}
