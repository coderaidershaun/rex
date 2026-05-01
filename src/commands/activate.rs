use std::path::Path;

use crate::{error::RexError, project::ProjectStore};

/// Run `rex activate <project_id>` against `cwd`.
///
/// # Errors
/// - [`RexError::ProjectNotFound`] if `rex/inactive/<project_id>/` does not exist
/// - [`RexError::SlugCollision`] if the current active id collides with an existing inactive entry
/// - [`RexError::Io`] for filesystem failures
pub fn run(cwd: &Path, project_id: &str) -> Result<(), RexError> {
    let store = ProjectStore::new(cwd);
    store.swap_active(project_id)?;
    println!("Activated project '{project_id}'");
    Ok(())
}
