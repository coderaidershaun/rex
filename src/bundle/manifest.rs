use std::{collections::HashMap, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::error::RexError;

pub(super) const MANIFEST_PATH: &str = ".claude/.rex-manifest.json";

/// Tracks which bundle files rex installed and at what hash, so a re-run
/// can tell user edits apart from upstream changes.
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    /// `CARGO_PKG_VERSION` of the rex build that wrote the manifest.
    pub rex_version: String,
    /// Map of bundle-relative path to the SHA-256 hex of the file at install time.
    pub files: HashMap<String, String>,
}

impl Manifest {
    /// Read the manifest from `cwd/.claude/.rex-manifest.json`, if present.
    ///
    /// # Errors
    /// - [`RexError::Io`] reading the manifest file
    /// - [`RexError::JsonParse`] if the manifest is malformed
    pub fn load(cwd: &Path) -> Result<Option<Self>, RexError> {
        let path = cwd.join(MANIFEST_PATH);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path).map_err(|source| RexError::Io {
            path: path.clone(),
            source,
        })?;
        let manifest =
            serde_json::from_str(&raw).map_err(|source| RexError::JsonParse { path, source })?;
        Ok(Some(manifest))
    }

    /// Write the manifest atomically: write to a tempfile in the same dir, then rename.
    ///
    /// # Errors
    /// - [`RexError::Io`] for filesystem failures
    /// - [`RexError::JsonSerialize`] if the manifest cannot be serialized
    pub fn save(&self, cwd: &Path) -> Result<(), RexError> {
        let manifest_path = cwd.join(MANIFEST_PATH);
        let parent = manifest_path
            .parent()
            .expect("MANIFEST_PATH constant always has a parent dir");

        fs::create_dir_all(parent).map_err(|source| RexError::Io {
            path: parent.to_owned(),
            source,
        })?;

        let json = serde_json::to_string_pretty(self)?;

        let tmp_path = manifest_path.with_extension("json.tmp");
        fs::write(&tmp_path, &json).map_err(|source| RexError::Io {
            path: tmp_path.clone(),
            source,
        })?;
        fs::rename(&tmp_path, &manifest_path).map_err(|source| RexError::Io {
            path: manifest_path,
            source,
        })?;
        Ok(())
    }
}
