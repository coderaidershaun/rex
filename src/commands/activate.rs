use std::path::Path;

use crate::{error::RexError, project::swap_active};

/// Run `rex activate <project_id>` against `cwd`.
///
/// # Errors
/// - [`RexError::ProjectNotFound`] if `rex/inactive/<project_id>/` does not exist
/// - [`RexError::SlugCollision`] if the current active id collides with an existing inactive entry
/// - [`RexError::Io`] for filesystem failures
pub fn run(cwd: &Path, project_id: &str) -> Result<(), RexError> {
    swap_active(cwd, project_id)?;
    println!("Activated project '{}'", project_id);
    Ok(())
}
