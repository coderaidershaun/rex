use std::{
    borrow::Cow,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RexError;

static EMBEDDED_CLAUDE: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude");

const EMBEDDED_PIPELINE: &str = include_str!("../rex/pipeline.yaml");
const EMBEDDED_CLAUDE_MD_TMPL: &str = include_str!("../templates/CLAUDE.md.tmpl");

const MANIFEST_PATH: &str = ".claude/.rex-manifest.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub rex_version: String,
    pub files: HashMap<String, String>,
}

/// Source of the bundle: either compiled-in or live disk (when REX_BUNDLE_DIR is set).
pub enum Bundle {
    Embedded,
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
        // Strip the ".claude/" prefix to look up in the embedded dir.
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
            // Pass the SAME root_prefix; don't descend with subdir.path() appended.
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

/// Hex-encoded SHA-256 of `data`.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// What the three-way merge logic should do with one file.
#[derive(Debug, PartialEq, Eq)]
pub enum MergeAction {
    /// Write the bundle version and record its hash.
    Write,
    /// Leave disk untouched. Update manifest hash to disk hash (adopting on first init).
    Adopt,
    /// User modified; skip this file.
    PreserveUser,
    /// Both disk and bundle changed relative to manifest. Write .rex-new sibling.
    WriteNew,
    /// No change needed.
    Noop,
}

/// Compute merge action.
///
/// # Parameters
/// - `manifest_hash`: stored hash from prior init run, or `None` on first init
/// - `disk_hash`: current hash of the on-disk file, or `None` if file absent
/// - `bundle_hash`: hash of the bundle's version of the file
///
/// First-init logic (manifest_hash == None):
///   - missing on disk → write
///   - same bytes as bundle → adopt (record bundle hash, don't write)
///   - differs from bundle → preserve user (treat existing file as user-modified)
///
/// Subsequent-run logic (manifest_hash == Some):
///   - disk == manifest, bundle == manifest → noop
///   - disk == manifest, bundle != manifest → upgrade (write bundle)
///   - disk != manifest, bundle == manifest → preserve user
///   - disk != manifest, bundle != manifest → conflict (write .rex-new sibling)
pub fn merge_action(
    manifest_hash: Option<&str>,
    disk_hash: Option<&str>,
    bundle_hash: &str,
) -> MergeAction {
    match (manifest_hash, disk_hash) {
        // First init: file absent on disk
        (None, None) => MergeAction::Write,

        // First init: file present on disk
        (None, Some(dh)) => {
            if dh == bundle_hash {
                MergeAction::Adopt
            } else {
                MergeAction::PreserveUser
            }
        }

        // Subsequent run: file absent on disk (deleted by user). Treat as user-modified.
        (Some(_), None) => MergeAction::PreserveUser,

        // Subsequent run: file present on disk
        (Some(mh), Some(dh)) => {
            let disk_matches_manifest = dh == mh;
            let bundle_matches_manifest = bundle_hash == mh;

            match (disk_matches_manifest, bundle_matches_manifest) {
                (true, true) => MergeAction::Noop,
                (true, false) => MergeAction::Write,
                (false, true) => MergeAction::PreserveUser,
                (false, false) => MergeAction::WriteNew,
            }
        }
    }
}

/// Read existing manifest from CWD, if present.
pub fn read_manifest(cwd: &Path) -> Result<Option<Manifest>, RexError> {
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
pub fn write_manifest(cwd: &Path, manifest: &Manifest) -> Result<(), RexError> {
    let manifest_path = cwd.join(MANIFEST_PATH);
    let parent = manifest_path
        .parent()
        .expect("manifest path has parent dir");

    fs::create_dir_all(parent).map_err(|source| RexError::Io {
        path: parent.to_owned(),
        source,
    })?;

    let json = serde_json::to_string_pretty(manifest).map_err(RexError::JsonSerialize)?;

    // Write to a sibling temp file then rename for atomicity.
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

/// Summary of what init did.
#[derive(Debug, Default)]
pub struct InitSummary {
    pub written: u32,
    pub upgraded: u32,
    pub preserved: u32,
    pub conflicts: u32,
    pub noops: u32,
}

/// Run the three-way merge for all bundle files in `cwd`, writing/skipping per the merge table.
pub fn apply_bundle(bundle: &Bundle, cwd: &Path, force: bool) -> Result<InitSummary, RexError> {
    let existing_manifest = read_manifest(cwd)?;
    let mut new_files: HashMap<String, String> = HashMap::new();
    let mut summary = InitSummary::default();

    let entries = bundle.walk()?;

    for (rel, bundle_contents) in &entries {
        // Skip the manifest itself — we write it last.
        let rel_str = rel.to_string_lossy().to_string();
        if rel_str == MANIFEST_PATH {
            continue;
        }

        let bundle_hash = sha256_hex(bundle_contents);
        let disk_path = cwd.join(rel);

        let disk_hash = if disk_path.exists() {
            let disk_contents = fs::read(&disk_path).map_err(|source| RexError::Io {
                path: disk_path.clone(),
                source,
            })?;
            Some(sha256_hex(&disk_contents))
        } else {
            None
        };

        let manifest_hash = existing_manifest
            .as_ref()
            .and_then(|m| m.files.get(&rel_str))
            .map(String::as_str);

        let action = if force {
            MergeAction::Write
        } else {
            merge_action(manifest_hash, disk_hash.as_deref(), &bundle_hash)
        };

        match action {
            MergeAction::Write => {
                write_bundle_file(&disk_path, bundle_contents)?;
                new_files.insert(rel_str, bundle_hash);
                if existing_manifest.is_some() {
                    summary.upgraded += 1;
                } else {
                    summary.written += 1;
                }
            }
            MergeAction::Adopt => {
                // Record disk hash as the baseline (user had this file, same as bundle).
                new_files.insert(rel_str, bundle_hash);
                summary.noops += 1;
            }
            MergeAction::PreserveUser => {
                // Keep the old manifest entry so we can still detect future upgrades.
                if let Some(h) = manifest_hash {
                    new_files.insert(rel_str, h.to_owned());
                } else if let Some(dh) = disk_hash {
                    // First init, file differs: record disk hash so next run detects user-modified.
                    new_files.insert(rel_str, dh);
                }
                summary.preserved += 1;
            }
            MergeAction::WriteNew => {
                let sibling = disk_path.with_extension(format!(
                    "{}.rex-new",
                    disk_path.extension().and_then(|e| e.to_str()).unwrap_or("")
                ));
                write_bundle_file(&sibling, bundle_contents)?;
                // Keep old manifest entry for the original file.
                if let Some(h) = manifest_hash {
                    new_files.insert(rel_str, h.to_owned());
                }
                summary.conflicts += 1;
            }
            MergeAction::Noop => {
                new_files.insert(rel_str, bundle_hash);
                summary.noops += 1;
            }
        }
    }

    let rex_version = env!("CARGO_PKG_VERSION").to_owned();
    let manifest = Manifest {
        rex_version,
        files: new_files,
    };
    write_manifest(cwd, &manifest)?;

    Ok(summary)
}

fn write_bundle_file(path: &Path, contents: &[u8]) -> Result<(), RexError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| RexError::Io {
            path: parent.to_owned(),
            source,
        })?;
    }
    fs::write(path, contents).map_err(|source| RexError::Io {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_produces_known_digest() {
        // echo -n "" | sha256sum = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let result = sha256_hex(b"");
        assert_eq!(
            result,
            "e3b0c44298fc1c149afbf4c8996fb924\
             27ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn first_init_absent_file_writes() {
        assert_eq!(merge_action(None, None, "abc"), MergeAction::Write);
    }

    #[test]
    fn first_init_matching_file_adopts() {
        assert_eq!(merge_action(None, Some("abc"), "abc"), MergeAction::Adopt);
    }

    #[test]
    fn first_init_differing_file_preserves() {
        assert_eq!(
            merge_action(None, Some("disk_hash"), "bundle_hash"),
            MergeAction::PreserveUser
        );
    }

    #[test]
    fn subsequent_all_match_is_noop() {
        assert_eq!(
            merge_action(Some("abc"), Some("abc"), "abc"),
            MergeAction::Noop
        );
    }

    #[test]
    fn subsequent_bundle_changed_upgrades() {
        assert_eq!(
            merge_action(Some("old"), Some("old"), "new"),
            MergeAction::Write
        );
    }

    #[test]
    fn subsequent_user_changed_preserves() {
        assert_eq!(
            merge_action(Some("old"), Some("user"), "old"),
            MergeAction::PreserveUser
        );
    }

    #[test]
    fn subsequent_both_changed_conflicts() {
        assert_eq!(
            merge_action(Some("old"), Some("user"), "new"),
            MergeAction::WriteNew
        );
    }

    #[test]
    fn subsequent_disk_deleted_preserves() {
        assert_eq!(
            merge_action(Some("old"), None, "old"),
            MergeAction::PreserveUser
        );
    }
}
