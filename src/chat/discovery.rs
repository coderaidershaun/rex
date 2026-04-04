//! Project discovery: scan the filesystem for rex projects and their autorun status.

use std::collections::HashSet;
use std::path::Path;
use std::{fs, io};

use tracing::debug;

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

/// Directories to skip during filesystem scanning.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".cargo",
    ".rustup",
    ".cache",
    ".local",
    ".npm",
    ".nvm",
    ".pyenv",
    ".venv",
    "venv",
    "vendor",
    "__pycache__",
    "Library",
    "Applications",
    ".Trash",
];

const MAX_DEPTH: usize = 8;

/// Scan the filesystem from `scan_dir` for all rex projects.
///
/// Walks directories looking for `rex/projects.json` files, loads each registry
/// found, and returns all projects with their autorun status. Deduplicates by
/// project ID (first-found wins).
pub fn discover_projects(scan_dir: &Path) -> RexResult<Vec<DiscoveredProject>> {
    let mut results = Vec::new();
    let mut seen_ids = HashSet::new();

    scan_recursive(scan_dir, 0, &mut results, &mut seen_ids);

    debug!(
        scan_dir = %scan_dir.display(),
        projects = results.len(),
        "filesystem scan complete"
    );

    Ok(results)
}

fn scan_recursive(
    dir: &Path,
    depth: usize,
    results: &mut Vec<DiscoveredProject>,
    seen_ids: &mut HashSet<String>,
) {
    if depth > MAX_DEPTH {
        return;
    }

    // Check for rex/projects.json in this directory
    let registry_path = dir.join("rex").join("projects.json");
    if registry_path.is_file() {
        if let Ok(contents) = fs::read_to_string(&registry_path) {
            if let Ok(registry) = serde_json::from_str::<ProjectRegistry>(&contents) {
                let all_projects = registry.active.iter().chain(registry.inactive.iter());
                for proj in all_projects {
                    if seen_ids.insert(proj.id.clone()) {
                        let proj_dir = Path::new(&proj.directory);
                        let state_path = proj_dir.join(".rex-autorun.json");
                        let status_path = proj_dir.join("rex").join("project-status.json");

                        let autorun_state = read_state(&state_path);
                        let running = autorun_state.is_some();

                        results.push(DiscoveredProject {
                            id: proj.id.clone(),
                            title: proj.title.clone(),
                            directory: proj.directory.clone(),
                            running,
                            autorun_state,
                            has_project_status: status_path.exists(),
                        });
                    }
                }
                debug!(
                    registry = %registry_path.display(),
                    "loaded registry"
                );
            }
        }
        // Don't recurse deeper — this directory is a rex root
        return;
    }

    // Recurse into subdirectories
    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(e) => e.filter_map(|e| e.ok()).collect(),
        Err(ref e) if e.kind() == io::ErrorKind::PermissionDenied => return,
        Err(_) => return,
    };

    for entry in entries {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(name) = entry.file_name().to_str().map(String::from) else {
            continue;
        };

        // Skip hidden directories and known non-project dirs
        if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }

        scan_recursive(&path, depth + 1, results, seen_ids);
    }
}
