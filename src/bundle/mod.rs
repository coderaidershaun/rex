pub mod manifest;
pub mod merge;

use std::{
    borrow::Cow,
    env, fs,
    path::{Path, PathBuf},
};

use include_dir::{Dir, include_dir};

use crate::error::RexError;

pub use manifest::Manifest;
pub use merge::{BundleMode, InitSummary, apply, sha256_hex};

static EMBEDDED_CLAUDE: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude");

const EMBEDDED_PIPELINE: &str = include_str!("../../rex/pipeline.yaml");
const EMBEDDED_CLAUDE_MD_TMPL: &str = include_str!("../../templates/CLAUDE.md.tmpl");

/// Source of the bundle: either compiled-in or live disk (when REX_BUNDLE_DIR is set).
pub enum Bundle {
    /// Files compiled into the binary via `include_dir!`.
    Embedded,
    /// Files read live from the given directory at runtime.
    LiveDisk(PathBuf),
}

impl Bundle {
    /// Build from environment. When `REX_BUNDLE_DIR` is set, uses live disk.
    pub fn from_env() -> Self {
        match env::var("REX_BUNDLE_DIR") {
            Ok(dir) => Self::LiveDisk(PathBuf::from(dir)),
            Err(_) => Self::Embedded,
        }
    }

    /// Read a file from the bundle. `rel` is relative to the bundle root.
    ///
    /// # Errors
    /// - [`RexError::Io`] for live-disk read failures
    /// - [`RexError::BundleFileNotFound`] if no embedded entry matches `rel`
    pub fn read_file(&self, rel: &Path) -> Result<Cow<'static, [u8]>, RexError> {
        match self {
            Self::Embedded => self.read_embedded(rel),
            Self::LiveDisk(root) => {
                let full = root.join(rel);
                let bytes = fs::read(&full).map_err(|source| RexError::Io {
                    path: full.clone(),
                    source,
                })?;
                Ok(Cow::Owned(bytes))
            }
        }
    }

    fn read_embedded(&self, rel: &Path) -> Result<Cow<'static, [u8]>, RexError> {
        let rel_str = rel.to_string_lossy();
        if rel_str == "rex/pipeline.yaml" {
            return Ok(Cow::Borrowed(EMBEDDED_PIPELINE.as_bytes()));
        }
        if rel_str == "templates/CLAUDE.md.tmpl" {
            return Ok(Cow::Borrowed(EMBEDDED_CLAUDE_MD_TMPL.as_bytes()));
        }
        let in_claude = rel
            .strip_prefix(".claude")
            .ok()
            .and_then(|p| EMBEDDED_CLAUDE.get_file(p));

        in_claude
            .map(|f| Cow::Borrowed(f.contents()))
            .ok_or_else(|| RexError::BundleFileNotFound {
                path: rel.to_owned(),
            })
    }

    /// Walk all bundle entries, returning (relative_path, contents).
    ///
    /// # Errors
    /// [`RexError::Io`] for live-disk read failures while walking.
    pub fn walk(&self) -> Result<Vec<(PathBuf, Vec<u8>)>, RexError> {
        match self {
            Self::Embedded => {
                let mut entries = Vec::new();
                self.walk_embedded_dir(&EMBEDDED_CLAUDE, Path::new(".claude"), &mut entries);
                entries.push((
                    PathBuf::from("rex/pipeline.yaml"),
                    EMBEDDED_PIPELINE.as_bytes().to_vec(),
                ));
                Ok(entries)
            }
            Self::LiveDisk(root) => {
                let mut entries = Vec::new();
                let claude_root = root.join(".claude");
                if claude_root.exists() {
                    self.walk_disk_dir(&claude_root, Path::new(".claude"), &mut entries)?;
                }
                let pipeline = root.join("rex/pipeline.yaml");
                if pipeline.exists() {
                    let contents = fs::read(&pipeline).map_err(|source| RexError::Io {
                        path: pipeline.clone(),
                        source,
                    })?;
                    entries.push((PathBuf::from("rex/pipeline.yaml"), contents));
                }
                Ok(entries)
            }
        }
    }

    fn walk_embedded_dir(
        &self,
        dir: &'static Dir<'static>,
        root_prefix: &Path,
        entries: &mut Vec<(PathBuf, Vec<u8>)>,
    ) {
        // file.path() is always relative to the include_dir! root, so we join with
        // root_prefix only — never the subdir's own path.
        for file in dir.files() {
            let rel = root_prefix.join(file.path());
            entries.push((rel, file.contents().to_vec()));
        }
        for subdir in dir.dirs() {
            self.walk_embedded_dir(subdir, root_prefix, entries);
        }
    }

    fn walk_disk_dir(
        &self,
        dir: &Path,
        prefix: &Path,
        entries: &mut Vec<(PathBuf, Vec<u8>)>,
    ) -> Result<(), RexError> {
        for entry in walkdir::WalkDir::new(dir).min_depth(1).sort_by_file_name() {
            let entry = entry.map_err(|e| RexError::Io {
                path: dir.to_owned(),
                source: e.into(),
            })?;
            if entry.file_type().is_file() {
                let rel_to_dir = entry
                    .path()
                    .strip_prefix(dir)
                    .expect("walkdir entry is under dir");
                let rel = prefix.join(rel_to_dir);
                let contents = fs::read(entry.path()).map_err(|source| RexError::Io {
                    path: entry.path().to_owned(),
                    source,
                })?;
                entries.push((rel, contents));
            }
        }
        Ok(())
    }
}
